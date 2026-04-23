#[cfg(mobile)]
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
#[cfg(target_os = "windows")]
use std::collections::HashMap;
#[cfg(not(mobile))]
use std::{fs, path::PathBuf};
#[cfg(target_os = "windows")]
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
#[cfg(target_os = "windows")]
use tauri::Emitter;
#[cfg(target_os = "windows")]
use tauri::WebviewWindow;
use tauri::{
    Manager, Runtime,
    plugin::{Builder, TauriPlugin},
};
#[cfg(not(mobile))]
use tauri::{Url, WebviewUrl, WebviewWindowBuilder};
#[cfg(target_os = "windows")]
use tracing::{debug, error, info, warn};
#[cfg(target_os = "windows")]
use webview2_com::{
    Microsoft::Web::WebView2::Win32::COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
    WebResourceRequestedEventHandler, take_pwstr,
};
#[cfg(target_os = "windows")]
use windows::core::{HSTRING, PWSTR};

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
#[cfg(target_os = "windows")]
const DESKTOP_DEBOUNCE_WINDOW: Duration = Duration::from_secs(30);
#[cfg(target_os = "windows")]
const REQUEST_SEEN_EVENT: &str = "capture:request-seen";
#[cfg(target_os = "windows")]
const MASTER_DETECTED_EVENT: &str = "capture:master-detected";

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

        let player_window =
            WebviewWindowBuilder::new(&self.app, DESKTOP_PLAYER_LABEL, WebviewUrl::External(url))
                .title("Streamkeep Player")
                .inner_size(1120.0, 760.0)
                .resizable(true);
        #[cfg(target_os = "windows")]
        {
            let window = player_window.build().map_err(|error| error.to_string())?;
            attach_windows_hls_observer(&window, self.app.clone())?;
        }
        #[cfg(not(target_os = "windows"))]
        {
            player_window.build().map_err(|error| error.to_string())?;
        }
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

