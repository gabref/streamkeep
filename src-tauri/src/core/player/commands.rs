use tauri::{AppHandle, Runtime};
use tauri_plugin_streamkeep_capture::{
    OpenPlayerRequest, PlayerLoadUrlRequest, PlayerState, StreamkeepCaptureExt,
};

#[tauri::command]
pub fn open_player_command<R: Runtime>(
    app: AppHandle<R>,
    url: Option<String>,
) -> Result<PlayerState, String> {
    app.streamkeep_capture()
        .open_player(OpenPlayerRequest { url })
}

#[tauri::command]
pub fn get_player_state_command<R: Runtime>(app: AppHandle<R>) -> Result<PlayerState, String> {
    app.streamkeep_capture().get_player_state()
}

#[tauri::command]
pub fn player_go_back_command<R: Runtime>(app: AppHandle<R>) -> Result<PlayerState, String> {
    app.streamkeep_capture().go_back()
}

#[tauri::command]
pub fn player_go_forward_command<R: Runtime>(app: AppHandle<R>) -> Result<PlayerState, String> {
    app.streamkeep_capture().go_forward()
}

#[tauri::command]
pub fn player_reload_command<R: Runtime>(app: AppHandle<R>) -> Result<PlayerState, String> {
    app.streamkeep_capture().reload()
}

#[tauri::command]
pub fn player_load_url_command<R: Runtime>(
    app: AppHandle<R>,
    url: String,
) -> Result<PlayerState, String> {
    app.streamkeep_capture()
        .load_url(PlayerLoadUrlRequest { url })
}
