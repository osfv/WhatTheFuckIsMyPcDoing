//! Battery status and estimated power use per app.
//!
//! Windows keeps real per-app energy numbers in the Energy Estimation Engine
//! (E3) and the SRUM database, but neither is a public API: SRUM is an ESE
//! database that needs SYSTEM/admin access and a JET parser, and E3 has no
//! documented interface. So per-app power here is a model built from CPU, GPU
//! and disk activity. When the battery reports a real discharge rate, or the CPU
//! and GPU expose package power sensors, those measurements calibrate the model.
#![allow(non_snake_case)]

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use serde::Deserialize;

use crate::analysis::{grouped_process_rows, normalize_name};
use crate::types::{AppPower, BatteryStatus, CpuMetrics, GpuMetrics, PowerReport, ProcessRow};

/// Share of measured battery drain that apps can be blamed for. The rest is
/// screen, radios, chipset and memory.
const APP_SHARE_OF_DRAIN: f32 = 0.85;
const SMOOTHING: f32 = 0.25;
const MAX_APPS: usize = 12;

/// Battery readings straight from Windows, before interpretation.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RawBattery {
    pub present: bool,
    pub ac_online: Option<bool>,
    pub charging: Option<bool>,
    pub percent: Option<f32>,
    pub os_time_remaining_sec: Option<u64>,
    pub remaining_mwh: Option<u64>,
    pub full_mwh: Option<u64>,
    pub design_mwh: Option<u64>,
    pub charge_rate_mw: Option<u64>,
    pub discharge_rate_mw: Option<u64>,
    pub cycle_count: Option<u32>,
    pub name: Option<String>,
    pub manufacturer: Option<String>,
    #[serde(skip)]
    pub mocked: bool,
}

/// Interprets the fields of `SYSTEM_POWER_STATUS`.
pub fn from_power_status(ac_line: u8, flag: u8, life_percent: u8, life_time: u32) -> RawBattery {
    const NO_BATTERY: u8 = 128;
    const CHARGING: u8 = 8;
    const UNKNOWN: u8 = 255;
    let present = flag != UNKNOWN && flag & NO_BATTERY == 0;
    RawBattery {
        present,
        ac_online: match ac_line {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        },
        charging: present.then_some(flag & CHARGING != 0),
        percent: (present && life_percent <= 100).then_some(life_percent as f32),
        os_time_remaining_sec: (present && life_time != u32::MAX).then_some(life_time as u64),
        ..Default::default()
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename = "BatteryStatus")]
pub struct WmiBatteryStatus {
    pub ChargeRate: Option<i32>,
    pub DischargeRate: Option<i32>,
    pub RemainingCapacity: Option<u32>,
    pub Charging: Option<bool>,
    pub PowerOnline: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename = "BatteryFullChargedCapacity")]
pub struct WmiFullCapacity {
    pub FullChargedCapacity: Option<u32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename = "BatteryStaticData")]
pub struct WmiStaticData {
    pub DesignedCapacity: Option<u32>,
    pub DeviceName: Option<String>,
    pub ManufactureName: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename = "BatteryCycleCount")]
pub struct WmiCycleCount {
    pub CycleCount: Option<u32>,
}

fn sum_positive<I: Iterator<Item = Option<i64>>>(values: I) -> Option<u64> {
    let mut total = None;
    for v in values.flatten() {
        if v > 0 {
            *total.get_or_insert(0u64) += v as u64;
        }
    }
    total
}

