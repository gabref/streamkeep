use serde::de::DeserializeOwned;
use tauri::{Runtime, plugin::PluginApi, plugin::mobile::PluginInvokeError};

use crate::{PLUGIN_IDENTIFIER, StreamkeepCapture};

pub fn init<R: Runtime, C: DeserializeOwned>(
    api: PluginApi<R, C>,
) -> Result<StreamkeepCapture<R>, PluginInvokeError> {
    #[cfg(target_os = "android")]
    let mobile_plugin_handle =
        api.register_android_plugin(PLUGIN_IDENTIFIER, "StreamkeepCapturePlugin")?;

    Ok(StreamkeepCapture {
        mobile_plugin_handle,
    })
}
