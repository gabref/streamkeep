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
    fn run_mobile<T: Serialize>(&self, command: &str, payload: T) -> Result<PlayerState, String> {
        self.mobile_plugin_handle
            .run_mobile_plugin(command, payload)
            .map_err(|error| error.to_string())
    }
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
