use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use streamkeep_download_core::{
    DownloadProgress, DownloadRequest, DownloadStatus, download_segments_to_transport_stream,
};
use streamkeep_storage_core::{DownloadHistory, DownloadJobRecord, DownloadJobStatus};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_streamkeep_capture::{
    PublishToDownloadsRequest, RemuxToMp4Request, StreamkeepCaptureExt,
};
use tracing::{debug, error, info};

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
    let download_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("downloads");
    fs::create_dir_all(&download_dir).map_err(|error| error.to_string())?;
    let history_path = history_file_path(&download_dir);

    let transport_path = temp_transport_path(&download_dir, &output_name);
    let output_path = unique_output_path(&download_dir, &output_name);
    debug!(
        download_dir = %download_dir.display(),
        transport_path = %transport_path.display(),
        output_path = %output_path.display(),
        output_name = %output_name,
        "resolved download paths"
    );
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
    let mut job_record = DownloadJobRecord::queued(
        title,
        output_name.clone(),
        request.page_url.unwrap_or_default(),
        download_request.master_url.clone(),
        download_request.media_playlist_url.clone(),
        request
            .quality_label
            .filter(|quality| !quality.trim().is_empty())
            .unwrap_or_else(|| "Best available".to_owned()),
    );
    let job_id = job_record.id.to_string();
    persist_job_record(&history_path, job_record.clone())?;

    let app_for_progress = app.clone();
    let history_path_for_progress = history_path.clone();
    let job_id_for_progress = job_id.clone();
    let progress_record = Arc::new(Mutex::new(job_record.clone()));
    let progress_record_for_callback = progress_record.clone();
    let segment_result = download_segments_to_transport_stream(
        &download_request,
        &transport_path,
        move |progress| {
            if let Ok(mut record) = progress_record_for_callback.lock() {
                record.apply_progress(
                    job_status_from_download(progress.status),
                    progress.percent(),
                );
                if let Err(error) = persist_job_record(&history_path_for_progress, record.clone()) {
                    error!(?error, "failed to persist download progress");
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
                downloaded_bytes: segment_result.downloaded_bytes,
                total_bytes: None,
                message: Some("remuxing transport stream".to_owned()),
            },
        },
    );
    if let Ok(mut record) = progress_record.lock() {
        record.apply_progress(DownloadJobStatus::Remuxing, Some(99));
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
        "publishing Streamkeep MP4 to Android Downloads"
    );
    let _ = app.emit(
        "download:progress",
        DownloadProgressEvent {
            job_id: job_id.clone(),
            progress: DownloadProgress {
                status: DownloadStatus::Remuxing,
                completed_segments: segment_result.completed_segments,
                total_segments: Some(segment_result.completed_segments),
                downloaded_bytes: remux_result.output_bytes,
                total_bytes: None,
                message: Some("publishing mp4 to Downloads".to_owned()),
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
                error!(?error, "failed to publish MP4 to Android Downloads");
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

    info!(
        output_path = %remux_result.output_path,
        output_uri = %publish_result.content_uri,
        output_bytes = publish_result.output_bytes,
        track_count = remux_result.track_count,
        "saved Streamkeep MP4 to public Downloads"
    );

    if let Ok(mut record) = progress_record.lock() {
        record.mark_done(
            remux_result.output_path.clone(),
            publish_result.content_uri.clone(),
            publish_result.output_bytes,
        );
        job_record = record.clone();
        persist_job_record(&history_path, job_record.clone())?;
    }

    let _ = app.emit("download:history-updated", job_record);

    let _ = app.emit(
        "download:progress",
        DownloadProgressEvent {
            job_id: job_id.clone(),
            progress: DownloadProgress {
                status: DownloadStatus::Done,
                completed_segments: segment_result.completed_segments,
                total_segments: Some(segment_result.completed_segments),
                downloaded_bytes: publish_result.output_bytes,
                total_bytes: Some(publish_result.output_bytes),
                message: Some("saved mp4".to_owned()),
            },
        },
    );

    Ok(StartDownloadResult {
        job_id,
        output_name,
        output_path: remux_result.output_path,
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

fn persist_job_record(history_path: &Path, record: DownloadJobRecord) -> Result<(), String> {
    let mut history = read_download_history(history_path)?;
    history.upsert(record);
    write_download_history(history_path, &history)
}

fn read_download_history(history_path: &Path) -> Result<DownloadHistory, String> {
    if !history_path.exists() {
        return Ok(DownloadHistory::default());
    }

    let body = fs::read_to_string(history_path).map_err(|error| error.to_string())?;
    serde_json::from_str(&body).map_err(|error| error.to_string())
}

fn write_download_history(history_path: &Path, history: &DownloadHistory) -> Result<(), String> {
    if let Some(parent) = history_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let body = serde_json::to_string_pretty(history).map_err(|error| error.to_string())?;
    fs::write(history_path, body).map_err(|error| error.to_string())
}

fn temp_transport_path(download_dir: &std::path::Path, output_name: &str) -> PathBuf {
    let stem = output_name.strip_suffix(".mp4").unwrap_or(output_name);
    download_dir.join(format!("{stem}.download.ts"))
}

fn unique_output_path(download_dir: &std::path::Path, output_name: &str) -> PathBuf {
    let candidate = download_dir.join(output_name);
    if !candidate.exists() {
        return candidate;
    }

    let stem = output_name.strip_suffix(".mp4").unwrap_or(output_name);
    for index in 1..1000 {
        let candidate = download_dir.join(format!("{stem} ({index}).mp4"));
        if !candidate.exists() {
            return candidate;
        }
    }

    download_dir.join(format!("{stem} (999).mp4"))
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