fn clean(s: &Option<String>) -> Option<String> {
    s.as_ref()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Folds the `root\wmi` battery classes into the `GetSystemPowerStatus` view.
/// Laptops with two packs report one row per pack, so capacities are summed.
pub fn merge_wmi(
    mut raw: RawBattery,
    status: &[WmiBatteryStatus],
    full: &[WmiFullCapacity],
    statics: &[WmiStaticData],
    cycles: &[WmiCycleCount],
) -> RawBattery {
    if !raw.present && status.is_empty() {
        return raw;
    }
    raw.present = true;
    raw.remaining_mwh = sum_positive(status.iter().map(|s| s.RemainingCapacity.map(i64::from)))
        .or(raw.remaining_mwh);
    raw.charge_rate_mw =
        sum_positive(status.iter().map(|s| s.ChargeRate.map(i64::from))).or(raw.charge_rate_mw);
    raw.discharge_rate_mw = sum_positive(status.iter().map(|s| s.DischargeRate.map(i64::from)))
        .or(raw.discharge_rate_mw);
    if raw.charging.is_none() {
        raw.charging = status.iter().find_map(|s| s.Charging);
    }
    if raw.ac_online.is_none() {
        raw.ac_online = status.iter().find_map(|s| s.PowerOnline);
    }
    raw.full_mwh =
        sum_positive(full.iter().map(|f| f.FullChargedCapacity.map(i64::from))).or(raw.full_mwh);
    raw.design_mwh =
        sum_positive(statics.iter().map(|s| s.DesignedCapacity.map(i64::from))).or(raw.design_mwh);
    raw.name = statics
        .iter()
        .find_map(|s| clean(&s.DeviceName))
        .or(raw.name);
    raw.manufacturer = statics
        .iter()
        .find_map(|s| clean(&s.ManufactureName))
        .or(raw.manufacturer);
    raw.cycle_count = cycles
        .iter()
        .filter_map(|c| c.CycleCount)
        .filter(|c| *c > 0)
        .max()
        .or(raw.cycle_count);
    if raw.percent.is_none() {
        if let (Some(rem), Some(full)) = (raw.remaining_mwh, raw.full_mwh) {
            if full > 0 {
                raw.percent = Some((rem as f32 / full as f32 * 100.0).min(100.0));
            }
        }
    }
    raw
}

fn mwh_to_wh(v: Option<u64>) -> Option<f32> {
    v.filter(|v| *v > 0).map(|v| v as f32 / 1000.0)
}

fn battery_state(
    raw: &RawBattery,
    charge_w: Option<f32>,
    discharge_w: Option<f32>,
) -> &'static str {
    let on_ac = raw.ac_online == Some(true);
    if raw.charging == Some(true) || (on_ac && charge_w.is_some()) {
        "charging"
    } else if raw.ac_online == Some(false) || (raw.ac_online.is_none() && discharge_w.is_some()) {
        "discharging"
    } else if on_ac && raw.percent.is_some_and(|p| p >= 99.0) {
        "full"
    } else if on_ac {
        "plugged"
    } else {
        "unknown"
    }
}

