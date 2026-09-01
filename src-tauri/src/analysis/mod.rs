mod knowledge;

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;

use crate::types::AppSettings;
use crate::types::{
    ActivityEvent, CpuMetrics, DiskMetrics, GpuMetrics, HealthSummary, Interpretation,
    MemoryMetrics, NetworkMetrics, ProcessRow, ResourcePressure,
};
use knowledge::{classify_path, hint_for, is_game, is_known_benign};

pub use knowledge::display_name;

const WINDOW: usize = 900;
const LIVE_LINGER_SECS: u64 = 75;
const SPIKE_LINGER_SECS: u64 = 45;

#[derive(Debug, Clone, Copy)]
pub struct RawView<'a> {
    pub cpu: &'a CpuMetrics,
    pub gpu: &'a GpuMetrics,
    pub memory: &'a MemoryMetrics,
    pub disks: &'a [DiskMetrics],
    pub network: &'a NetworkMetrics,
    pub processes: &'a [ProcessRow],
}

#[derive(Debug, Clone)]
struct Stretch {
    since: Instant,
    peak: f32,
    process: Option<String>,
    alerted: bool,
}

#[derive(Debug)]
pub struct AnalysisEngine {
    cpu: VecDeque<f32>,
    gpu: VecDeque<f32>,
    memory: VecDeque<f32>,
    disk: VecDeque<f32>,
    net_down: VecDeque<f32>,
    net_up: VecDeque<f32>,
    cpu_stretch: Option<Stretch>,
    gpu_stretch: Option<Stretch>,
    ram_stretch: Option<Stretch>,
    disk_io_stretch: Option<Stretch>,
    capacity_alerted: HashSet<String>,
    capacity_seeded: HashSet<String>,
    last_event_keys: HashMap<String, Instant>,
    lingering: HashMap<String, LingerEntry>,
    last_cpu: f32,
    last_gpu: f32,
    samples: u32,
}

#[derive(Debug, Clone)]
struct LingerEntry {
    event: ActivityEvent,
    last_active: Instant,
    hold_secs: u64,
}

#[derive(Debug, Clone)]
pub struct AnalysisOutput {
    pub interpretation: Interpretation,
    pub health: HealthSummary,
    pub events: Vec<ActivityEvent>,
    pub persist_events: Vec<ActivityEvent>,
    pub new_alerts: Vec<AlertSeed>,
}

#[derive(Debug, Clone)]
pub struct AlertSeed {
    pub fingerprint: String,
    pub severity: String,
    pub category: String,
    pub title: String,
    pub body: String,
    pub notify: bool,
}

impl Default for AnalysisEngine {
    fn default() -> Self {
        Self {
            cpu: VecDeque::with_capacity(WINDOW),
            gpu: VecDeque::with_capacity(WINDOW),
            memory: VecDeque::with_capacity(WINDOW),
            disk: VecDeque::with_capacity(WINDOW),
            net_down: VecDeque::with_capacity(WINDOW),
            net_up: VecDeque::with_capacity(WINDOW),
            cpu_stretch: None,
            gpu_stretch: None,
            ram_stretch: None,
            disk_io_stretch: None,
            capacity_alerted: HashSet::new(),
            capacity_seeded: HashSet::new(),
            last_event_keys: HashMap::new(),
            lingering: HashMap::new(),
            last_cpu: 0.0,
            last_gpu: 0.0,
            samples: 0,
        }
    }
}

impl AnalysisEngine {
    pub fn push_and_analyze(&mut self, view: RawView<'_>, settings: &AppSettings) -> AnalysisOutput {
        let gpu_util = view.gpu.utilization.unwrap_or(0.0);
        let disk_util = disk_busy(view.disks);
        push_capped(&mut self.cpu, view.cpu.utilization, WINDOW);
        push_capped(&mut self.gpu, gpu_util, WINDOW);
        push_capped(&mut self.memory, view.memory.utilization, WINDOW);
        push_capped(&mut self.disk, disk_util, WINDOW);
        push_capped(
            &mut self.net_down,
            (view.network.download_bps as f32) / (1024.0 * 1024.0),
            WINDOW,
        );
        push_capped(
            &mut self.net_up,
            (view.network.upload_bps as f32) / (1024.0 * 1024.0),
            WINDOW,
        );

        let now = Instant::now();
        let groups = group_processes(view.processes);
        let top_cpu_name = top_cpu(&groups).map(|g| g.name.clone());
        let top_mem_name = top_memory(&groups).map(|g| g.name.clone());
        let top_gpu_name = top_gpu(view.processes).map(|p| process_label(&p));

        let mut persist = Vec::new();
        update_stretch(
            &mut self.cpu_stretch,
            view.cpu.utilization >= settings.cpu_alert_percent,
            view.cpu.utilization,
            settings.cpu_alert_percent - 8.0,
            top_cpu_name.clone(),
            now,
            "cpu",
            &mut persist,
        );
        update_stretch(
            &mut self.gpu_stretch,
            view.gpu
                .temperature_c
                .map(|t| t >= settings.gpu_temp_alert_c)
                .unwrap_or(false),
            view.gpu.temperature_c.unwrap_or(0.0),
            settings.gpu_temp_alert_c - 5.0,
            top_gpu_name.clone(),
            now,
            "thermal",
            &mut persist,
        );
        update_stretch(
            &mut self.ram_stretch,
            view.memory.utilization >= settings.ram_alert_percent,
            view.memory.utilization,
            settings.ram_alert_percent - 6.0,
            top_mem_name.clone(),
            now,
            "memory",
            &mut persist,
        );
        update_stretch(
            &mut self.disk_io_stretch,
            disk_util >= 75.0,
            disk_util,
            55.0,
            top_disk(view.processes).map(|p| process_label(&p)),
            now,
            "disk",
            &mut persist,
        );

        let cpu_delta = view.cpu.utilization - self.last_cpu;
        let gpu_delta = gpu_util - self.last_gpu;
        self.last_cpu = view.cpu.utilization;
        self.last_gpu = gpu_util;

        let (events, mut event_persist) =
            self.build_events(view, &groups, cpu_delta, gpu_delta, disk_util, now);
        persist.append(&mut event_persist);

        for disk in view.disks {
            if disk.removable {
                continue;
            }
            let key = format!("disk-full-{}", disk.mount);
            let full = disk.utilization * 100.0 >= settings.disk_full_percent;
            if full {
                self.capacity_alerted.insert(key);
            } else if disk.utilization * 100.0 < settings.disk_full_percent - 3.0 {
                self.capacity_seeded.remove(&key);
                if self.capacity_alerted.remove(&key) {
                    persist.push(resolution_event(
                        "storage",
                        format!(
                            "{} is no longer critically full. It peaked at {:.0}% used.",
                            disk.mount,
                            disk.utilization * 100.0
                        ),
                        None,
                    ));
                }
            }
        }

        let events = self.linger_events(events, &persist, now);
        let interpretation = interpret(view, &groups, disk_util, &self.cpu_stretch, now);
        let health = health_of(view, settings, disk_util, &self.cpu_stretch, now);
        let new_alerts = self.build_alerts(view, settings, now);

        AnalysisOutput {
            interpretation,
            health,
            events,
            persist_events: persist,
            new_alerts,
        }
    }