#[cfg(target_os = "windows")]
fn attach_windows_hls_observer<R: Runtime>(
    window: &WebviewWindow<R>,
    app: tauri::AppHandle<R>,
) -> Result<(), String> {
    let seen_requests = Arc::new(Mutex::new(HashMap::<String, Instant>::new()));
    window
        .with_webview(move |webview| {
            let seen_requests = Arc::clone(&seen_requests);
            let app_for_event = app.clone();

            unsafe {
                let webview2 = match webview.controller().CoreWebView2() {
                    Ok(webview2) => webview2,
                    Err(error) => {
                        error!(?error, "failed to access Windows WebView2 instance");
                        return;
                    }
                };

                let filter = HSTRING::from("*");
                if let Err(error) =
                    webview2.AddWebResourceRequestedFilter(&filter, COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL)
                {
                    error!(?error, "failed to register Windows HLS request filter");
                    return;
                }

                let handler = WebResourceRequestedEventHandler::create(Box::new(move |_, args| {
                    let Some(args) = args else {
                        return Ok(());
                    };

                    let request = args.Request()?;
                    let request_url = {
                        let mut uri = PWSTR::null();
                        request.Uri(&mut uri)?;
                        take_pwstr(uri)
                    };

                    let Some(request_type) = classify_hls_request(&request_url) else {
                        return Ok(());
                    };

                    if is_recent_duplicate(&seen_requests, request_type, &request_url) {
                        debug!(%request_url, %request_type, "ignored duplicate Windows HLS request");
                        return Ok(());
                    }

                    let headers = request.Headers().ok();
                    let referer = headers
                        .as_ref()
                        .and_then(|headers| webview_header(headers, "Referer").or_else(|| webview_header(headers, "Referrer")));
                    let user_agent = headers
                        .as_ref()
                        .and_then(|headers| webview_header(headers, "User-Agent"));
                    let cookies = headers
                        .as_ref()
                        .and_then(|headers| webview_header(headers, "Cookie"));
                    let player_state = app_for_event
                        .get_webview_window(DESKTOP_PLAYER_LABEL)
                        .map(|window| {
                            (
                                window.url().ok().map(|url| url.to_string()),
                                window.title().ok(),
                            )
                        });
                    let (page_url, page_title) = player_state.unwrap_or((None, None));
                    let master_url = if request_type == "master" {
                        Some(request_url.clone())
                    } else {
                        None
                    };
                    let referer = referer.or_else(|| page_url.clone());
                    let payload = serde_json::json!({
                        "url": request_url.clone(),
                        "requestUrl": request_url.clone(),
                        "masterUrl": master_url,
                        "pageUrl": page_url.clone(),
                        "referer": referer,
                        "userAgent": user_agent,
                        "cookies": cookies,
                        "pageTitle": page_title.clone(),
                        "documentTitle": page_title.clone(),
                        "openGraphTitle": null,
                        "headingTitle": null,
                        "titleSuggestion": page_title,
                        "detectedAt": current_timestamp(),
                        "source": "webview",
                        "requestType": request_type,
                        "confidence": if request_type == "master" { "strong" } else { "candidate" },
                    });

                    info!(
                        url = payload["url"].as_str().unwrap_or_default(),
                        request_type = payload["requestType"].as_str().unwrap_or_default(),
                        "detected Windows HLS request"
                    );
                    if let Err(error) = app_for_event.emit(REQUEST_SEEN_EVENT, payload.clone()) {
                        warn!(?error, "failed to emit Windows HLS request event");
                    }

                    if request_type == "master" || request_type == "playlist" {
                        if let Err(error) = app_for_event.emit(MASTER_DETECTED_EVENT, payload) {
                            warn!(?error, "failed to emit Windows HLS detection event");
                        }
                        if let Some(main_window) = app_for_event.get_webview_window("main") {
                            let _ = main_window.show();
                            let _ = main_window.set_focus();
                        }
                    }

                    Ok(())
                }));
                let mut token = 0_i64;
                if let Err(error) = webview2.add_WebResourceRequested(&handler, &mut token) {
                    error!(?error, "failed to attach Windows HLS request handler");
                    return;
                }

                info!(token, "attached Windows HLS request observer");
            }
        })
        .map_err(|error| error.to_string())
}

#[cfg(target_os = "windows")]
fn classify_hls_request(url: &str) -> Option<&'static str> {
    let lower_url = url.to_ascii_lowercase();
    if lower_url.contains("master.m3u8") {
        return Some("master");
    }
    if lower_url.contains(".m3u8") {
        return Some("playlist");
    }
    if lower_url.contains(".ts") {
        return Some("segment");
    }
    None
}

#[cfg(target_os = "windows")]
fn is_recent_duplicate(
    seen_requests: &Arc<Mutex<HashMap<String, Instant>>>,
    request_type: &str,
    url: &str,
) -> bool {
    let now = Instant::now();
    let key = format!("{request_type}:{url}");
    let Ok(mut seen_requests) = seen_requests.lock() else {
        return false;
    };
    seen_requests.retain(|_, seen_at| now.duration_since(*seen_at) < DESKTOP_DEBOUNCE_WINDOW);
    if let Some(seen_at) = seen_requests.get(&key)
        && now.duration_since(*seen_at) < DESKTOP_DEBOUNCE_WINDOW
    {
        return true;
    }
    seen_requests.insert(key, now);
    false
}

#[cfg(target_os = "windows")]
fn webview_header(
    headers: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2HttpRequestHeaders,
    name: &str,
) -> Option<String> {
    unsafe {
        let mut value = PWSTR::null();
        if headers.GetHeader(&HSTRING::from(name), &mut value).is_err() {
            return None;
        }
        let value = take_pwstr(value);
        let value = value.trim();
        if value.is_empty() {
            None
        } else {
            Some(value.to_owned())
        }
    }
}

#[cfg(target_os = "windows")]
fn current_timestamp() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
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