pub fn battery_status(raw: &RawBattery) -> BatteryStatus {
    if !raw.present {
        return BatteryStatus {
            present: false,
            state: "none".into(),
            health_label: "Unknown".into(),
            headline: "No battery on this PC".into(),
            summary: "This looks like a desktop, so it runs straight off the wall. Power numbers below are estimates of what is working hardest.".into(),
            mocked: raw.mocked,
            ..Default::default()
        };
    }

    let percent = raw.percent.map(|p| p.clamp(0.0, 100.0));
    let charge_w = raw
        .charge_rate_mw
        .filter(|r| *r > 0)
        .map(|r| r as f32 / 1000.0);
    let discharge_w = raw
        .discharge_rate_mw
        .filter(|r| *r > 0)
        .map(|r| r as f32 / 1000.0);
    let state = battery_state(raw, charge_w, discharge_w);

    let remaining_wh = mwh_to_wh(raw.remaining_mwh);
    let full_wh = mwh_to_wh(raw.full_mwh);
    let design_wh = mwh_to_wh(raw.design_mwh);

    let rate_w = match state {
        "charging" => charge_w,
        "discharging" => discharge_w,
        _ => None,
    };
    let time_remaining_sec = match (state, remaining_wh, rate_w) {
        ("discharging", Some(rem), Some(w)) if w > 0.05 => Some((rem / w * 3600.0) as u64),
        ("discharging", _, _) => raw.os_time_remaining_sec,
        _ => None,
    };
    let time_to_full_sec = match (state, remaining_wh, full_wh, rate_w) {
        ("charging", Some(rem), Some(full), Some(w)) if w > 0.05 && full > rem => {
            Some(((full - rem) / w * 3600.0) as u64)
        }
        _ => None,
    };

    let health_percent = match (full_wh, design_wh) {
        (Some(full), Some(design)) if design > 0.0 => Some((full / design * 100.0).min(100.0)),
        _ => None,
    };
    let health_label = match health_percent {
        Some(h) if h >= 80.0 => "Healthy",
        Some(h) if h >= 60.0 => "Worn",
        Some(_) => "Heavily worn",
        None => "Unknown",
    }
    .to_string();

    let pct = percent
        .map(|p| format!("{p:.0}%"))
        .unwrap_or_else(|| "unknown charge".into());
    let (headline, mut summary) = match state {
        "charging" => (
            format!("Charging, {pct}"),
            match time_to_full_sec {
                Some(s) => format!("Full in about {}.", human_duration(s)),
                None => "Plugged in and topping up.".into(),
            },
        ),
        "discharging" => (
            if percent.is_some_and(|p| p <= 15.0) {
                format!("Battery is low, {pct}")
            } else {
                format!("On battery, {pct}")
            },
            match (time_remaining_sec, rate_w) {
                (Some(s), Some(w)) => format!(
                    "About {} left at the current {w:.1} W drain.",
                    human_duration(s)
                ),
                (Some(s), None) => format!("Windows estimates about {} left.", human_duration(s)),
                _ => "Windows has not estimated time left yet.".into(),
            },
        ),
        "full" => (format!("Plugged in, {pct}"), "Fully charged.".into()),
        "plugged" => (
            format!("Plugged in, {pct}"),
            "On wall power, not charging.".into(),
        ),
        _ => (
            format!("Battery at {pct}"),
            "Windows did not say whether it is charging.".into(),
        ),
    };
    if let Some(h) = health_percent.filter(|h| *h < 80.0) {
        summary.push_str(&format!(
            " The battery holds {h:.0}% of what it did new, so expect shorter runtimes."
        ));
    }

    BatteryStatus {
        present: true,
        state: state.into(),
        percent,
        ac_online: raw.ac_online,
        rate_w,
        time_remaining_sec,
        time_to_full_sec,
        remaining_wh,
        full_wh,
        design_wh,
        health_percent,
        health_label,
        cycle_count: raw.cycle_count,
        name: raw.name.clone(),
        manufacturer: raw.manufacturer.clone(),
        headline,
        summary,
        mocked: raw.mocked,
    }
}

fn human_duration(secs: u64) -> String {
    let mins = secs / 60;
    if mins < 1 {
        return "less than a minute".into();
    }
    if mins < 60 {
        return format!("{mins} min");
    }
    let (h, m) = (mins / 60, mins % 60);
    if m == 0 {
        format!("{h} h")
    } else {
        format!("{h} h {m} min")
    }
}

/// Watts a typical laptop spends at full load on each component, used when
/// no sensor reports real power.
#[derive(Debug, Clone, Copy)]
pub struct PowerModel {
    pub cpu_full_w: f32,
    pub gpu_full_w: f32,
    pub disk_w_per_mbps: f32,
}