    pub fn sparkline_cpu(&self) -> Vec<f32> {
        self.cpu.iter().copied().collect()
    }
    pub fn sparkline_gpu(&self) -> Vec<f32> {
        self.gpu.iter().copied().collect()
    }
    pub fn sparkline_memory(&self) -> Vec<f32> {
        self.memory.iter().copied().collect()
    }
    pub fn sparkline_disk(&self) -> Vec<f32> {
        self.disk.iter().copied().collect()
    }
    pub fn sparkline_net_down(&self) -> Vec<f32> {
        self.net_down.iter().copied().collect()
    }
    pub fn sparkline_net_up(&self) -> Vec<f32> {
        self.net_up.iter().copied().collect()
    }

    fn build_events(
        &mut self,
        view: RawView<'_>,
        groups: &HashMap<String, Group>,
        cpu_delta: f32,
        gpu_delta: f32,
        disk_util: f32,
        now: Instant,
    ) -> (Vec<ActivityEvent>, Vec<ActivityEvent>) {
        let mut live = Vec::new();
        let mut notable: Vec<ActivityEvent> = Vec::new();
        let ts = chrono::Utc::now().timestamp();
        let has_history = self.samples > 0;
        self.samples = self.samples.saturating_add(1);

        let fullest = fullest_fixed_disk(view.disks);
        if let Some(disk) = fullest {
            if disk.utilization >= 0.88 {
                let free = crate::format_bytes(disk.available_bytes);
                live.push(ActivityEvent {
                    id: format!("storage-{}", disk.mount),
                    timestamp: ts,
                    title: format!("{} is running out of space", disk.mount.trim_end_matches('\\')),
                    body: format!(
                        "{} is {:.0}% full. {} remains free.",
                        disk.mount,
                        disk.utilization * 100.0,
                        free
                    ),
                    why: Some(
                        "Large games, video, installers, and backups fill a system drive over time. This is capacity, not disk speed.".into(),
                    ),
                    impact: if disk.utilization >= 0.97 {
                        "High".into()
                    } else {
                        "Moderate".into()
                    },
                    recommendation: Some(
                        "Move large files off C:, empty Recycle Bin, or use Storage Sense. Leaving a little free space keeps Windows and updates from stalling.".into(),
                    ),
                    category: "storage".into(),
                    kind: "capacity".into(),
                    process: None,
                    severity: if disk.utilization >= 0.97 {
                        "warning".into()
                    } else {
                        "notice".into()
                    },
                });
            }
        }

        if let Some(top) = top_memory(groups) {
            if view.memory.utilization >= 85.0 || top.memory_bytes >= 1_500_000_000 && view.memory.utilization >= 78.0 {
                let remain = crate::format_bytes(view.memory.available_bytes);
                let secs = self
                    .ram_stretch
                    .as_ref()
                    .map(|s| now.saturating_duration_since(s.since).as_secs())
                    .unwrap_or(0);
                live.push(ActivityEvent {
                    id: "ram-now".into(),
                    timestamp: ts,
                    title: if view.memory.utilization >= 93.0 {
                        "RAM is under real pressure".into()
                    } else {
                        "Memory is the heavy item right now".into()
                    },
                    body: format!(
                        "System RAM is at {:.0}%{}. {} remains free. {} is using {} across {} processes.",
                        view.memory.utilization,
                        if secs >= 60 {
                            format!(" and has been elevated for {}.", format_duration(secs))
                        } else {
                            String::new()
                        },
                        remain,
                        top.name,
                        crate::format_bytes(top.memory_bytes),
                        top.count
                    ),
                    why: why_text(&top.name),
                    impact: if view.memory.utilization >= 93.0 {
                        "High".into()
                    } else {
                        "Moderate".into()
                    },
                    recommendation: Some(
                        "Closing unused windows in that app usually frees the most memory.".into(),
                    ),
                    category: "memory".into(),
                    kind: "current".into(),
                    process: Some(top.name.clone()),
                    severity: if view.memory.utilization >= 93.0 {
                        "warning".into()
                    } else {
                        "notice".into()
                    },
                });
            }
        }

        let cpu_ranked = top_n_cpu(groups, 3);
        if let Some(top) = cpu_ranked.first() {
            let gaming = is_game(&top.name);
            let cpu_secs = self
                .cpu_stretch
                .as_ref()
                .map(|s| now.saturating_duration_since(s.since).as_secs())
                .unwrap_or(0);
            if view.cpu.utilization >= 70.0 {
                let title = if gaming {
                    "A game is using the CPU. That is expected.".into()
                } else if cpu_secs >= 60 {
                    "CPU has been working hard for a while".into()
                } else {
                    "CPU is working pretty hard".into()
                };
                live.push(ActivityEvent {
                    id: "cpu-high".into(),
                    timestamp: ts,
                    title,
                    body: format!(
                        "CPU is at {:.0}%. {} is the main consumer at {:.1}%.{}",
                        view.cpu.utilization,
                        top.name,
                        top.cpu,
                        if cpu_secs >= 30 {
                            format!(" This has lasted {}.", format_duration(cpu_secs))
                        } else {
                            " Short spikes like this are common.".into()
                        }
                    ),
                    why: why_text(&top.name),
                    impact: if gaming {
                        "Low".into()
                    } else {
                        impact(view.cpu.utilization)
                    },
                    recommendation: Some(cpu_recommendation(&top.name, view.cpu.utilization, gaming, view.cpu.temperature_c)),
                    category: "cpu".into(),
                    kind: if cpu_secs >= 60 {
                        "sustained".into()
                    } else {
                        "high-resource".into()
                    },
                    process: Some(top.name.clone()),
                    severity: if !gaming && view.cpu.utilization >= 95.0 && cpu_secs >= 60 {
                        "warning".into()
                    } else {
                        "notice".into()
                    },
                });
            } else if has_history && cpu_delta >= 35.0 && view.cpu.utilization >= 40.0 {
                live.push(ActivityEvent {
                    id: "cpu-spike".into(),
                    timestamp: ts,
                    title: "CPU just jumped".into(),
                    body: format!(
                        "Utilization jumped by {:.0} points. {} is currently using {:.1}% CPU.",
                        cpu_delta, top.name, top.cpu
                    ),
                    why: why_text(&top.name),
                    impact: "Low".into(),
                    recommendation: Some(
                        "If this lasts more than a minute, open that process. One spike is normal.".into(),
                    ),
                    category: "cpu".into(),
                    kind: "temporary-spike".into(),
                    process: Some(top.name.clone()),
                    severity: "info".into(),
                });
            }
        }

        let gpu_util = view.gpu.utilization.unwrap_or(0.0);
        if gpu_util >= 40.0 {
            let top = top_gpu(view.processes);
            let gaming = top
                .as_ref()
                .map(|p| is_game(&p.name))
                .unwrap_or(false);
            live.push(ActivityEvent {
                id: "gpu-now".into(),
                timestamp: ts,
                title: if gaming {
                    "The GPU is doing game work".into()
                } else if gpu_util >= 80.0 {
                    "GPU is being used heavily".into()
                } else {
                    "Your GPU is busy".into()
                },
                body: match &top {
                    Some(p) => format!(
                        "GPU utilization is {:.0}%{}. {} is the dominant GPU process.",
                        gpu_util,
                        view.gpu
                            .temperature_c
                            .map(|t| format!(" at {t:.0}°C"))
                            .unwrap_or_default(),
                        process_label(p)
                    ),
                    None => format!(
                        "GPU utilization is {:.0}% on {}.",
                        gpu_util,
                        if view.gpu.name.is_empty() {
                            "this system"
                        } else {
                            &view.gpu.name
                        }
                    ),
                },
                why: top.as_ref().and_then(|p| why_text(&p.name)),
                impact: if gaming { "Low".into() } else { impact(gpu_util) },
                recommendation: if gaming {
                    Some("High GPU load while playing is the card doing its job.".into())
                } else {
                    None
                },
                category: "gpu".into(),
                kind: if has_history && gpu_delta >= 40.0 {
                    "temporary-spike".into()
                } else {
                    "current".into()
                },
                process: top.map(|p| process_label(&p)),
                severity: if !gaming && gpu_util >= 80.0 {
                    "notice".into()
                } else {
                    "info".into()
                },
            });
        }

        if let Some(top) = top_disk(view.processes) {
            if disk_util >= 40.0 || top.disk_bps > 8_000_000 {
                let share = if disk_total_bps(view.processes) > 0 {
                    (top.disk_bps as f64 / disk_total_bps(view.processes) as f64) * 100.0
                } else {
                    0.0
                };
                let indexer = normalize_name(&top.name).contains("searchindexer");
                live.push(ActivityEvent {
                    id: "disk-now".into(),
                    timestamp: ts,
                    title: if indexer {
                        "Windows Search is hitting the disk".into()
                    } else {
                        "Something is hitting your disk".into()
                    },
                    body: format!(
                        "{} is currently responsible for ~{:.0}% of disk activity ({}/s). This is transfer rate, not how full the drive is.",
                        process_label(&top),
                        share,
                        crate::format_bytes(top.disk_bps)
                    ),
                    why: why_text(&top.name),
                    impact: if indexer { "Low".into() } else { impact(disk_util) },
                    recommendation: Some(if indexer {
                        "No action needed. Windows Search is indexing recently changed files.".into()
                    } else {
                        "If this continues, that process is likely reading or writing a large set of files.".into()
                    }),
                    category: "disk".into(),
                    kind: if indexer { "normal".into() } else { "current".into() },
                    process: Some(process_label(&top)),
                    severity: if indexer { "info".into() } else { "notice".into() },
                });
            }
        }

        if view.network.upload_bps > 2_000_000 || view.network.download_bps > 8_000_000 {
            if let Some(top) = top_connections(view.processes) {
                let up = view.network.upload_bps > view.network.download_bps;
                live.push(ActivityEvent {
                    id: if up { "net-up" } else { "net-down" }.into(),
                    timestamp: ts,
                    title: if up {
                        "Large upload in progress".into()
                    } else {
                        "Large download in progress".into()
                    },
                    body: format!(
                        "Network is {} down and {} up. {} currently holds the most active connections. A large transfer is not evidence of malware.",
                        crate::format_bytes(view.network.download_bps),
                        crate::format_bytes(view.network.upload_bps),
                        process_label(&top)
                    ),
                    why: why_text(&top.name),
                    impact: "Low".into(),
                    recommendation: Some(
                        "This is often cloud sync, game updates, or a backup. Confirm the process if you were not expecting a transfer.".into(),
                    ),
                    category: "network".into(),
                    kind: "high-resource".into(),
                    process: Some(process_label(&top)),
                    severity: "info".into(),
                });
            }
        }

        for proc in view.processes.iter().take(40) {
            if let Some(path) = &proc.path {
                let class = classify_path(path, &proc.name);
                if class.suspicious && proc.cpu > 4.0 {
                    live.push(ActivityEvent {
                        id: format!("sus-{}", proc.pid),
                        timestamp: ts,
                        title: "Unexpected process activity".into(),
                        body: format!(
                            "{} looks like a system process but is running from an unusual location: {}",
                            process_label(proc),
                            path
                        ),
                        why: Some(
                            "High-resource usage alone is not suspicious. A system-looking name outside a Windows folder is.".into(),
                        ),
                        impact: "High".into(),
                        recommendation: Some(
                            "Inspect the file location and publisher before ending the process.".into(),
                        ),
                        category: "security".into(),
                        kind: "suspicious".into(),
                        process: Some(process_label(proc)),
                        severity: "warning".into(),
                    });
                }
            }
        }

        if live.is_empty() {
            if truly_idle(view, disk_util) {
                live.push(ActivityEvent {
                    id: "idle".into(),
                    timestamp: ts,
                    title: "Nothing interesting is happening right now.".into(),
                    body: "CPU, GPU, memory, disk activity, and drive capacity are all in a quiet range.".into(),
                    why: None,
                    impact: "None".into(),
                    recommendation: None,
                    category: "system".into(),
                    kind: "normal".into(),
                    process: None,
                    severity: "info".into(),
                });
            } else if let Some(top) = top_cpu(groups).or_else(|| top_memory(groups)) {
                live.push(ActivityEvent {
                    id: "right-now".into(),
                    timestamp: ts,
                    title: "Here is who is using the machine".into(),
                    body: format!(
                        "CPU is at {:.0}% and RAM is at {:.0}%. {} is the heaviest thing running ({:.1}% CPU, {}).",
                        view.cpu.utilization,
                        view.memory.utilization,
                        top.name,
                        top.cpu,
                        crate::format_bytes(top.memory_bytes)
                    ),
                    why: why_text(&top.name),
                    impact: "Low".into(),
                    recommendation: None,
                    category: "system".into(),
                    kind: "current".into(),
                    process: Some(top.name.clone()),
                    severity: "info".into(),
                });
            }
        }

        live.truncate(5);

        for event in &live {
            if !should_persist(event) {
                continue;
            }
            let key = event.id.clone();
            let stored = ActivityEvent {
                id: format!("{}-{ts}", event.id),
                ..event.clone()
            };
            maybe_push(
                &mut self.last_event_keys,
                &mut notable,
                now,
                &key,
                stored,
                persist_cooldown(event),
            );
        }

        (live, notable)
    }

