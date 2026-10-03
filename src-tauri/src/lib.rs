mod analysis;
mod error;
mod ipc;
mod notify;
mod presence;
mod storage;
mod telemetry;
mod tray;
mod types;

use parking_lot::Mutex;
use tauri::Manager;
use tracing_subscriber::EnvFilter;

use crate::error::load_settings;
use crate::ipc::{spawn_sampler, AppState, Inner};
use crate::storage::Database;
use crate::telemetry::Collector;

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if bytes == 0 {
        return "0 B".into();
    }
    let mut i = (bytes as f64).log(1024.0).floor() as usize;
    i = i.min(UNITS.len() - 1);
    let mut value = bytes as f64 / 1024f64.powi(i as i32);
    if value >= 1000.0 && i < UNITS.len() - 1 {
        i += 1;
        value = bytes as f64 / 1024f64.powi(i as i32);
    }
    let digits = if i == 0 {
        0
    } else if value >= 100.0 {
        0
    } else if value >= 10.0 {
        1
    } else {
        1
    };
    if i == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.digits$} {}", UNITS[i])
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let app_dir = app.path().app_data_dir().unwrap_or_else(|_| {
                std::env::temp_dir().join("wtf-is-my-pc-doing")
            });
            std::fs::create_dir_all(&app_dir)?;
            let settings = load_settings(&app_dir);
            let db = Database::open(&app_dir)?;
            let collector = Collector::new();
            app.manage(AppState {
                inner: Mutex::new(Inner {
                    collector,
                    snapshot: Default::default(),
                }),
                settings: Mutex::new(settings),
                db,
                app_dir,
            });
            tray::setup(app)?;
            crate::presence::start();
            spawn_sampler(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if let Some(state) = window.try_state::<AppState>() {
                    if state.settings.lock().minimize_to_tray {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            ipc::get_snapshot,
            ipc::get_settings,
            ipc::save_app_settings,
            ipc::get_hardware,
            ipc::get_power,
            ipc::get_connections,
            ipc::get_process_details,
            ipc::kill_process,
            ipc::reveal_path,
            ipc::search_web,
            ipc::get_history,
            ipc::get_event_history,
            ipc::get_alerts,
            ipc::acknowledge_alert,
            ipc::acknowledge_all_alerts,
            ipc::pause_monitoring,
            ipc::complete_onboarding
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|_app, event| {
            if let tauri::RunEvent::Exit = event {
                crate::presence::shutdown();
            }
        });
}
