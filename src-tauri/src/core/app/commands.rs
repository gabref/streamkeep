use streamkeep_app_core::HealthSnapshot;

#[tauri::command]
pub fn get_health_command() -> HealthSnapshot {
    HealthSnapshot::new(
        "Streamkeep",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
    )
}