    fn linger_events(
        &mut self,
        events: Vec<ActivityEvent>,
        persist: &[ActivityEvent],
        now: Instant,
    ) -> Vec<ActivityEvent> {
        for ev in persist {
            if ev.kind == "resolved" {
                self.lingering
                    .retain(|_, entry| entry.event.category != ev.category);
            }
        }

        let mut fillers = Vec::new();
        let mut fresh = Vec::new();
        for ev in events {
            if ev.id == "idle" || ev.id == "right-now" {
                fillers.push(ev);
            } else {
                fresh.push(ev);
            }
        }

        for ev in &fresh {
            self.lingering.insert(
                ev.id.clone(),
                LingerEntry {
                    event: ev.clone(),
                    last_active: now,
                    hold_secs: linger_hold(ev),
                },
            );
        }

        let present: HashSet<String> = fresh.iter().map(|e| e.id.clone()).collect();
        let mut held = Vec::new();
        self.lingering.retain(|id, entry| {
            if present.contains(id) {
                return true;
            }
            if now.saturating_duration_since(entry.last_active).as_secs() < entry.hold_secs {
                held.push(entry.event.clone());
                true
            } else {
                false
            }
        });

        fresh.extend(held);
        if fresh.is_empty() {
            fresh = fillers;
        }
        fresh.truncate(5);
        fresh
    }

