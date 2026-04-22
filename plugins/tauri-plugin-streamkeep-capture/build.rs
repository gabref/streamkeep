const COMMANDS: &[&str] = &[
    "registerListener",
    "removeListener",
    "openPlayer",
    "getPlayerState",
    "goBack",
    "goForward",
    "reload",
    "loadUrl",
    "remuxToMp4",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .build();
}
