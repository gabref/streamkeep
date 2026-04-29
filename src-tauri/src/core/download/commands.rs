use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use streamkeep_download_core::{
    DownloadProgress, DownloadRequest, DownloadStatus, download_segments_to_transport_stream,
};
use streamkeep_storage_core::{
    DownloadHistory, DownloadJobRecord, DownloadJobStatus, QueuedDownloadJob,
};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_streamkeep_capture::{
    DeletePublishedDownloadRequest, OpenUriRequest, PublishToDownloadsRequest, RemuxToMp4Request,
    StreamkeepCaptureExt,
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

#[cfg(not(target_os = "android"))]
use crate::core::settings::commands::effective_download_directory;
use crate::core::settings::commands::unique_file_path;

const HISTORY_PROGRESS_PERSIST_INTERVAL: Duration = Duration::from_secs(2);

static HISTORY_IO_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartDownloadRequest {
    pub master_url: String,
    pub media_playlist_url: Option<String>,
    pub referer: Option<String>,
    pub user_agent: Option<String>,
    pub cookies: Option<String>,
    pub output_name: String,
    pub title: Option<String>,
    pub page_url: Option<String>,
    pub quality_label: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartDownloadResult {
    pub job_id: String,
    pub output_name: String,
    pub output_path: String,
    pub output_uri: String,
    pub media_playlist_url: String,
    pub output_bytes: u64,
    pub track_count: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgressEvent {
    pub job_id: String,
    #[serde(flatten)]
    pub progress: DownloadProgress,
}

#[tauri::command]
pub async fn start_download_command<R: Runtime>(
    app: AppHandle<R>,
    request: StartDownloadRequest,
) -> Result<StartDownloadResult, String> {
    info!(
        master_url = %request.master_url,
        media_playlist_url = request.media_playlist_url.as_deref().unwrap_or(""),
        requested_output_name = %request.output_name,
        "starting Streamkeep download command"
    );
    let output_name = ensure_mp4_extension(&sanitize_file_name(&request.output_name));
    let workspace_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("downloads");
    fs::create_dir_all(&workspace_dir).map_err(|error| error.to_string())?;
    let history_path = history_file_path(&workspace_dir);
    let final_output_dir = final_output_directory(&app)?;

    let transport_path = temp_transport_path(&workspace_dir, &output_name);
    let output_path = remux_output_path(&app, &workspace_dir, &final_output_dir, &output_name);
    debug!(
        workspace_dir = %workspace_dir.display(),
        final_output_dir = %final_output_dir.display(),
        transport_path = %transport_path.display(),
        output_path = %output_path.display(),
        output_name = %output_name,
        "resolved download paths"
    );
    let referer = request.referer.clone();
    let user_agent = request.user_agent.clone();
    let cookies = request.cookies.clone();
    let download_request = DownloadRequest {
        master_url: request.master_url,
        media_playlist_url: request.media_playlist_url,
        referer: request.referer,
        user_agent: request.user_agent,
        cookies: request.cookies,
        output_name: output_name.clone(),
    };
    let title = request
        .title
        .as_deref()
        .map(sanitize_file_name)
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| {
            output_name
                .strip_suffix(".mp4")
                .unwrap_or(&output_name)
                .to_owned()
        });
    let mut job_record = DownloadJobRecord::queued(QueuedDownloadJob {
        title,
        output_name: output_name.clone(),
        page_url: request.page_url.unwrap_or_default(),
        master_url: download_request.master_url.clone(),
        media_playlist_url: download_request.media_playlist_url.clone(),
        referer,
        user_agent,
        cookies,
        quality: request
            .quality_label
            .filter(|quality| !quality.trim().is_empty())
            .unwrap_or_else(|| "Best available".to_owned()),
    });
    let job_id = job_record.id.to_string();
    persist_job_record(&history_path, job_record.clone())?;
    let _ = app.emit("download:history-updated", job_record.clone());
    let _download_keep_alive = DownloadKeepAlive::start(app.clone());

    let app_for_progress = app.clone();
    let history_path_for_progress = history_path.clone();
    let job_id_for_progress = job_id.clone();
    let progress_record = Arc::new(Mutex::new(job_record.clone()));
    let progress_record_for_callback = progress_record.clone();
    let mut last_persisted_status: Option<DownloadJobStatus> = None;
    let mut last_persisted_percent: Option<u8> = None;
    let mut last_persisted_at = Instant::now() - HISTORY_PROGRESS_PERSIST_INTERVAL;
    let segment_result = download_segments_to_transport_stream(
        &download_request,
        &transport_path,
        move |progress| {
            let job_status = job_status_from_download(progress.status);
            let progress_percent = progress.percent();
            if let Ok(mut record) = progress_record_for_callback.lock() {
                record.apply_progress(job_status, progress_percent);
                let should_persist = last_persisted_status != Some(job_status)
                    || last_persisted_percent != progress_percent
                    || last_persisted_at.elapsed() >= HISTORY_PROGRESS_PERSIST_INTERVAL;
                if should_persist {
                    if let Err(error) =
                        persist_job_record(&history_path_for_progress, record.clone())
                    {
                        error!(?error, "failed to persist download progress");
                    }
                    last_persisted_status = Some(job_status);
                    last_persisted_percent = progress_percent;
                    last_persisted_at = Instant::now();
                }
            }
            emit_download_progress(&app_for_progress, &job_id_for_progress, progress);
        },
    )
    .await
    .map_err(|error| {
        let message = error.to_string();
        error!(?error, "failed to download HLS segments");
        mark_job_failed(
            &app,
            &history_path,
            &progress_record,
            &job_id,
            &transport_path,
            message.clone(),
        );
        message
    })?;

    info!(
        transport_path = %segment_result.transport_stream_path,
        downloaded_bytes = segment_result.downloaded_bytes,
        completed_segments = segment_result.completed_segments,
        "downloaded HLS transport stream"
    );
    let _ = app.emit(
        "download:progress",
        DownloadProgressEvent {
            job_id: job_id.clone(),
            progress: DownloadProgress {
                status: DownloadStatus::Remuxing,
                completed_segments: segment_result.completed_segments,
                total_segments: Some(segment_result.completed_segments),
                current_segment_index: None,
                current_segment_downloaded_bytes: None,
                current_segment_total_bytes: None,
                downloaded_bytes: segment_result.downloaded_bytes,
                total_bytes: None,
                message: Some("remuxing transport stream".to_owned()),
            },
        },
    );
    if let Ok(mut record) = progress_record.lock() {
        record.apply_progress(DownloadJobStatus::Remuxing, Some(98));
        let _ = persist_job_record(&history_path, record.clone());
    }

    let remux_result = match app.streamkeep_capture().remux_to_mp4(RemuxToMp4Request {
        input_path: transport_path.to_string_lossy().to_string(),
        output_path: output_path.to_string_lossy().to_string(),
    }) {
        Ok(result) => result,
        Err(error) => {
            error!(?error, "failed to remux transport stream to mp4");
            mark_job_failed(
                &app,
                &history_path,
                &progress_record,
                &job_id,
                &transport_path,
                error.clone(),
            );
            return Err(error);
        }
    };

    let _ = fs::remove_file(&transport_path);
    info!(
        input_path = %remux_result.output_path,
        output_name = %output_name,
        "publishing Streamkeep MP4"
    );
    let _ = app.emit(
        "download:progress",
        DownloadProgressEvent {
            job_id: job_id.clone(),
            progress: DownloadProgress {
                status: DownloadStatus::Remuxing,
                completed_segments: segment_result.completed_segments,
                total_segments: Some(segment_result.completed_segments),
                current_segment_index: None,
                current_segment_downloaded_bytes: None,
                current_segment_total_bytes: None,
                downloaded_bytes: remux_result.output_bytes,
                total_bytes: None,
                message: Some("saving mp4".to_owned()),
            },
        },
    );

    let publish_result =
        match app
            .streamkeep_capture()
            .publish_to_downloads(PublishToDownloadsRequest {
                input_path: remux_result.output_path.clone(),
                display_name: output_name.clone(),
            }) {
            Ok(result) => result,
            Err(error) => {
                error!(?error, "failed to publish MP4");
                mark_job_failed(
                    &app,
                    &history_path,
                    &progress_record,
                    &job_id,
                    &PathBuf::from(&remux_result.output_path),
                    error.clone(),
                );
                return Err(error);
            }
        };

    info!(
        output_path = %remux_result.output_path,
        output_uri = %publish_result.content_uri,
        output_bytes = publish_result.output_bytes,
        track_count = remux_result.track_count,
        "saved Streamkeep MP4 to public media library"
    );

    if let Ok(mut record) = progress_record.lock() {
        let public_output_path = public_output_path(&publish_result);
        record.mark_done(
            public_output_path.clone(),
            publish_result.content_uri.clone(),
            publish_result.output_bytes,
        );
        job_record = record.clone();
        persist_job_record(&history_path, job_record.clone())?;
    }

    let _ = app.emit("download:history-updated", job_record);
    remove_private_remux_output(&app, &remux_result.output_path);

    let _ = app.emit(
        "download:progress",
        DownloadProgressEvent {
            job_id: job_id.clone(),
            progress: DownloadProgress {
                status: DownloadStatus::Done,
                completed_segments: segment_result.completed_segments,
                total_segments: Some(segment_result.completed_segments),
                current_segment_index: None,
                current_segment_downloaded_bytes: None,
                current_segment_total_bytes: None,
                downloaded_bytes: publish_result.output_bytes,
                total_bytes: Some(publish_result.output_bytes),
                message: Some("saved mp4".to_owned()),
            },
        },
    );

    Ok(StartDownloadResult {
        job_id,
        output_name,
        output_path: public_output_path(&publish_result),
        output_uri: publish_result.content_uri,
        media_playlist_url: segment_result.media_playlist_url,
        output_bytes: publish_result.output_bytes,
        track_count: remux_result.track_count,
    })
}

#[tauri::command]
pub fn list_download_history_command<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Vec<DownloadJobRecord>, String> {
    let download_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("downloads");
    let history = read_download_history(&history_file_path(&download_dir))?;
    Ok(history.jobs)
}

#[tauri::command]
pub fn delete_download_history_command<R: Runtime>(
    app: AppHandle<R>,
    job_id: String,
) -> Result<Vec<DownloadJobRecord>, String> {
    info!(job_id = %job_id, "deleting Streamkeep download history entry");
    let id = Uuid::parse_str(&job_id).map_err(|error| error.to_string())?;
    let download_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("downloads");
    let history_path = history_file_path(&download_dir);
    let mut history = read_download_history(&history_path)?;
    let removed_job = history.jobs.iter().find(|job| job.id == id).cloned();
    if let Some(job) = &removed_job {
        delete_job_files(&app, job);
    }
    history.remove(id);
    write_download_history(&history_path, &history)?;
    Ok(history.jobs)
}

#[tauri::command]
pub fn open_download_command<R: Runtime>(
    app: AppHandle<R>,
    content_uri: String,
) -> Result<(), String> {
    info!(content_uri = %content_uri, "opening published Streamkeep download");
    app.streamkeep_capture().open_uri(OpenUriRequest {
        content_uri,
        mime_type: Some("video/*".to_owned()),
    })
}

fn emit_download_progress<R: Runtime>(
    app: &AppHandle<R>,
    job_id: &str,
    progress: DownloadProgress,
) {
    let _ = app.emit(
        "download:progress",
        DownloadProgressEvent {
            job_id: job_id.to_owned(),
            progress,
        },
    );
}

fn mark_job_failed<R: Runtime>(
    app: &AppHandle<R>,
    history_path: &Path,
    progress_record: &Arc<Mutex<DownloadJobRecord>>,
    job_id: &str,
    transport_path: &Path,
    message: String,
) {
    let _ = fs::remove_file(transport_path);
    if let Ok(mut record) = progress_record.lock() {
        record.mark_failed(message.clone());
        if let Err(error) = persist_job_record(history_path, record.clone()) {
            error!(?error, "failed to persist failed download");
        }
        let _ = app.emit("download:history-updated", record.clone());
        emit_download_progress(
            app,
            job_id,
            DownloadProgress {
                status: DownloadStatus::Failed,
                completed_segments: 0,
                total_segments: None,
                current_segment_index: None,
                current_segment_downloaded_bytes: None,
                current_segment_total_bytes: None,
                downloaded_bytes: 0,
                total_bytes: None,
                message: Some(message),
            },
        );
    }
}

fn job_status_from_download(status: DownloadStatus) -> DownloadJobStatus {
    match status {
        DownloadStatus::Queued => DownloadJobStatus::Queued,
        DownloadStatus::Preparing => DownloadJobStatus::Preparing,
        DownloadStatus::Downloading => DownloadJobStatus::Downloading,
        DownloadStatus::Remuxing => DownloadJobStatus::Remuxing,
        DownloadStatus::Done => DownloadJobStatus::Done,
        DownloadStatus::Failed => DownloadJobStatus::Failed,
        DownloadStatus::Cancelled => DownloadJobStatus::Cancelled,
    }
}

fn history_file_path(download_dir: &Path) -> PathBuf {
    download_dir.join("history.json")
}

fn delete_job_files<R: Runtime>(app: &AppHandle<R>, job: &DownloadJobRecord) {
    if let Some(content_uri) = &job.output_uri
        && let Err(error) =
            app.streamkeep_capture()
                .delete_published_download(DeletePublishedDownloadRequest {
                    content_uri: content_uri.clone(),
                })
    {
        warn!(
            ?error,
            content_uri, "failed to delete published Streamkeep download"
        );
    }

    if let Some(output_path) = &job.output_path {
        delete_private_file_if_app_owned(output_path);
    }
}

fn delete_private_file_if_app_owned(path: &str) {
    let candidate = PathBuf::from(path);
    if !candidate.is_absolute() || !candidate.exists() {
        return;
    }

    if let Err(error) = fs::remove_file(&candidate) {
        debug!(
            ?error,
            path = %candidate.display(),
            "failed to remove private Streamkeep file"
        );
    }
}

fn persist_job_record(history_path: &Path, record: DownloadJobRecord) -> Result<(), String> {
    with_history_lock(|| {
        let mut history = read_download_history_unlocked(history_path)?;
        history.upsert(record);
        write_download_history_unlocked(history_path, &history)
    })
}

fn read_download_history(history_path: &Path) -> Result<DownloadHistory, String> {
    with_history_lock(|| read_download_history_unlocked(history_path))
}

fn write_download_history(history_path: &Path, history: &DownloadHistory) -> Result<(), String> {
    with_history_lock(|| write_download_history_unlocked(history_path, history))
}

fn with_history_lock<T>(operation: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    let _guard = HISTORY_IO_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "Streamkeep download history lock is poisoned".to_owned())?;
    operation()
}

fn read_download_history_unlocked(history_path: &Path) -> Result<DownloadHistory, String> {
    if !history_path.exists() {
        return Ok(DownloadHistory::default());
    }

    let body = fs::read_to_string(history_path).map_err(|error| error.to_string())?;
    if body.trim().is_empty() {
        return Ok(DownloadHistory::default());
    }

    match serde_json::from_str(&body) {
        Ok(history) => Ok(history),
        Err(error) => {
            if let Some(history) = recover_first_download_history(&body) {
                warn!(
                    ?error,
                    path = %history_path.display(),
                    "recovered Streamkeep download history with trailing data"
                );
                if let Err(write_error) = write_download_history_unlocked(history_path, &history) {
                    warn!(
                        ?write_error,
                        path = %history_path.display(),
                        "failed to rewrite recovered Streamkeep download history"
                    );
                }
                return Ok(history);
            }

            quarantine_invalid_history(history_path, &body, &error)?;
            Ok(DownloadHistory::default())
        }
    }
}

fn write_download_history_unlocked(
    history_path: &Path,
    history: &DownloadHistory,
) -> Result<(), String> {
    if let Some(parent) = history_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let body = serde_json::to_string_pretty(history).map_err(|error| error.to_string())?;
    let temp_path = history_path.with_extension("json.tmp");
    fs::write(&temp_path, body).map_err(|error| error.to_string())?;

    #[cfg(target_os = "windows")]
    if history_path.exists() {
        fs::remove_file(history_path).map_err(|error| error.to_string())?;
    }

    fs::rename(&temp_path, history_path).map_err(|error| error.to_string())
}

fn recover_first_download_history(body: &str) -> Option<DownloadHistory> {
    serde_json::Deserializer::from_str(body)
        .into_iter::<DownloadHistory>()
        .next()
        .and_then(Result::ok)
}

fn quarantine_invalid_history(
    history_path: &Path,
    body: &str,
    error: &serde_json::Error,
) -> Result<(), String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let backup_path = history_path.with_extension(format!("json.invalid-{timestamp}"));

    warn!(
        ?error,
        path = %history_path.display(),
        backup_path = %backup_path.display(),
        "quarantining unreadable Streamkeep download history"
    );
    fs::write(&backup_path, body).map_err(|write_error| write_error.to_string())?;
    fs::remove_file(history_path).map_err(|remove_error| remove_error.to_string())
}