    fn build_alerts(
        &mut self,
        view: RawView<'_>,
        settings: &AppSettings,
        now: Instant,
    ) -> Vec<AlertSeed> {
        let mut seeds = Vec::new();
        if let Some(stretch) = &mut self.cpu_stretch {
            let secs = now.saturating_duration_since(stretch.since).as_secs();
            if secs >= settings.cpu_alert_seconds && !stretch.alerted {
                stretch.alerted = true;
                let gaming = stretch
                    .process
                    .as_deref()
                    .map(is_game)
                    .unwrap_or(false);
                if !gaming {
                    seeds.push(AlertSeed {
                        fingerprint: "cpu-sustained".into(),
                        severity: "warning".into(),
                        category: "cpu".into(),
                        title: "High CPU usage".into(),
                        body: format!(
                            "CPU usage has remained above {:.0}% for {}. Peak: {:.0}%. {}",
                            settings.cpu_alert_percent,
                            format_duration(secs),
                            stretch.peak,
                            stretch
                                .process
                                .as_deref()
                                .map(|p| format!("{p} was the main consumer."))
                                .unwrap_or_default()
                        ),
                        notify: true,
                    });
                }
            }
        }
        if let Some(stretch) = &mut self.gpu_stretch {
            let secs = now.saturating_duration_since(stretch.since).as_secs();
            if secs >= settings.gpu_temp_alert_seconds && !stretch.alerted {
                stretch.alerted = true;
                seeds.push(AlertSeed {
                    fingerprint: "gpu-thermal".into(),
                    severity: "critical".into(),
                    category: "thermal".into(),
                    title: "Thermal warning".into(),
                    body: format!(
                        "GPU temperature has remained above {:.0}°C for {}. Peak: {:.0}°C.",
                        settings.gpu_temp_alert_c,
                        format_duration(secs),
                        stretch.peak
                    ),
                    notify: true,
                });
            }
        }
        if let Some(stretch) = &mut self.ram_stretch {
            let secs = now.saturating_duration_since(stretch.since).as_secs();
            if secs >= settings.ram_alert_seconds && !stretch.alerted {
                stretch.alerted = true;
                seeds.push(AlertSeed {
                    fingerprint: "ram-sustained".into(),
                    severity: "warning".into(),
                    category: "memory".into(),
                    title: "Memory pressure is high".into(),
                    body: format!(
                        "RAM has stayed above {:.0}% for {}. Peak: {:.0}%. {} remains available.",
                        settings.ram_alert_percent,
                        format_duration(secs),
                        stretch.peak,
                        crate::format_bytes(view.memory.available_bytes)
                    ),
                    notify: true,
                });
            }
        }
        for disk in view.disks {
            if disk.utilization * 100.0 >= settings.disk_full_percent && !disk.removable {
                let key = format!("disk-full-{}", disk.mount);
                if self.capacity_seeded.insert(key.clone()) {
                    seeds.push(AlertSeed {
                        fingerprint: key,
                        severity: if disk.utilization >= 0.97 {
                            "critical".into()
                        } else {
                            "warning".into()
                        },
                        category: "storage".into(),
                        title: format!("{} is running out of space", disk.mount.trim_end_matches('\\')),
                        body: format!(
                            "{} is {:.0}% full ({} free). This is drive capacity, not disk speed.",
                            disk.mount,
                            disk.utilization * 100.0,
                            crate::format_bytes(disk.available_bytes)
                        ),
                        notify: true,
                    });
                }
            }
        }
        seeds
    }
}

