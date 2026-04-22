use tracing::info;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt};

mod core;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    initialize_tracing();

    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_os::init())
        .setup(|_app| {
            info!("starting Streamkeep app");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            core::app::commands::get_health_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running Streamkeep application");
}

fn initialize_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("streamkeep=debug,app_lib=debug,info"));

    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().with_target(true).compact());

    if let Err(error) = tracing::subscriber::set_global_default(subscriber) {
        eprintln!("failed to initialize tracing subscriber: {error}");
    }
}
