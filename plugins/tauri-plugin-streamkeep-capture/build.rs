const COMMANDS: &[&str] = &[
    "registerListener",
    "removeListener",
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