#[derive(Clone)]
struct Group {
    name: String,
    cpu: f32,
    memory_bytes: u64,
    disk_bps: u64,
    count: usize,
}

fn group_processes(rows: &[ProcessRow]) -> HashMap<String, Group> {
    let mut map: HashMap<String, Group> = HashMap::new();
    for row in rows {
        let key = normalize_name(&row.name);
        let shown = process_label(row);
        let entry = map.entry(key).or_insert(Group {
            name: shown.clone(),
            cpu: 0.0,
            memory_bytes: 0,
            disk_bps: 0,
            count: 0,
        });
        entry.cpu += row.cpu;
        entry.memory_bytes += row.memory_bytes;
        entry.disk_bps += row.disk_bps;
        entry.count += 1;
        if shown != entry.name && shown.len() > entry.name.len() {
            entry.name = shown;
        }
    }
    map
}

pub fn grouped_process_rows(rows: &[ProcessRow]) -> Vec<ProcessRow> {
    let mut buckets: HashMap<String, Vec<&ProcessRow>> = HashMap::new();
    for row in rows {
        buckets
            .entry(normalize_name(&row.name))
            .or_default()
            .push(row);
    }
    buckets
        .into_values()
        .map(|members| collapse_group(&members))
        .collect()
}

fn collapse_group(members: &[&ProcessRow]) -> ProcessRow {
    let pids: HashSet<u32> = members.iter().map(|p| p.pid).collect();
    let mut best = members
        .iter()
        .find(|p| p.parent_pid.map(|pp| !pids.contains(&pp)).unwrap_or(true))
        .or_else(|| members.iter().max_by_key(|p| p.memory_bytes))
        .copied()
        .expect("group is not empty")
        .clone();
    let count = members.len();
    let shown = members
        .iter()
        .map(|p| process_label(p))
        .max_by_key(|s| s.len())
        .unwrap_or_else(|| best.name.clone());
    best.cpu = members.iter().map(|p| p.cpu).sum();
    best.memory_bytes = members.iter().map(|p| p.memory_bytes).sum();
    best.disk_bps = members.iter().map(|p| p.disk_bps).sum();
    best.disk_read_bps = members.iter().map(|p| p.disk_read_bps).sum();
    best.disk_write_bps = members.iter().map(|p| p.disk_write_bps).sum();
    best.connection_count = members.iter().map(|p| p.connection_count).sum();
    best.gpu = members
        .iter()
        .filter_map(|p| p.gpu)
        .max_by(|a, b| a.total_cmp(b));
    best.gpu_memory_bytes = members.iter().filter_map(|p| p.gpu_memory_bytes).max();
    best.display_name = if count > 1 {
        format!("{shown} ({count})")
    } else {
        shown
    };
    best
}

fn process_label(row: &ProcessRow) -> String {
    if !row.display_name.is_empty() {
        row.display_name.clone()
    } else {
        display_name(&row.name)
    }
}

fn top_n_cpu(groups: &HashMap<String, Group>, n: usize) -> Vec<Group> {
    let mut ranked: Vec<Group> = groups.values().cloned().collect();
    ranked.sort_by(|a, b| b.cpu.total_cmp(&a.cpu));
    ranked.truncate(n);
    ranked
}

fn top_cpu(groups: &HashMap<String, Group>) -> Option<Group> {
    groups.values().cloned().max_by(|a, b| a.cpu.total_cmp(&b.cpu))
}
fn top_memory(groups: &HashMap<String, Group>) -> Option<Group> {
    groups
        .values()
        .cloned()
        .max_by_key(|g| g.memory_bytes)
}
fn top_disk(rows: &[ProcessRow]) -> Option<ProcessRow> {
    rows.iter().max_by_key(|p| p.disk_bps).cloned()
}
fn top_gpu(rows: &[ProcessRow]) -> Option<ProcessRow> {
    rows.iter()
        .filter(|p| p.gpu.unwrap_or(0.0) > 1.0 || p.gpu_memory_bytes.unwrap_or(0) > 64 * 1024 * 1024)
        .max_by(|a, b| {
            a.gpu
                .unwrap_or(0.0)
                .total_cmp(&b.gpu.unwrap_or(0.0))
        })
        .cloned()
}
fn top_connections(rows: &[ProcessRow]) -> Option<ProcessRow> {
    rows.iter().max_by_key(|p| p.connection_count).cloned()
}

fn fullest_fixed_disk(disks: &[DiskMetrics]) -> Option<&DiskMetrics> {
    disks
        .iter()
        .filter(|d| !d.removable)
        .max_by(|a, b| a.utilization.total_cmp(&b.utilization))
}

fn truly_idle(view: RawView<'_>, disk_util: f32) -> bool {
    let gpu = view.gpu.utilization.unwrap_or(0.0);
    let cap_ok = view
        .disks
        .iter()
        .filter(|d| !d.removable)
        .all(|d| d.utilization < 0.88);
    view.cpu.utilization < 25.0
        && gpu < 20.0
        && view.memory.utilization < 70.0
        && disk_util < 25.0
        && cap_ok
}

