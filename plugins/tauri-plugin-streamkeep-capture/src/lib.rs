#[cfg(mobile)]
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tauri::{
    Manager, Runtime,
    plugin::{Builder, TauriPlugin},
};

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
pub struct CreateThumbnailRequest {
    pub input_path: String,
    pub output_path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateThumbnailResult {
    pub output_path: String,
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

#[cfg(not(mobile))]
impl PlayerState {
    fn unsupported() -> Self {
        Self {
            supported: false,
            visible: false,
            loading: false,
            url: None,
            title: None,
            can_go_back: false,
            can_go_forward: false,
        }
    }
}

pub struct StreamkeepCapture<R: Runtime> {
    #[cfg(mobile)]
    mobile_plugin_handle: tauri::plugin::PluginHandle<R>,
    #[cfg(not(mobile))]
    _marker: std::marker::PhantomData<fn() -> R>,
}

impl<R: Runtime> StreamkeepCapture<R> {
    pub fn open_player(&self, request: OpenPlayerRequest) -> Result<PlayerState, String> {
        #[cfg(mobile)]
        {
            return self.run_mobile("openPlayer", request);
        }

        #[cfg(not(mobile))]
        {
            let _ = request;
            Ok(PlayerState::unsupported())
        }
    }

    pub fn get_player_state(&self) -> Result<PlayerState, String> {
        #[cfg(mobile)]
        {
            return self.run_mobile("getPlayerState", serde_json::json!({}));
        }

        #[cfg(not(mobile))]
        {
            Ok(PlayerState::unsupported())
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
            let _ = request;
            Ok(PlayerState::unsupported())
        }
    }

    pub fn remux_to_mp4(&self, request: RemuxToMp4Request) -> Result<RemuxToMp4Result, String> {
        #[cfg(mobile)]
        {
            return self.run_mobile("remuxToMp4", request);
        }

        #[cfg(not(mobile))]
        {
            let _ = request;
            Err("Streamkeep MP4 remuxing is available on Android".to_owned())
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
            let _ = request;
            Err("Publishing Streamkeep downloads is available on Android".to_owned())
        }
    }

    pub fn create_thumbnail(
        &self,
        request: CreateThumbnailRequest,
    ) -> Result<CreateThumbnailResult, String> {
        #[cfg(mobile)]
        {
            return self.run_mobile("createThumbnail", request);
        }

        #[cfg(not(mobile))]
        {
            let _ = request;
            Err("Streamkeep thumbnail extraction is available on Android".to_owned())
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
            let _ = request;
            Ok(())
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
            let _ = command;
            Ok(PlayerState::unsupported())
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
fn open_uri_on_desktop(value: &str) -> Result<(), String> {
    let target = value
        .strip_prefix("file://")
        .unwrap_or(value)
        .trim()
        .to_owned();
    if target.is_empty() {
        return Err("No Streamkeep file path was provided".to_owned());
    }

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
                    _marker: std::marker::PhantomData::<fn() -> R>,
                });
            }

            Ok(())
        })
        .build()
}
