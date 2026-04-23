const COMMANDS: &[&str] = &[
    "register_listener",
    "registerListener",
    "remove_listener",
    "removeListener",
    "openPlayer",
    "getPlayerState",
    "goBack",
    "goForward",
    "reload",
    "loadUrl",
    "remuxToMp4",
    "publishToDownloads",
    "createThumbnail",
    "deletePublishedDownload",
    "startDownloadKeepAlive",
    "stopDownloadKeepAlive",
    "openUri",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .build();
}