fn interpret(
    view: RawView<'_>,
    groups: &HashMap<String, Group>,
    disk_util: f32,
    cpu_stretch: &Option<Stretch>,
    now: Instant,
) -> Interpretation {
    let gpu = view.gpu.utilization.unwrap_or(0.0);
    let cpu = view.cpu.utilization;
    let ram = view.memory.utilization;
    let top_cpu = top_cpu(groups);
    let top_mem = top_memory(groups);
    let top_gpu = top_gpu(view.processes);
    let fullest = fullest_fixed_disk(view.disks);
    let cap = fullest.map(|d| d.utilization).unwrap_or(0.0);
    let gaming = top_gpu
        .as_ref()
        .map(|p| is_game(&p.name))
        .or_else(|| top_cpu.as_ref().map(|g| is_game(&g.name)))
        .unwrap_or(false);
    let cpu_secs = cpu_stretch
        .as_ref()
        .map(|s| now.saturating_duration_since(s.since).as_secs())
        .unwrap_or(0);
    let hot = view
        .cpu
        .temperature_c
        .into_iter()
        .chain(view.gpu.temperature_c)
        .fold(0.0_f32, f32::max);

    let (headline, summary, mood) = if cap >= 0.97 {
        (
            format!(
                "{} is nearly full.",
                fullest
                    .map(|d| d.mount.trim_end_matches('\\').to_string())
                    .unwrap_or_else(|| "Storage".into())
            ),
            format!(
                "{} is {:.0}% full with {} free. That will slow installs and updates even if CPU looks idle.",
                fullest.map(|d| d.mount.as_str()).unwrap_or("The system drive"),
                cap * 100.0,
                fullest
                    .map(|d| crate::format_bytes(d.available_bytes))
                    .unwrap_or_else(|| "little space".into())
            ),
            "tense".to_string(),
        )
    } else if cap >= 0.90 {
        (
            format!(
                "{} is running out of space.",
                fullest
                    .map(|d| d.mount.trim_end_matches('\\').to_string())
                    .unwrap_or_else(|| "Storage".into())
            ),
            format!(
                "The drive is {:.0}% full ({} free). CPU and RAM are separate from this. Free some space when you can.",
                cap * 100.0,
                fullest
                    .map(|d| crate::format_bytes(d.available_bytes))
                    .unwrap_or_else(|| "little space".into())
            ),
            "active".to_string(),
        )
    } else if hot >= 90.0 {
        (
            "The machine is running hot.".to_string(),
            format!(
                "Peak temperature is {hot:.0}°C. High load is one thing. Heat that stays up is another."
            ),
            "tense".to_string(),
        )
    } else if ram >= 93.0 {
        (
            "RAM is under real pressure.".to_string(),
            format!(
                "{:.1} GB remains available. {} is the largest consumer. Apps may hitch until something closes.",
                view.memory.available_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
                top_mem.as_ref().map(|p| p.name.as_str()).unwrap_or("An app")
            ),
            "tense".to_string(),
        )
    } else if ram >= 85.0 {
        (
            "RAM is getting a little crowded.".to_string(),
            format!(
                "Memory is at {:.0}% with {} free. {} is the largest consumer.",
                ram,
                crate::format_bytes(view.memory.available_bytes),
                top_mem.as_ref().map(|p| p.name.as_str()).unwrap_or("An app")
            ),
            if ram >= 90.0 {
                "tense".to_string()
            } else {
                "active".to_string()
            },
        )
    } else if gaming && gpu >= 50.0 {
        (
            "You are gaming. The PC is supposed to work this hard.".to_string(),
            format!(
                "{} is driving the GPU at {:.0}%. CPU at {:.0}% is expected while you play, not a fault.",
                top_gpu
                    .as_ref()
                    .map(|p| process_label(p))
                    .or_else(|| top_cpu.as_ref().map(|p| p.name.clone()))
                    .unwrap_or_else(|| "A game".into()),
                gpu,
                cpu
            ),
            "busy".to_string(),
        )
    } else if gpu >= 75.0 {
        (
            "GPU is working overtime.".to_string(),
            format!(
                "{} is currently driving the graphics load at {:.0}%.",
                top_gpu
                    .as_ref()
                    .map(|p| process_label(p))
                    .unwrap_or_else(|| "A process".into()),
                gpu
            ),
            "busy".to_string(),
        )
    } else if disk_util >= 75.0 {
        (
            "Your disk is busy with file activity.".to_string(),
            "Storage is transferring a lot of data. Other apps may feel sluggish until this settles. This is speed, not how full the drive is.".to_string(),
            "tense".to_string(),
        )
    } else if cpu >= 80.0 {
        let who = top_cpu
            .as_ref()
            .map(|p| p.name.as_str())
            .unwrap_or("A process");
        if cpu_secs >= 60 {
            (
                "Something has been cooking for a bit.".to_string(),
                format!(
                    "CPU is at {:.0}% and has been high for {}. {} is currently responsible for most of that.",
                    cpu,
                    format_duration(cpu_secs),
                    who
                ),
                "busy".to_string(),
            )
        } else {
            (
                "CPU is busy, briefly.".to_string(),
                format!(
                    "CPU is at {:.0}%. {} is currently responsible. High CPU for a few seconds is normal.",
                    cpu, who
                ),
                "busy".to_string(),
            )
        }
    } else if truly_idle(view, disk_util) {
        (
            "Your PC is chilling.".to_string(),
            "CPU, GPU, memory, disk activity, and drive capacity are all within quiet ranges.".to_string(),
            "calm".to_string(),
        )
    } else if ram >= 70.0 || cpu >= 35.0 || gpu >= 30.0 || cap >= 0.80 {
        (
            "Your PC is doing a few things at once.".to_string(),
            format!(
                "CPU {:.0}%, RAM {:.0}%, GPU {:.0}%. Nothing here is a crisis, but it is not idle either.",
                cpu, ram, gpu
            ),
            "active".to_string(),
        )
    } else {
        (
            "Your PC is mostly idle.".to_string(),
            format!(
                "Load is light. {} is the heaviest process, and it is not stressing the machine.",
                top_cpu
                    .as_ref()
                    .map(|p| p.name.as_str())
                    .unwrap_or("Background work")
            ),
            "calm".to_string(),
        )
    };

    Interpretation {
        headline,
        summary,
        mood,
        pressure: ResourcePressure {
            cpu: cpu_band(cpu, cpu_secs),
            gpu: if view.gpu.available {
                gpu_band(gpu)
            } else {
                "unknown".into()
            },
            memory: mem_band(ram),
            disk: disk_io_band(disk_util),
            network: net_band(view.network.download_bps + view.network.upload_bps),
            thermal: thermal_band(view.cpu.temperature_c, view.gpu.temperature_c),
        },
    }
}

