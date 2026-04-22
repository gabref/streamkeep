const COMMANDS: &[&str] = &[
    "openPlayer",
    "getPlayerState",
    "goBack",
    "goForward",
    "reload",
    "loadUrl",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .build();
}
