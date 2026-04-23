use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadSettings {
    pub output_directory: String,
    pub default_output_directory: String,
    pub custom_output_directory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PersistedSettings {
    output_directory: Option<String>,
}

#[tauri::command]
pub fn get_download_settings_command<R: Runtime>(
    app: AppHandle<R>,
) -> Result<DownloadSettings, String> {
    read_download_settings(&app)
}

#[tauri::command]
pub fn set_download_directory_command<R: Runtime>(
    app: AppHandle<R>,
    output_directory: String,
) -> Result<DownloadSettings, String> {
    let output_directory = PathBuf::from(output_directory.trim());
    if output_directory.as_os_str().is_empty() {
        return Err("Choose a Streamkeep output folder".to_owned());
    }

    fs::create_dir_all(&output_directory).map_err(|error| error.to_string())?;
    let persisted = PersistedSettings {
        output_directory: Some(output_directory.to_string_lossy().to_string()),
    };
    write_persisted_settings(&app, &persisted)?;
    info!(
        output_directory = %output_directory.display(),
        "updated Streamkeep download folder"
    );
    read_download_settings(&app)
}

#[tauri::command]
pub fn reset_download_directory_command<R: Runtime>(
    app: AppHandle<R>,
) -> Result<DownloadSettings, String> {
    write_persisted_settings(&app, &PersistedSettings::default())?;
    info!("reset Streamkeep download folder");
    read_download_settings(&app)
}

#[cfg(not(target_os = "android"))]
pub fn effective_download_directory<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let settings = read_download_settings(app)?;
    let path = PathBuf::from(settings.output_directory);
    fs::create_dir_all(&path).map_err(|error| error.to_string())?;
    Ok(path)
}

fn read_download_settings<R: Runtime>(app: &AppHandle<R>) -> Result<DownloadSettings, String> {
    let default_output_directory = default_download_directory(app)?;
    let persisted = read_persisted_settings(app)?;
    let output_directory = persisted
        .output_directory
        .filter(|path| !path.trim().is_empty())
        .unwrap_or_else(|| default_output_directory.to_string_lossy().to_string());

    Ok(DownloadSettings {
        custom_output_directory: output_directory != default_output_directory.to_string_lossy(),
        output_directory,
        default_output_directory: default_output_directory.to_string_lossy().to_string(),
    })
}

fn read_persisted_settings<R: Runtime>(app: &AppHandle<R>) -> Result<PersistedSettings, String> {
    let path = settings_path(app)?;
    if !path.exists() {
        return Ok(PersistedSettings::default());
    }

    let body = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    serde_json::from_str(&body).map_err(|error| error.to_string())
}

fn write_persisted_settings<R: Runtime>(
    app: &AppHandle<R>,
    settings: &PersistedSettings,
) -> Result<(), String> {
    let path = settings_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let body = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;
    fs::write(&path, body).map_err(|error| error.to_string())
}

fn settings_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?
        .join("settings.json"))
}

fn default_download_directory<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    #[cfg(target_os = "android")]
    let base = platform_download_base(app);

    #[cfg(not(target_os = "android"))]
    let base = platform_download_base(app)?;

    Ok(base.join("Streamkeep"))
}

#[cfg(target_os = "android")]
fn platform_download_base<R: Runtime>(_app: &AppHandle<R>) -> PathBuf {
    PathBuf::from("Movies")
}

#[cfg(not(target_os = "android"))]
fn platform_download_base<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(path) = app.path().video_dir() {
            return Ok(path);
        }
    }

    if let Ok(path) = app.path().download_dir() {
        return Ok(path);
    }

    let fallback = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    tracing::debug!(
        fallback = %fallback.display(),
        "using app data directory as Streamkeep output fallback"
    );
    Ok(fallback)
}

pub fn unique_file_path(directory: &Path, file_name: &str) -> PathBuf {
    let candidate = directory.join(file_name);
    if !candidate.exists() {
        return candidate;
    }

    let stem = file_name.strip_suffix(".mp4").unwrap_or(file_name);
    for index in 1..1000 {
        let candidate = directory.join(format!("{stem} ({index}).mp4"));
        if !candidate.exists() {
            return candidate;
        }
    }

    directory.join(format!("{stem} (999).mp4"))
}
