use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

use crate::types::Snapshot;

const PAUSE_LABEL: &str = "Pause monitoring";
const RESUME_LABEL: &str = "Resume monitoring";

struct PauseMenuItem(MenuItem<tauri::Wry>);

pub fn setup(app: &tauri::App) -> tauri::Result<()> {
    let paused = app
        .try_state::<crate::ipc::AppState>()
        .map(|s| s.settings.lock().pause_monitoring)
        .unwrap_or(false);
    let open = MenuItem::with_id(app, "open", "Open dashboard", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", pause_label(paused), true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &pause, &settings, &sep, &quit])?;
    app.manage(PauseMenuItem(pause));

    let mut builder = TrayIconBuilder::with_id("tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("WTF Is My PC Doing")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main(app),
            "settings" => {
                show_main(app);
                let _ = app.emit("navigate", "settings");
            }
            "pause" => {
                if let Some(state) = app.try_state::<crate::ipc::AppState>() {
                    let mut settings = state.settings.lock();
                    settings.pause_monitoring = !settings.pause_monitoring;
                    let paused = settings.pause_monitoring;
                    let copy = settings.clone();
                    drop(settings);
                    let _ = crate::error::save_settings(&state.app_dir, &copy);
                    let snap = state.current_snapshot();
                    crate::presence::publish(&copy, &snap);
                    let _ = app.emit("settings-updated", copy);
                    sync_pause_label(app, paused);
                    tracing::info!(paused, "monitoring pause toggled from tray");
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

pub fn sync_pause_label(app: &AppHandle, paused: bool) {
    if let Some(item) = app.try_state::<PauseMenuItem>() {
        let _ = item.0.set_text(pause_label(paused));
    }
}

fn pause_label(paused: bool) -> &'static str {
    if paused {
        RESUME_LABEL
    } else {
        PAUSE_LABEL
    }
}

fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn update_tooltip(app: &AppHandle, snapshot: &Snapshot) {
    let Some(tray) = app.tray_by_id("tray") else {
        return;
    };
    let gpu = snapshot
        .gpu
        .utilization
        .map(|v| format!("{v:.0}%"))
        .unwrap_or_else(|| "n/a".into());
    let text = format!(
        "CPU {:.0}%  GPU {gpu}  RAM {:.0}%\n{}",
        snapshot.cpu.utilization,
        snapshot.memory.utilization,
        snapshot.interpretation.headline
    );
    let _ = tray.set_tooltip(Some(text));
}