impl Default for PowerModel {
    fn default() -> Self {
        Self {
            cpu_full_w: 15.0,
            gpu_full_w: 12.0,
            disk_w_per_mbps: 0.02,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Sensors {
    pub cpu_total_percent: f32,
    pub cpu_package_w: Option<f32>,
    pub gpu_w: Option<f32>,
    pub system_draw_w: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppEstimate {
    pub key: String,
    pub name: String,
    pub pid: u32,
    pub cpu_w: f32,
    pub gpu_w: f32,
    pub disk_w: f32,
    pub cpu: f32,
    pub gpu: f32,
    pub disk_bps: u64,
}

impl AppEstimate {
    pub fn watts(&self) -> f32 {
        self.cpu_w + self.gpu_w + self.disk_w
    }
}

/// Instantaneous per-app estimate, grouped the same way as the Processes page.
/// Returns the estimates and which data source drove them.
pub fn estimate_apps(
    rows: &[ProcessRow],
    sensors: &Sensors,
    model: &PowerModel,
) -> (Vec<AppEstimate>, &'static str) {
    let groups: Vec<ProcessRow> = grouped_process_rows(rows)
        .into_iter()
        .filter(|g| g.pid != 0 && normalize_name(&g.name) != "idle")
        .collect();
    let gpu_sum: f32 = groups
        .iter()
        .filter_map(|g| g.gpu)
        .map(|g| g.max(0.0))
        .sum();
    let proc_cpu_sum: f32 = groups.iter().map(|g| g.cpu.max(0.0)).sum();
    let cpu_total = sensors.cpu_total_percent.max(proc_cpu_sum).max(0.1);

    let mut apps: Vec<AppEstimate> = groups
        .iter()
        .map(|g| {
            let cpu = g.cpu.max(0.0);
            let gpu = g.gpu.unwrap_or(0.0).max(0.0);
            let cpu_w = match sensors.cpu_package_w {
                Some(pkg) if pkg > 0.0 => pkg * cpu / cpu_total,
                _ => model.cpu_full_w * cpu / 100.0,
            };
            let gpu_w = match sensors.gpu_w {
                Some(w) if w > 0.0 && gpu_sum > 0.0 => w * gpu / gpu_sum,
                _ => model.gpu_full_w * gpu / 100.0,
            };
            let disk_w = model.disk_w_per_mbps * g.disk_bps as f32 / (1024.0 * 1024.0);
            AppEstimate {
                key: normalize_name(&g.name),
                name: if g.display_name.is_empty() {
                    g.name.clone()
                } else {
                    g.display_name.clone()
                },
                pid: g.pid,
                cpu_w,
                gpu_w,
                disk_w,
                cpu,
                gpu,
                disk_bps: g.disk_bps,
            }
        })
        .collect();

    let has_sensors =
        sensors.cpu_package_w.is_some_and(|w| w > 0.0) || sensors.gpu_w.is_some_and(|w| w > 0.0);
    let Some(draw) = sensors.system_draw_w.filter(|d| *d > 0.0) else {
        return (apps, if has_sensors { "sensors" } else { "model" });
    };
    let total: f32 = apps.iter().map(AppEstimate::watts).sum();
    if total > 0.0 {
        let scale = draw * APP_SHARE_OF_DRAIN / total;
        // Sensor readings are real, so the battery may only pull them down.
        let scale = if has_sensors { scale.min(1.0) } else { scale };
        for a in &mut apps {
            a.cpu_w *= scale;
            a.gpu_w *= scale;
            a.disk_w *= scale;
        }
    }
    (apps, "calibrated")
}

pub fn source_label(source: &str) -> &'static str {
    match source {
        "calibrated" => "Estimated from CPU, GPU and disk activity, scaled to the battery's measured drain.",
        "sensors" => "Estimated from CPU and GPU power sensors, split by each app's share of the work.",
        _ => "Estimated from CPU, GPU and disk activity. Windows does not give apps its own per-app energy numbers.",
    }
}

/// Plain-language headline for the per-app list.
pub fn power_headline(battery: &BatteryStatus, apps: &[AppPower]) -> (String, String) {
    let total: f32 = apps.iter().map(|a| a.watts).sum();
    let top = match apps.first() {
        Some(top) if total >= 1.0 && top.watts >= 0.5 => top,
        _ => {
            let headline = if battery.state == "discharging" {
                "Nothing is draining your battery hard"
            } else {
                "Nothing is burning much power"
            };
            return (
                headline.into(),
                "Apps are mostly idle. The screen and the rest of the system use most of the power right now.".into(),
            );
        }
    };
    let dominant = top.share >= 0.35;
    let headline = match (battery.state.as_str(), dominant) {
        ("discharging", true) => format!("{} is the reason your battery drains", top.name),
        ("discharging", false) => "Battery drain is spread across a few apps".into(),
        (_, true) if battery.present => {
            format!("{} is the hungriest app, but you are plugged in", top.name)
        }
        (_, true) => format!("{} is using the most power", top.name),
        _ => "Power use is spread across a few apps".into(),
    };
    let cost = top
        .battery_percent_per_hour
        .filter(|p| *p >= 1.0 && battery.state == "discharging")
        .map(|p| format!(" That is roughly {p:.0}% of your battery per hour."))
        .unwrap_or_default();
    let summary = format!(
        "{} accounts for about {:.0}% of estimated app power (~{:.1} W), mostly {}.{cost}",
        top.name,
        top.share * 100.0,
        top.watts,
        top.main_component.to_lowercase()
    );
    (headline, summary)
}

/// Smooths per-app estimates across ticks so the ranking does not flicker.
#[derive(Default)]
pub struct PowerSmoother {
    state: HashMap<String, AppEstimate>,
}

impl PowerSmoother {
    pub fn push(&mut self, apps: Vec<AppEstimate>) -> Vec<AppEstimate> {
        let mix = |old: f32, new: f32| old + (new - old) * SMOOTHING;
        let mut seen = HashSet::new();
        for app in apps {
            seen.insert(app.key.clone());
            match self.state.get_mut(&app.key) {
                Some(cur) => {
                    cur.cpu_w = mix(cur.cpu_w, app.cpu_w);
                    cur.gpu_w = mix(cur.gpu_w, app.gpu_w);
                    cur.disk_w = mix(cur.disk_w, app.disk_w);
                    cur.cpu = app.cpu;
                    cur.gpu = app.gpu;
                    cur.disk_bps = app.disk_bps;
                    cur.name = app.name;
                    cur.pid = app.pid;
                }
                None => {
                    self.state.insert(app.key.clone(), app);
                }
            }
        }
        self.state.retain(|key, cur| {
            if seen.contains(key) {
                return true;
            }
            cur.cpu_w = mix(cur.cpu_w, 0.0);
            cur.gpu_w = mix(cur.gpu_w, 0.0);
            cur.disk_w = mix(cur.disk_w, 0.0);
            cur.cpu = 0.0;
            cur.gpu = 0.0;
            cur.disk_bps = 0;
            cur.watts() > 0.01
        });
        self.state.values().cloned().collect()
    }
}

pub fn build_report(
    battery: BatteryStatus,
    mut apps: Vec<AppEstimate>,
    source: &str,
    system_draw_w: Option<f32>,
) -> PowerReport {
    apps.sort_by(|a, b| {
        b.watts()
            .total_cmp(&a.watts())
            .then_with(|| a.key.cmp(&b.key))
    });
    let total: f32 = apps.iter().map(AppEstimate::watts).sum();
    let rows: Vec<AppPower> = apps
        .iter()
        .filter(|a| a.watts() >= 0.01)
        .take(MAX_APPS)
        .map(|a| {
            let watts = a.watts();
            let main = if a.gpu_w >= a.cpu_w && a.gpu_w >= a.disk_w {
                "GPU"
            } else if a.disk_w > a.cpu_w {
                "Disk"
            } else {
                "CPU"
            };
            AppPower {
                key: a.key.clone(),
                name: a.name.clone(),
                pid: a.pid,
                watts,
                share: if total > 0.0 { watts / total } else { 0.0 },
                cpu_w: a.cpu_w,
                gpu_w: a.gpu_w,
                disk_w: a.disk_w,
                cpu: a.cpu,
                gpu: a.gpu,
                disk_bps: a.disk_bps,
                main_component: main.into(),
                battery_percent_per_hour: battery
                    .full_wh
                    .filter(|f| *f > 0.0)
                    .map(|full| watts / full * 100.0),
            }
        })
        .collect();
    let (headline, summary) = power_headline(&battery, &rows);
    PowerReport {
        other_w: system_draw_w.map(|d| (d - total).max(0.0)),
        battery,
        apps: rows,
        headline,
        summary,
        source: source.into(),
        source_label: source_label(source).into(),
        estimated_app_w: total,
        system_draw_w,
        updated_at: chrono::Utc::now().timestamp_millis(),
    }
}

/// Owns the battery polling thread and the per-app smoother.
pub struct PowerTracker {
    rx: Receiver<RawBattery>,
    raw: RawBattery,
    smoother: PowerSmoother,
    model: PowerModel,
    report: PowerReport,
}

impl PowerTracker {
    pub fn new() -> Self {
        Self {
            rx: start_battery_thread(),
            raw: RawBattery::default(),
            smoother: PowerSmoother::default(),
            model: PowerModel::default(),
            report: build_report(
                battery_status(&RawBattery::default()),
                Vec::new(),
                "model",
                None,
            ),
        }
    }

    pub fn update(&mut self, processes: &[ProcessRow], cpu: &CpuMetrics, gpu: &GpuMetrics) {
        while let Ok(raw) = self.rx.try_recv() {
            self.raw = raw;
        }
        let battery = battery_status(&self.raw);
        let system_draw_w = battery.rate_w.filter(|_| battery.state == "discharging");
        let sensors = Sensors {
            cpu_total_percent: cpu.utilization,
            cpu_package_w: cpu.package_power_w,
            gpu_w: gpu.power_w,
            system_draw_w,
        };
        let (apps, source) = estimate_apps(processes, &sensors, &self.model);
        let smoothed = self.smoother.push(apps);
        self.report = build_report(battery, smoothed, source, system_draw_w);
    }

    pub fn report(&self) -> PowerReport {
        self.report.clone()
    }
}

/// `WTF_MOCK_BATTERY` swaps the real battery for a fixture so the UI can be
/// checked on machines without one: `discharging`, `charging`, `none`, or a
/// path to a JSON file shaped like [`RawBattery`].
pub fn mock_battery(spec: &str) -> Option<RawBattery> {
    let pack = RawBattery {
        present: true,
        full_mwh: Some(50_000),
        design_mwh: Some(57_000),
        cycle_count: Some(312),
        name: Some("Mock Li-ion pack".into()),
        manufacturer: Some("Fixture Co".into()),
        ..Default::default()
    };
    let mut raw = match spec.trim() {
        "" => return None,
        "none" | "desktop" => RawBattery::default(),
        "discharging" => RawBattery {
            ac_online: Some(false),
            charging: Some(false),
            percent: Some(62.0),
            remaining_mwh: Some(31_000),
            discharge_rate_mw: Some(14_200),
            ..pack
        },
        "charging" => RawBattery {
            ac_online: Some(true),
            charging: Some(true),
            percent: Some(41.0),
            remaining_mwh: Some(20_500),
            charge_rate_mw: Some(45_000),
            ..pack
        },
        path => match std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|s| serde_json::from_str::<RawBattery>(&s).map_err(|e| e.to_string()))
        {
            Ok(raw) => raw,
            Err(err) => {
                tracing::warn!("WTF_MOCK_BATTERY fixture {path} unreadable: {err}");
                return None;
            }
        },
    };
    raw.mocked = true;
    Some(raw)
}

fn start_battery_thread() -> Receiver<RawBattery> {
    let (tx, rx) = mpsc::channel();
    if let Some(raw) = std::env::var("WTF_MOCK_BATTERY")
        .ok()
        .and_then(|s| mock_battery(&s))
    {
        tracing::info!("Using mocked battery data from WTF_MOCK_BATTERY");
        let _ = tx.send(raw);
        return rx;
    }
    thread::Builder::new()
        .name("battery".into())
        .spawn(move || {
            let wmi_root = wmi::COMLibrary::new()
                .ok()
                .and_then(|com| wmi::WMIConnection::with_namespace_path("root\\wmi", com).ok());
            loop {
                if tx.send(read_battery(wmi_root.as_ref())).is_err() {
                    break;
                }
                thread::sleep(Duration::from_secs(5));
            }
        })
        .ok();
    rx
}

fn read_battery(wmi_root: Option<&wmi::WMIConnection>) -> RawBattery {
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut status = SYSTEM_POWER_STATUS::default();
    let raw = if unsafe { GetSystemPowerStatus(&mut status) }.is_ok() {
        from_power_status(
            status.ACLineStatus,
            status.BatteryFlag,
            status.BatteryLifePercent,
            status.BatteryLifeTime,
        )
    } else {
        RawBattery::default()
    };
    let Some(wmi) = wmi_root else {
        return raw;
    };
    let status: Vec<WmiBatteryStatus> = wmi.query().unwrap_or_default();
    if !raw.present && status.is_empty() {
        return raw;
    }
    let full: Vec<WmiFullCapacity> = wmi.query().unwrap_or_default();
    let statics: Vec<WmiStaticData> = wmi.query().unwrap_or_default();
    let cycles: Vec<WmiCycleCount> = wmi.query().unwrap_or_default();
    merge_wmi(raw, &status, &full, &statics, &cycles)
}

#[cfg(test)]
#[path = "power_tests.rs"]
mod tests;