fn health_of(
    view: RawView<'_>,
    settings: &AppSettings,
    disk_util: f32,
    cpu_high_since: &Option<Stretch>,
    now: Instant,
) -> HealthSummary {
    let mut score: i32 = 100;
    let mut reasons = Vec::new();
    let gpu = view.gpu.utilization.unwrap_or(0.0);
    let gaming = view
        .processes
        .iter()
        .any(|p| is_game(&p.name) && (p.gpu.unwrap_or(0.0) > 20.0 || p.cpu > 15.0))
        || (gpu >= 60.0
            && view
                .processes
                .iter()
                .any(|p| is_game(&p.name)));

    if view.cpu.utilization >= 90.0 {
        if gaming {
            score -= 3;
            reasons.push("CPU is busy because a game is running. That is expected.".into());
        } else {
            score -= 12;
            reasons.push("CPU is under heavy load.".into());
        }
    }
    if view.memory.utilization >= 92.0 {
        score -= 16;
        reasons.push(format!(
            "Memory pressure is high ({:.0}% used).",
            view.memory.utilization
        ));
    } else if view.memory.utilization >= 85.0 {
        score -= 8;
        reasons.push(format!(
            "Memory is getting tight ({:.0}% used).",
            view.memory.utilization
        ));
    }
    if let Some(temp) = view.gpu.temperature_c {
        if temp >= settings.gpu_temp_alert_c {
            score -= 18;
            reasons.push(format!("GPU is hot ({temp:.0}°C)."));
        }
    }
    if let Some(temp) = view.cpu.temperature_c {
        if temp >= 90.0 {
            score -= 14;
            reasons.push(format!("CPU is hot ({temp:.0}°C)."));
        }
    }
    for disk in view.disks {
        if !disk.removable && disk.utilization >= 0.9 {
            score -= 15;
            reasons.push(format!(
                "{} is running out of space ({:.0}% full).",
                disk.mount,
                disk.utilization * 100.0
            ));
        } else if !disk.removable && disk.utilization >= 0.85 {
            score -= 6;
            reasons.push(format!(
                "{} is getting full ({:.0}%).",
                disk.mount,
                disk.utilization * 100.0
            ));
        }
    }
    if disk_util >= 85.0 {
        score -= 8;
        reasons.push("Disk is busy enough to slow other work. This is transfer activity, not capacity.".into());
    }
    if let Some(since) = cpu_high_since {
        if now.saturating_duration_since(since.since).as_secs() >= 60 && !gaming {
            score -= 6;
            reasons.push("High CPU has lasted more than a minute.".into());
        }
    }
    let score = score.clamp(5, 100) as u8;
    let (label, summary) = if score >= 90 {
        (
            "Excellent".to_string(),
            "Thermals, capacity and load all look healthy.".to_string(),
        )
    } else if score >= 75 {
        (
            "Good".to_string(),
            "The machine is fine. A couple of resources are working harder than usual.".to_string(),
        )
    } else if score >= 55 {
        (
            "Needs Attention".to_string(),
            "Something is stressing the system enough that you may feel it.".to_string(),
        )
    } else {
        (
            "Rough".to_string(),
            "This is the kind of load that makes apps hitch. Check the alerts.".to_string(),
        )
    };
    if reasons.is_empty() {
        reasons.push("No active pressure signals.".into());
    }
    HealthSummary {
        score,
        label,
        summary,
        reasons,
    }
}

fn update_stretch(
    slot: &mut Option<Stretch>,
    over: bool,
    value: f32,
    recover_below: f32,
    process: Option<String>,
    now: Instant,
    kind: &str,
    persist: &mut Vec<ActivityEvent>,
) {
    if over {
        match slot {
            Some(s) => {
                if value > s.peak {
                    s.peak = value;
                }
                if process.is_some() {
                    s.process = process;
                }
            }
            None => {
                *slot = Some(Stretch {
                    since: now,
                    peak: value,
                    process,
                    alerted: false,
                });
            }
        }
        return;
    }
    if value > recover_below {
        return;
    }
    if let Some(s) = slot.take() {
            let secs = now.saturating_duration_since(s.since).as_secs();
            if s.alerted || secs >= 60 {
                let process = s.process.clone();
                persist.push(resolution_event(
                    kind,
                    match kind {
                        "cpu" => format!(
                            "CPU usage returned to normal after {}. Peak: {:.0}%.",
                            format_duration(secs),
                            s.peak
                        ),
                        "memory" => format!(
                            "Memory pressure eased after {}. Peak: {:.0}%.",
                            format_duration(secs),
                            s.peak
                        ),
                        "thermal" => format!(
                            "GPU temperature came down after {}. Peak: {:.0}°C.",
                            format_duration(secs),
                            s.peak
                        ),
                        "disk" => format!(
                            "Disk activity settled after {}. Peak activity {:.0}.",
                            format_duration(secs),
                            s.peak
                        ),
                        _ => format!("Load returned to normal after {}.", format_duration(secs)),
                    },
                    process,
                ));
            }
        }
}

fn resolution_event(category: &str, body: String, process: Option<String>) -> ActivityEvent {
    let ts = chrono::Utc::now().timestamp();
    ActivityEvent {
        id: format!("resolved-{category}-{ts}"),
        timestamp: ts,
        title: match category {
            "cpu" => "CPU usage returned to normal".into(),
            "memory" => "Memory pressure returned to normal".into(),
            "thermal" => "GPU temperature returned to normal".into(),
            "disk" => "Disk activity returned to normal".into(),
            "storage" => "Drive space is no longer critical".into(),
            _ => "Issue cleared".into(),
        },
        body,
        why: None,
        impact: "None".into(),
        recommendation: None,
        category: category.into(),
        kind: "resolved".into(),
        process,
        severity: "info".into(),
    }
}

