use std::sync::Once;

use tracing::info;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt};

mod core;

static TRACING_INIT: Once = Once::new();

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
        .plugin(tauri_plugin_streamkeep_capture::init())
        .setup(|_app| {
            info!("starting Streamkeep app");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            core::app::commands::get_health_command,
            core::download::commands::delete_download_history_command,
            core::download::commands::list_download_history_command,
            core::download::commands::open_download_command,
            core::download::commands::start_download_command,
            core::settings::commands::get_download_settings_command,
            core::settings::commands::reset_download_directory_command,
            core::settings::commands::set_download_directory_command,
            core::player::commands::get_player_state_command,
            core::player::commands::open_player_command,
            core::player::commands::player_go_back_command,
            core::player::commands::player_go_forward_command,
            core::player::commands::player_reload_command,
            core::player::commands::player_load_url_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running Streamkeep application");
}

fn initialize_tracing() {
    TRACING_INIT.call_once(|| {
        let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new(
                "streamkeep=debug,streamkeep_download_core=debug,tauri_plugin_streamkeep_capture=debug,app_lib=debug,info",
            )
        });

        let subscriber = tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().with_target(true).compact());

        if let Err(error) = tracing::subscriber::set_global_default(subscriber) {
            eprintln!("failed to initialize tracing subscriber: {error}");
        }
    });
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn Java_app_streamkeep_mobile_MainActivity_initializeRustlsPlatformVerifier(
    mut env: jni::JNIEnv<'_>,
    _activity: jni::objects::JObject<'_>,
    context: jni::objects::JObject<'_>,
) {
    initialize_tracing();
    match rustls_platform_verifier::android::init_with_env(&mut env, context) {
        Ok(()) => tracing::info!("initialized Android rustls platform verifier"),
        Err(error) => tracing::error!(
            ?error,
            "failed to initialize Android rustls platform verifier"
        ),
    }
}
