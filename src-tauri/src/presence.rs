//! Discord Rich Presence over local IPC to the Discord client.
//!
//! The empty Playing tile with a question mark is Discord detecting the exe
//! as a registered game. This module sends SET_ACTIVITY so CPU/RAM and GitHub
//! replace that tile. Discord must be running, and Show on Discord must be on.

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use discord_rich_presence::activity::{Activity, Assets, Button, Timestamps};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use parking_lot::Mutex;

use crate::types::{AppSettings, Snapshot};

const GITHUB_URL: &str = "https://github.com/osfv/WhatTheFuckIsMyPcDoing";
const IMAGE_URL: &str = "https://raw.githubusercontent.com/osfv/WhatTheFuckIsMyPcDoing/main/discord-icon.png";
const DISCORD_APPLICATION_ID: &str = "1544372119251722262";
const UPDATE_EVERY: Duration = Duration::from_secs(15);
const RETRY_EVERY: Duration = Duration::from_secs(5);

static TX: OnceLock<Sender<()>> = OnceLock::new();
static LATEST: OnceLock<Mutex<Option<PresenceInput>>> = OnceLock::new();

#[derive(Clone)]
struct PresenceInput {
    enabled: bool,
    application_id: String,
    paused: bool,
    cpu: f32,
    ram: f32,
    gpu: Option<f32>,
    headline: String,
}

impl PresenceInput {
    fn from_parts(settings: &AppSettings, snapshot: &Snapshot) -> Self {
        Self {
            enabled: settings.discord_presence,
            application_id: resolve_application_id(settings),
            paused: settings.pause_monitoring || snapshot.paused,
            cpu: snapshot.cpu.utilization,
            ram: snapshot.memory.utilization,
            gpu: snapshot.gpu.utilization.filter(|_| snapshot.gpu.available),
            headline: snapshot.interpretation.headline.clone(),
        }
    }

    fn fingerprint(&self) -> String {
        format!(
            "{}|{}|{}|{:.0}|{:.0}|{:.0}|{}",
            self.enabled,
            self.application_id.trim(),
            self.paused,
            self.cpu,
            self.ram,
            self.gpu.unwrap_or(-1.0),
            self.headline
        )
    }
}

fn latest() -> &'static Mutex<Option<PresenceInput>> {
    LATEST.get_or_init(|| Mutex::new(None))
}

pub fn start() {
    let (tx, rx) = mpsc::channel();
    if TX.set(tx).is_err() {
        return;
    }
    let _ = std::thread::Builder::new()
        .name("discord-presence".into())
        .spawn(move || run_loop(rx));
}

pub fn publish(settings: &AppSettings, snapshot: &Snapshot) {
    *latest().lock() = Some(PresenceInput::from_parts(settings, snapshot));
    if let Some(tx) = TX.get() {
        let _ = tx.send(());
    }
}

pub fn shutdown() {
    *latest().lock() = None;
    if let Some(tx) = TX.get() {
        let _ = tx.send(());
    }
}