fn final_output_directory<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    #[cfg(target_os = "android")]
    {
        let _ = app;
        return Ok(PathBuf::new());
    }

    #[cfg(not(target_os = "android"))]
    {
        effective_download_directory(app)
    }
}

fn remux_output_path<R: Runtime>(
    app: &AppHandle<R>,
    _workspace_dir: &Path,
    final_output_dir: &Path,
    output_name: &str,
) -> PathBuf {
    #[cfg(target_os = "android")]
    {
        let _ = app;
        let _ = final_output_dir;
        unique_file_path(_workspace_dir, output_name)
    }

    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        unique_file_path(final_output_dir, output_name)
    }
}

fn public_output_path(
    publish_result: &tauri_plugin_streamkeep_capture::PublishToDownloadsResult,
) -> String {
    if publish_result.relative_path.is_empty() {
        publish_result.display_name.clone()
    } else {
        format!(
            "{}/{}",
            publish_result.relative_path, publish_result.display_name
        )
    }
}

fn remove_private_remux_output<R: Runtime>(app: &AppHandle<R>, output_path: &str) {
    #[cfg(target_os = "android")]
    {
        let _ = app;
        if let Err(error) = fs::remove_file(output_path) {
            debug!(
                ?error,
                path = %output_path,
                "failed to remove private Streamkeep MP4 after publishing"
            );
        }
    }

    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        let _ = output_path;
    }
}