fn should_persist(event: &ActivityEvent) -> bool {
    matches!(
        event.kind.as_str(),
        "sustained" | "temporary-spike" | "suspicious" | "capacity" | "high-resource" | "resolved"
    ) || event.severity == "warning"
        || event.severity == "critical"
        || event.category == "storage"
        || event.category == "security"
}

fn linger_hold(event: &ActivityEvent) -> u64 {
    if event.kind == "temporary-spike" {
        SPIKE_LINGER_SECS
    } else {
        LIVE_LINGER_SECS
    }
}

fn persist_cooldown(event: &ActivityEvent) -> u64 {
    if event.kind == "resolved" || event.category == "storage" {
        180
    } else if event.kind == "temporary-spike" {
        45
    } else {
        90
    }
}

fn maybe_push(
    last: &mut HashMap<String, Instant>,
    events: &mut Vec<ActivityEvent>,
    now: Instant,
    key: &str,
    event: ActivityEvent,
    cooldown: u64,
) {
    if let Some(prev) = last.get(key) {
        if now.saturating_duration_since(*prev).as_secs() < cooldown {
            return;
        }
    }
    last.insert(key.to_string(), now);
    events.push(event);
}

fn push_capped(buf: &mut VecDeque<f32>, value: f32, cap: usize) {
    if buf.len() == cap {
        buf.pop_front();
    }
    buf.push_back(value);
}

fn disk_busy(disks: &[DiskMetrics]) -> f32 {
    let mut any = false;
    let mut max_active = 0.0f32;
    for disk in disks {
        if let Some(active) = disk.active_time_percent {
            any = true;
            max_active = max_active.max(active);
        }
    }
    if any {
        return max_active;
    }
    if disks.is_empty() {
        return 0.0;
    }
    let io: u64 = disks.iter().map(|d| d.read_bps + d.write_bps).sum();
    ((io as f64 / (40.0 * 1024.0 * 1024.0)) * 100.0).clamp(0.0, 100.0) as f32
}

fn disk_total_bps(rows: &[ProcessRow]) -> u64 {
    rows.iter().map(|p| p.disk_bps).sum()
}

fn cpu_band(v: f32, secs: u64) -> String {
    if v < 40.0 {
        "normal".into()
    } else if v < 75.0 {
        "elevated".into()
    } else if v < 92.0 || secs < 30 {
        "high".into()
    } else {
        "critical".into()
    }
}

fn gpu_band(v: f32) -> String {
    if v < 40.0 {
        "normal".into()
    } else if v < 75.0 {
        "elevated".into()
    } else if v < 95.0 {
        "high".into()
    } else {
        "critical".into()
    }
}

fn mem_band(v: f32) -> String {
    if v < 70.0 {
        "normal".into()
    } else if v < 85.0 {
        "elevated".into()
    } else if v < 93.0 {
        "high".into()
    } else {
        "critical".into()
    }
}

fn disk_io_band(v: f32) -> String {
    if v < 25.0 {
        "normal".into()
    } else if v < 55.0 {
        "elevated".into()
    } else if v < 85.0 {
        "high".into()
    } else {
        "critical".into()
    }
}

fn net_band(bps: u64) -> String {
    if bps < 200_000 {
        "normal".into()
    } else if bps < 2_000_000 {
        "elevated".into()
    } else {
        "high".into()
    }
}

fn thermal_band(cpu: Option<f32>, gpu: Option<f32>) -> String {
    let max = cpu.into_iter().chain(gpu).fold(0.0_f32, f32::max);
    if max == 0.0 {
        "unknown".into()
    } else if max < 70.0 {
        "normal".into()
    } else if max < 85.0 {
        "elevated".into()
    } else {
        "high".into()
    }
}

fn impact(v: f32) -> String {
    if v < 60.0 {
        "Low".into()
    } else if v < 88.0 {
        "Moderate".into()
    } else {
        "High".into()
    }
}

fn why_text(name: &str) -> Option<String> {
    match hint_for(name) {
        Some(h) => Some(h.why.to_string()),
        None => Some("Unable to determine the exact cause.".into()),
    }
}

fn cpu_recommendation(name: &str, util: f32, gaming: bool, temp: Option<f32>) -> String {
    if gaming {
        return "A game is allowed to use the CPU. Check temperature if fans are screaming, not the CPU percent.".into();
    }
    if let Some(t) = temp {
        if t >= 90.0 {
            return format!("CPU is {t:.0}°C as well as busy. Let it cool, check airflow, then inspect {name}.");
        }
    }
    if is_known_benign(name) {
        "This process is a known Windows or app component. No action needed unless it stays pegged.".into()
    } else if util >= 95.0 {
        "If the machine feels stuck, inspect that process. Ending it is a last resort.".into()
    } else {
        "Watch it for a minute. Short spikes are normal.".into()
    }
}

pub fn normalize_name(name: &str) -> String {
    name.rsplit(['\\', '/'])
        .next()
        .unwrap_or(name)
        .trim_end_matches(".exe")
        .trim_end_matches(".EXE")
        .to_lowercase()
}

fn format_duration(secs: u64) -> String {
    if secs < 60 {
        format!("{secs} seconds")
    } else {
        let m = secs / 60;
        if m == 1 {
            "1 minute".into()
        } else {
            format!("{m} minutes")
        }
    }
}

pub fn process_hint(name: &str) -> Option<&'static str> {
    hint_for(name).map(|h| h.why)
}

pub fn process_expected(name: &str, count: usize) -> (bool, String) {
    if let Some(h) = hint_for(name) {
        let text = if count > 1 {
            format!("{} has {count} related processes. {}", name, h.normal)
        } else {
            h.normal.to_string()
        };
        (h.benign, text)
    } else {
        (true, "No built-in profile for this process.".into())
    }
}