fn run_loop(rx: mpsc::Receiver<()>) {
    let hostname = sysinfo::System::host_name().unwrap_or_default();
    let mut client: Option<DiscordIpcClient> = None;
    let mut connected_id = String::new();
    let mut last_connect = Instant::now() - RETRY_EVERY;
    let mut last_push = Instant::now() - UPDATE_EVERY;
    let mut last_sent = String::new();
    let mut started_at: Option<i64> = None;

    loop {
        match rx.recv_timeout(Duration::from_millis(800)) {
            Ok(()) | Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                disconnect(&mut client, &mut connected_id, &mut started_at);
                return;
            }
        }

        let Some(input) = latest().lock().clone() else {
            disconnect(&mut client, &mut connected_id, &mut started_at);
            continue;
        };

        if !input.enabled {
            disconnect(&mut client, &mut connected_id, &mut started_at);
            last_sent.clear();
            continue;
        }

        let app_id = input.application_id.trim().to_string();
        if app_id.is_empty() {
            tracing::warn!("Discord presence has no application id");
            disconnect(&mut client, &mut connected_id, &mut started_at);
            continue;
        }

        if connected_id != app_id {
            disconnect(&mut client, &mut connected_id, &mut started_at);
            last_sent.clear();
        }

        if client.is_none() {
            if last_connect.elapsed() < RETRY_EVERY {
                continue;
            }
            last_connect = Instant::now();
            let mut next = DiscordIpcClient::new(&app_id);
            match next.connect() {
                Ok(()) => {
                    client = Some(next);
                    connected_id = app_id;
                    started_at = Some(chrono::Utc::now().timestamp());
                    last_sent.clear();
                    tracing::info!("Discord presence connected");
                }
                Err(err) => {
                    tracing::warn!("Discord presence connect failed: {err}");
                    continue;
                }
            }
        }

        let fingerprint = input.fingerprint();
        if fingerprint == last_sent && last_push.elapsed() < UPDATE_EVERY {
            continue;
        }

        let details = details_line(&input);
        let state = state_line(&input, &hostname);
        let start = started_at.unwrap_or_else(|| chrono::Utc::now().timestamp());
        let Some(ipc) = client.as_mut() else {
            continue;
        };

        let sent = match set_activity(ipc, &details, &state, start) {
            Ok(()) => true,
            Err(err) => {
                tracing::warn!("Discord presence update failed: {err}");
                false
            }
        };

        last_push = Instant::now();
        if sent {
            last_sent = fingerprint;
        } else {
            disconnect(&mut client, &mut connected_id, &mut started_at);
            last_sent.clear();
        }
    }
}

fn set_activity(
    ipc: &mut DiscordIpcClient,
    details: &str,
    state: &str,
    start_secs: i64,
) -> Result<(), discord_rich_presence::error::Error> {
    let activity = Activity::new()
        .details(details)
        .state(state)
        .timestamps(Timestamps::new().start(start_secs))
        .assets(
            Assets::new()
                .large_image(IMAGE_URL)
                .large_text("What The F*ck Is My PC Doing?"),
        )
        .buttons(vec![Button::new("GitHub", GITHUB_URL)]);
    ipc.set_activity(activity)
}

fn resolve_application_id(settings: &AppSettings) -> String {
    let custom = settings.discord_application_id.trim();
    if looks_like_snowflake(custom) {
        return custom.to_string();
    }
    DISCORD_APPLICATION_ID.to_string()
}

fn looks_like_snowflake(id: &str) -> bool {
    let digits = id.len();
    digits >= 17 && digits <= 20 && id.bytes().all(|b| b.is_ascii_digit())
}

fn disconnect(
    client: &mut Option<DiscordIpcClient>,
    connected_id: &mut String,
    started_at: &mut Option<i64>,
) {
    if let Some(mut ipc) = client.take() {
        let _ = ipc.clear_activity();
        let _ = ipc.close();
    }
    connected_id.clear();
    *started_at = None;
}

fn details_line(input: &PresenceInput) -> String {
    if input.paused {
        "Paused".into()
    } else {
        format!("CPU {:.0}%  RAM {:.0}%", input.cpu, input.ram)
    }
}

fn state_line(input: &PresenceInput, hostname: &str) -> String {
    if input.paused {
        return clip(if hostname.is_empty() {
            "Monitoring paused"
        } else {
            hostname
        });
    }
    if let Some(gpu) = input.gpu {
        return format!("GPU {gpu:.0}%");
    }
    let headline = input.headline.trim();
    if !headline.is_empty() && headline.chars().count() <= 80 {
        return headline.to_string();
    }
    if !hostname.is_empty() {
        return clip(hostname);
    }
    if !headline.is_empty() {
        return clip(headline);
    }
    "Watching this PC".into()
}

fn clip(text: &str) -> String {
    const MAX: usize = 120;
    let mut out = String::new();
    for (i, ch) in text.chars().enumerate() {
        if i >= MAX {
            break;
        }
        out.push(ch);
    }
    out
}
