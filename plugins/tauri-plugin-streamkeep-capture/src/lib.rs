#[cfg(mobile)]
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
#[cfg(not(mobile))]
use std::{fs, path::PathBuf};
use tauri::{
    Manager, Runtime,
    plugin::{Builder, TauriPlugin},
};
#[cfg(not(mobile))]
use tauri::{Url, WebviewUrl, WebviewWindowBuilder};

#[cfg(mobile)]
mod mobile;

#[cfg(target_os = "android")]
const PLUGIN_IDENTIFIER: &str = "app.streamkeep.capture";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenPlayerRequest {
    pub url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerLoadUrlRequest {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub supported: bool,
    pub visible: bool,
    pub loading: bool,
    pub url: Option<String>,
    pub title: Option<String>,
    pub can_go_back: bool,
    pub can_go_forward: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemuxToMp4Request {
    pub input_path: String,
    pub output_path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemuxToMp4Result {
    pub output_path: String,
    pub track_count: u32,
    pub output_bytes: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishToDownloadsRequest {
    pub input_path: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishToDownloadsResult {
    pub content_uri: String,
    pub display_name: String,
    pub relative_path: String,
    pub output_bytes: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePublishedDownloadRequest {
    pub content_uri: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenUriRequest {
    pub content_uri: String,
    pub mime_type: Option<String>,
}

pub struct StreamkeepCapture<R: Runtime> {
    #[cfg(mobile)]
    mobile_plugin_handle: tauri::plugin::PluginHandle<R>,
    #[cfg(not(mobile))]
    app: tauri::AppHandle<R>,
}

impl<R: Runtime> StreamkeepCapture<R> {
    pub fn open_player(&self, request: OpenPlayerRequest) -> Result<PlayerState, String> {
        #[cfg(mobile)]
        {
            return self.run_mobile("openPlayer", request);
        }

        #[cfg(not(mobile))]
        {
            self.open_desktop_player(request)
        }
    }

    pub fn get_player_state(&self) -> Result<PlayerState, String> {
        #[cfg(mobile)]
        {
            return self.run_mobile("getPlayerState", serde_json::json!({}));
        }

        #[cfg(not(mobile))]
        {
            Ok(self.desktop_player_state())
        }
    }

    pub fn go_back(&self) -> Result<PlayerState, String> {
        self.run_empty_command("goBack")
    }

    pub fn go_forward(&self) -> Result<PlayerState, String> {
        self.run_empty_command("goForward")
    }

    pub fn reload(&self) -> Result<PlayerState, String> {
        self.run_empty_command("reload")
    }

    pub fn load_url(&self, request: PlayerLoadUrlRequest) -> Result<PlayerState, String> {
        #[cfg(mobile)]
        {
            return self.run_mobile("loadUrl", request);
        }

        #[cfg(not(mobile))]
        {
            self.navigate_desktop_player(&request.url)
        }
    }

    pub fn remux_to_mp4(&self, request: RemuxToMp4Request) -> Result<RemuxToMp4Result, String> {
        #[cfg(mobile)]
        {
            return self.run_mobile("remuxToMp4", request);
        }

        #[cfg(not(mobile))]
        {
            remux_to_mp4_on_desktop(&request)
        }
    }

    pub fn publish_to_downloads(
        &self,
        request: PublishToDownloadsRequest,
    ) -> Result<PublishToDownloadsResult, String> {
        #[cfg(mobile)]
        {
            return self.run_mobile("publishToDownloads", request);
        }

        #[cfg(not(mobile))]
        {
            publish_to_desktop_file(&request)
        }
    }

    pub fn delete_published_download(
        &self,
        request: DeletePublishedDownloadRequest,
    ) -> Result<(), String> {
        #[cfg(mobile)]
        {
            return self.run_mobile("deletePublishedDownload", request);
        }

        #[cfg(not(mobile))]
        {
            delete_desktop_file(&request.content_uri)
        }
    }

    pub fn open_uri(&self, request: OpenUriRequest) -> Result<(), String> {
        #[cfg(mobile)]
        {
            return self.run_mobile("openUri", request);
        }

        #[cfg(not(mobile))]
        {
            open_uri_on_desktop(&request.content_uri)
        }
    }

    pub fn start_download_keep_alive(&self) -> Result<(), String> {
        #[cfg(mobile)]
        {
            return self.run_mobile("startDownloadKeepAlive", serde_json::json!({}));
        }

        #[cfg(not(mobile))]
        {
            Ok(())
        }
    }

    pub fn stop_download_keep_alive(&self) -> Result<(), String> {
        #[cfg(mobile)]
        {
            return self.run_mobile("stopDownloadKeepAlive", serde_json::json!({}));
        }

        #[cfg(not(mobile))]
        {
            Ok(())
        }
    }

    fn run_empty_command(&self, command: &str) -> Result<PlayerState, String> {
        #[cfg(mobile)]
        {
            return self.run_mobile(command, serde_json::json!({}));
        }

        #[cfg(not(mobile))]
        {
            self.run_desktop_player_command(command)
        }
    }

    #[cfg(mobile)]
    fn run_mobile<T, O>(&self, command: &str, payload: T) -> Result<O, String>
    where
        T: Serialize,
        O: DeserializeOwned,
    {
        self.mobile_plugin_handle
            .run_mobile_plugin(command, payload)
            .map_err(|error| error.to_string())
    }
}

#[cfg(not(mobile))]
const DESKTOP_PLAYER_LABEL: &str = "streamkeep-player";

#[cfg(not(mobile))]
impl<R: Runtime> StreamkeepCapture<R> {
    fn open_desktop_player(&self, request: OpenPlayerRequest) -> Result<PlayerState, String> {
        let url = normalize_desktop_url(request.url.as_deref())?;
        if let Some(window) = self.app.get_webview_window(DESKTOP_PLAYER_LABEL) {
            window.navigate(url).map_err(|error| error.to_string())?;
            let _ = window.show();
            let _ = window.set_focus();
            return Ok(self.desktop_player_state());
        }

        WebviewWindowBuilder::new(&self.app, DESKTOP_PLAYER_LABEL, WebviewUrl::External(url))
            .title("Streamkeep Player")
            .inner_size(1120.0, 760.0)
            .resizable(true)
            .build()
            .map_err(|error| error.to_string())?;
        Ok(self.desktop_player_state())
    }

    fn navigate_desktop_player(&self, value: &str) -> Result<PlayerState, String> {
        let url = normalize_desktop_url(Some(value))?;
        if let Some(window) = self.app.get_webview_window(DESKTOP_PLAYER_LABEL) {
            window.navigate(url).map_err(|error| error.to_string())?;
            let _ = window.show();
            let _ = window.set_focus();
            return Ok(self.desktop_player_state());
        }

        self.open_desktop_player(OpenPlayerRequest {
            url: Some(value.to_owned()),
        })
    }

    fn run_desktop_player_command(&self, command: &str) -> Result<PlayerState, String> {
        if let Some(window) = self.app.get_webview_window(DESKTOP_PLAYER_LABEL) {
            match command {
                "reload" => window.reload().map_err(|error| error.to_string())?,
                "goBack" | "goForward" => {}
                _ => {}
            }
        }
        Ok(self.desktop_player_state())
    }

    fn desktop_player_state(&self) -> PlayerState {
        if let Some(window) = self.app.get_webview_window(DESKTOP_PLAYER_LABEL) {
            return PlayerState {
                supported: true,
                visible: window.is_visible().unwrap_or(true),
                loading: false,
                url: window.url().ok().map(|url| url.to_string()),
                title: window.title().ok(),
                can_go_back: false,
                can_go_forward: false,
            };
        }

        PlayerState {
            supported: true,
            visible: false,
            loading: false,
            url: None,
            title: None,
            can_go_back: false,
            can_go_forward: false,
        }
    }
}

#[cfg(not(mobile))]
fn normalize_desktop_url(value: Option<&str>) -> Result<Url, String> {
    let trimmed = value.unwrap_or("https://example.com").trim();
    let with_scheme = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_owned()
    } else {
        format!("https://{trimmed}")
    };
    Url::parse(&with_scheme).map_err(|error| format!("Invalid Streamkeep player URL: {error}"))
}

#[cfg(not(mobile))]
fn remux_to_mp4_on_desktop(request: &RemuxToMp4Request) -> Result<RemuxToMp4Result, String> {
    ts_to_mp4::remux_file(&request.input_path, &request.output_path)
        .map_err(|error| format!("Failed to remux MPEG-TS to MP4: {error}"))?;

    let output_bytes = fs::metadata(&request.output_path)
        .map_err(|error| format!("Failed to read Streamkeep MP4 metadata: {error}"))?
        .len();

    Ok(RemuxToMp4Result {
        output_path: request.output_path.clone(),
        track_count: 0,
        output_bytes,
    })
}

#[cfg(not(mobile))]
fn publish_to_desktop_file(
    request: &PublishToDownloadsRequest,
) -> Result<PublishToDownloadsResult, String> {
    let input_path = PathBuf::from(&request.input_path);
    let display_name = input_path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            PathBuf::from(&request.display_name)
                .file_name()
                .and_then(|name| name.to_str())
                .filter(|name| !name.trim().is_empty())
                .unwrap_or("Streamkeep capture.mp4")
                .to_owned()
        });
    let relative_path = input_path
        .parent()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default();
    let output_bytes = fs::metadata(&input_path)
        .map_err(|error| format!("Failed to read Streamkeep MP4 metadata: {error}"))?
        .len();

    Ok(PublishToDownloadsResult {
        content_uri: format!("file://{}", input_path.to_string_lossy()),
        display_name,
        relative_path,
        output_bytes,
    })
}

#[cfg(not(mobile))]
fn delete_desktop_file(value: &str) -> Result<(), String> {
    let target = desktop_path_from_uri(value)?;
    if target.exists() {
        fs::remove_file(&target)
            .map_err(|error| format!("Failed to delete Streamkeep file: {error}"))?;
    }
    Ok(())
}

#[cfg(not(mobile))]
fn open_uri_on_desktop(value: &str) -> Result<(), String> {
    let target = desktop_path_from_uri(value)?;
    let target = target.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = std::process::Command::new("cmd");
        command.args(["/C", "start", "", &target]);
        command
    };

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = std::process::Command::new("open");
        command.arg(&target);
        command
    };

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let mut command = {
        let mut command = std::process::Command::new("xdg-open");
        command.arg(&target);
        command
    };

    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Failed to open Streamkeep file: {error}"))
}

#[cfg(not(mobile))]
fn desktop_path_from_uri(value: &str) -> Result<PathBuf, String> {
    let target = value.strip_prefix("file://").unwrap_or(value).trim();
    if target.is_empty() {
        return Err("No Streamkeep file path was provided".to_owned());
    }
    Ok(PathBuf::from(target))
}

pub trait StreamkeepCaptureExt<R: Runtime> {
    fn streamkeep_capture(&self) -> &StreamkeepCapture<R>;
}

impl<R: Runtime, T: Manager<R>> StreamkeepCaptureExt<R> for T {
    fn streamkeep_capture(&self) -> &StreamkeepCapture<R> {
        self.state::<StreamkeepCapture<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("streamkeep-capture")
        .setup(|app, _api| {
            #[cfg(mobile)]
            {
                let capture = mobile::init(_api)?;
                app.manage(capture);
            }

            #[cfg(not(mobile))]
            {
                app.manage(StreamkeepCapture {
                    app: app.app_handle().clone(),
                });
            }

            Ok(())
        })
        .build()
}
