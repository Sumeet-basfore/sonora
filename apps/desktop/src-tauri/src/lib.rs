use sonora_common::{init_logging, LogConfig};
use sonora_core::{SonoraApp, SonoraConfig};
use std::sync::Mutex;

struct AppState {
    _app: Mutex<Option<SonoraApp>>,
}

#[tauri::command]
fn get_system_status() -> String {
    "Sonora Core v0.1.0 Ready".to_string()
}

pub fn run() {
    init_logging(&LogConfig::default());

    let config = SonoraConfig::default();
    let app_instance = SonoraApp::new(config).ok();

    tauri::Builder::default()
        .manage(AppState {
            _app: Mutex::new(app_instance),
        })
        .invoke_handler(tauri::generate_handler![get_system_status])
        .run(tauri::generate_context!())
        .expect("error while running sonora desktop application");
}
