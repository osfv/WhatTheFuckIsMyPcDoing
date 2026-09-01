use std::path::PathBuf;

use parking_lot::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::error::{save_settings, AppError};
use crate::storage::Database;
use crate::telemetry::Collector;
use crate::types::{
    AppSettings, HardwareInfo, HistoryPoint, NetConnection, ProcessDetails, Snapshot,
};

pub struct Inner {
    pub collector: Collector,
    pub snapshot: Snapshot,
}

pub struct AppState {
    pub inner: Mutex<Inner>,
    pub settings: Mutex<AppSettings>,
    pub db: Database,
    pub app_dir: PathBuf,
}

impl AppState {
    pub fn current_snapshot(&self) -> Snapshot {
        self.inner.lock().snapshot.clone()
    }
}

#[tauri::command]
pub fn get_snapshot(state: State<AppState>) -> Snapshot {
    state.current_snapshot()
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> AppSettings {
    state.settings.lock().clone()
}

#[tauri::command]
pub fn save_app_settings(
    app: AppHandle,
    state: State<AppState>,
    settings: AppSettings,
) -> Result<AppSettings, AppError> {
    save_settings(&state.app_dir, &settings)?;
    let start = settings.start_on_login;
    let paused = settings.pause_monitoring;
    *state.settings.lock() = settings.clone();
    let _ = apply_autostart(&app, start);
    crate::tray::sync_pause_label(&app, paused);
    crate::presence::publish(&settings, &state.current_snapshot());
    Ok(settings)
}

fn apply_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let mgr = app.autolaunch();
    if enabled {
        mgr.enable().map_err(|e| e.to_string())?;
    } else {
        mgr.disable().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_hardware(state: State<AppState>) -> HardwareInfo {
    state.inner.lock().collector.hardware()
}

#[tauri::command]
pub fn get_connections(state: State<AppState>) -> Vec<NetConnection> {
    state.inner.lock().collector.connections()
}

#[tauri::command]
pub fn get_process_details(state: State<AppState>, pid: u32) -> Result<ProcessDetails, String> {
    let inner = state.inner.lock();
    inner
        .collector
        .process_details(pid, &inner.snapshot)
        .ok_or_else(|| "Process is no longer running.".into())
}

#[tauri::command]
pub fn kill_process(state: State<AppState>, pid: u32) -> Result<(), String> {
    state.inner.lock().collector.kill_process(pid)
}

#[tauri::command]
pub fn reveal_path(path: String) -> Result<(), String> {
    let path = PathBuf::from(&path);
    if !path.exists() {
        return Err("That file is no longer on disk.".into());
    }
    std::process::Command::new("explorer")
        .arg(format!("/select,{}", path.display()))
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn search_web(query: String) -> Result<(), String> {
    let encoded = urlencoding_lite(&query);
    let url = format!("https://www.google.com/search?q={}", encoded);
    open::that(url).map_err(|e| e.to_string())
}

fn urlencoding_lite(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[tauri::command]
pub fn get_history(state: State<AppState>, hours: u32) -> Result<Vec<HistoryPoint>, AppError> {
    let from = chrono::Utc::now().timestamp() - (hours as i64 * 3600);
    state.db.history_since(from)
}

#[tauri::command]
pub fn get_event_history(state: State<AppState>, hours: u32) -> Result<Vec<crate::types::ActivityEvent>, AppError> {
    let from = chrono::Utc::now().timestamp() - (hours as i64 * 3600);
    state.db.events_since(from)
}

#[tauri::command]
pub fn get_alerts(state: State<AppState>) -> Result<Vec<crate::types::Alert>, AppError> {
    state.db.recent_alerts(80)
}

#[tauri::command]
pub fn acknowledge_alert(state: State<AppState>, id: i64) -> Result<(), AppError> {
    state.db.acknowledge(id)
}

#[tauri::command]
pub fn acknowledge_all_alerts(state: State<AppState>) -> Result<(), AppError> {
    state.db.acknowledge_all()
}

#[tauri::command]
pub fn pause_monitoring(
    app: AppHandle,
    state: State<AppState>,
    paused: bool,
) -> Result<AppSettings, AppError> {
    let mut settings = state.settings.lock();
    settings.pause_monitoring = paused;
    save_settings(&state.app_dir, &settings)?;
    crate::tray::sync_pause_label(&app, paused);
    let copy = settings.clone();
    drop(settings);
    crate::presence::publish(&copy, &state.current_snapshot());
    Ok(copy)
}

#[tauri::command]
pub fn complete_onboarding(state: State<AppState>, settings: AppSettings) -> Result<AppSettings, AppError> {
    let mut next = settings;
    next.onboarding_complete = true;
    save_settings(&state.app_dir, &next)?;
    *state.settings.lock() = next.clone();
    Ok(next)
}

pub fn spawn_sampler(app: AppHandle) {
    std::thread::Builder::new()
        .name("telemetry".into())
        .spawn(move || loop {
            let state = app.state::<AppState>();
            let settings = state.settings.lock().clone();
            let interval = std::time::Duration::from_millis(
                settings.sample_interval_ms.clamp(500, 10_000),
            );
            if !settings.pause_monitoring {
                let snapshot = {
                    let mut inner = state.inner.lock();
                    let snap = inner.collector.tick(&settings, &state.db);
                    inner.snapshot = snap.clone();
                    snap
                };
                let _ = app.emit("snapshot", &snapshot);
                crate::tray::update_tooltip(&app, &snapshot);
                crate::presence::publish(&settings, &snapshot);
            } else {
                let snapshot = state.current_snapshot();
                crate::presence::publish(&settings, &snapshot);
            }
            drop(state);
            std::thread::sleep(interval);
        })
        .expect("telemetry thread");
}