fn temp_transport_path(download_dir: &std::path::Path, output_name: &str) -> PathBuf {
    let stem = output_name.strip_suffix(".mp4").unwrap_or(output_name);
    download_dir.join(format!("{stem}.download.ts"))
}

fn sanitize_file_name(value: &str) -> String {
    let cleaned = value
        .chars()
        .map(|character| {
            if character.is_control() || r#"<>:"/\|?*"#.contains(character) {
                ' '
            } else {
                character
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(['.', ' '])
        .to_owned();

    if cleaned.is_empty() {
        "Streamkeep capture".to_owned()
    } else {
        cleaned
    }
}

fn ensure_mp4_extension(value: &str) -> String {
    if value.to_lowercase().ends_with(".mp4") {
        value.to_owned()
    } else {
        format!("{value}.mp4")
    }
}

struct DownloadKeepAlive<R: Runtime> {
    app: AppHandle<R>,
    active: bool,
}

impl<R: Runtime> DownloadKeepAlive<R> {
    fn start(app: AppHandle<R>) -> Self {
        let active = match app.streamkeep_capture().start_download_keep_alive() {
            Ok(()) => {
                debug!("started Streamkeep Android download keep-alive service");
                true
            }
            Err(error) => {
                warn!(
                    ?error,
                    "failed to start Streamkeep Android download keep-alive service"
                );
                false
            }
        };

        Self { app, active }
    }
}

impl<R: Runtime> Drop for DownloadKeepAlive<R> {
    fn drop(&mut self) {
        if !self.active {
            return;
        }

        if let Err(error) = self.app.streamkeep_capture().stop_download_keep_alive() {
            warn!(
                ?error,
                "failed to stop Streamkeep Android download keep-alive service"
            );
        } else {
            debug!("stopped Streamkeep Android download keep-alive service");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DownloadHistory, read_download_history};
    use std::{fs, path::PathBuf};
    use uuid::Uuid;

    #[test]
    fn recovers_history_with_trailing_characters_and_rewrites_file() {
        let (directory, history_path) = temp_history_path();
        fs::create_dir_all(&directory).expect("create temp history directory");
        fs::write(&history_path, r#"{"jobs":[]}}"#).expect("write malformed history");

        let history = read_download_history(&history_path).expect("read recovered history");

        assert!(history.jobs.is_empty());
        let rewritten = fs::read_to_string(&history_path).expect("read rewritten history");
        serde_json::from_str::<DownloadHistory>(&rewritten)
            .expect("rewritten history is valid JSON");

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn quarantines_unrecoverable_history_and_allows_empty_history() {
        let (directory, history_path) = temp_history_path();
        fs::create_dir_all(&directory).expect("create temp history directory");
        fs::write(&history_path, "not json").expect("write invalid history");

        let history = read_download_history(&history_path).expect("read default history");

        assert!(history.jobs.is_empty());
        assert!(!history_path.exists());
        let backup_count = fs::read_dir(&directory)
            .expect("read temp history directory")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".invalid-"))
            .count();
        assert_eq!(backup_count, 1);

        let _ = fs::remove_dir_all(directory);
    }

    fn temp_history_path() -> (PathBuf, PathBuf) {
        let directory =
            std::env::temp_dir().join(format!("streamkeep-history-test-{}", Uuid::new_v4()));
        let history_path = directory.join("history.json");
        (directory, history_path)
    }
}
