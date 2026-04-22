use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};
use streamkeep_download_core::{
    DownloadProgress, DownloadRequest, DownloadStatus, download_segments_to_transport_stream,
};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_streamkeep_capture::{RemuxToMp4Request, StreamkeepCaptureExt};
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
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartDownloadResult {
    pub output_name: String,
    pub output_path: String,
    pub media_playlist_url: String,
    pub output_bytes: u64,
    pub track_count: u32,
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

    let app_for_progress = app.clone();
    let segment_result = download_segments_to_transport_stream(
        &download_request,
        &transport_path,
        move |progress| {
            let _ = app_for_progress.emit("download:progress", progress);
        },
    )
    .await
    .map_err(|error| {
        error!(?error, "failed to download HLS segments");
        error.to_string()
    })?;

    info!(
        transport_path = %segment_result.transport_stream_path,
        downloaded_bytes = segment_result.downloaded_bytes,
        completed_segments = segment_result.completed_segments,
        "downloaded HLS transport stream"
    );
    let _ = app.emit(
        "download:progress",
        DownloadProgress {
            status: DownloadStatus::Remuxing,
            completed_segments: segment_result.completed_segments,
            total_segments: Some(segment_result.completed_segments),
            downloaded_bytes: segment_result.downloaded_bytes,
            total_bytes: None,
            message: Some("remuxing transport stream".to_owned()),
        },
    );

    let remux_result = app
        .streamkeep_capture()
        .remux_to_mp4(RemuxToMp4Request {
            input_path: transport_path.to_string_lossy().to_string(),
            output_path: output_path.to_string_lossy().to_string(),
        })
        .map_err(|error| {
            error!(?error, "failed to remux transport stream to mp4");
            error
        })?;

    let _ = fs::remove_file(&transport_path);
    info!(
        output_path = %remux_result.output_path,
        output_bytes = remux_result.output_bytes,
        track_count = remux_result.track_count,
        "saved Streamkeep MP4"
    );

    let _ = app.emit(
        "download:progress",
        DownloadProgress {
            status: DownloadStatus::Done,
            completed_segments: segment_result.completed_segments,
            total_segments: Some(segment_result.completed_segments),
            downloaded_bytes: remux_result.output_bytes,
            total_bytes: Some(remux_result.output_bytes),
            message: Some("saved mp4".to_owned()),
        },
    );

    Ok(StartDownloadResult {
        output_name,
        output_path: remux_result.output_path,
        media_playlist_url: segment_result.media_playlist_url,
        output_bytes: remux_result.output_bytes,
        track_count: remux_result.track_count,
    })
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
