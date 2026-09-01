pub mod gpu;
pub mod pdh;
pub mod peinfo;
pub mod wmi_hw;

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use sysinfo::{
    Components, CpuRefreshKind, Disks, MemoryRefreshKind, Networks, ProcessRefreshKind, ProcessesToUpdate,
    RefreshKind, System, UpdateKind,
};

use crate::analysis::{process_expected, process_hint, AnalysisEngine, RawView};
use crate::storage::Database;
use crate::telemetry::gpu::GpuCollector;
use crate::telemetry::pdh::{volume_key, DiskPerf, PdhSample, PdhSampler};
use crate::telemetry::peinfo::enrich_details;
use crate::telemetry::wmi_hw::{start_wmi_thread, WmiSnapshot};
use crate::types::{
    AppSettings, CpuMetrics, DiskMetrics, HardwareInfo, HistoryPoint, HistoryWindow, HwDevice,
    HwField, MemoryMetrics, NetConnection, NetInterface, NetworkMetrics, ProcessDetails, ProcessRow,
    Snapshot,
};

pub struct Collector {
    sys: System,
    disks: Disks,
    networks: Networks,
    components: Components,
    gpu: GpuCollector,
    analysis: AnalysisEngine,
    wmi_rx: Receiver<WmiSnapshot>,
    wmi: WmiSnapshot,
    pdh: Option<PdhSampler>,
    last_connections: Vec<NetConnection>,
    last_conn_refresh: Instant,
    last_sample_write: Instant,
    last_prune: Instant,
}

impl Collector {
    pub fn new() -> Self {
        let mut sys = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            process_kind(),
        );
        std::thread::sleep(Duration::from_millis(220));
        sys.refresh_cpu_all();
        sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            process_kind(),
        );

        Self {
            sys,
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            components: Components::new_with_refreshed_list(),
            gpu: GpuCollector::new(),
            analysis: AnalysisEngine::default(),
            wmi_rx: start_wmi_thread(),
            wmi: WmiSnapshot::default(),
            pdh: PdhSampler::new(),
            last_connections: Vec::new(),
            last_conn_refresh: Instant::now() - Duration::from_secs(10),
            last_sample_write: Instant::now() - Duration::from_secs(20),
            last_prune: Instant::now(),
        }
    }

    pub fn tick(&mut self, settings: &AppSettings, db: &Database) -> Snapshot {
        while let Ok(snap) = self.wmi_rx.try_recv() {
            self.wmi = snap;
        }

        if settings.monitor_cpu || settings.monitor_memory {
            self.sys.refresh_specifics(
                RefreshKind::nothing()
                    .with_cpu(CpuRefreshKind::everything())
                    .with_memory(MemoryRefreshKind::everything()),
            );
        }
        if settings.monitor_processes {
            self.sys.refresh_processes_specifics(
                ProcessesToUpdate::All,
                true,
                process_kind(),
            );
        }
        if settings.monitor_disk {
            self.disks.refresh(true);
        }
        if settings.monitor_network {
            self.networks.refresh(true);
        }
        self.components.refresh(true);

        let pdh = self.pdh.as_mut().map(|p| p.sample()).unwrap_or_default();
        let cpu = sample_cpu(&self.sys, &self.components, &self.wmi, pdh.thermal_c);
        let mut gpu = if settings.monitor_gpu {
            self.gpu.sample()
        } else {
            crate::types::GpuMetrics {
                name: "GPU monitoring disabled".into(),
                available: false,
                unavailable_reason: Some("Disabled in settings.".into()),
                ..Default::default()
            }
        };
        if gpu.driver_version.is_none() {
            gpu.driver_version = self.wmi.gpu_driver.clone();
        }
        if gpu.name.is_empty() || gpu.name == "Unavailable on this system" {
            if let Some(name) = &self.wmi.gpu_name {
                gpu.name = name.clone();
            }
        }

        let memory = sample_memory(&self.sys);
        let gpu_map = if settings.monitor_gpu {
            self.gpu.process_gpu()
        } else {
            HashMap::new()
        };

        if self.last_conn_refresh.elapsed() >= Duration::from_secs(2) && settings.monitor_network {
            self.last_connections = sample_connections(&self.sys);
            self.last_conn_refresh = Instant::now();
        }
        let conn_counts = connection_counts(&self.last_connections);

        let processes = if settings.monitor_processes {
            sample_processes(&self.sys, &gpu_map, &conn_counts)
        } else {
            Vec::new()
        };

        let disks = if settings.monitor_disk {
            sample_disks(&self.disks, &processes, &pdh, &self.wmi.disk_perf)
        } else {
            Vec::new()
        };
        let mut network = if settings.monitor_network {
            sample_network(&self.networks, self.last_connections.len())
        } else {
            NetworkMetrics::default()
        };
        network.explanation = network_explanation(&network, &processes);

        let view = RawView {
            cpu: &cpu,
            gpu: &gpu,
            memory: &memory,
            disks: &disks,
            network: &network,
            processes: &processes,
        };
        let analyzed = self.analysis.push_and_analyze(view, settings);

        for event in &analyzed.persist_events {
            let _ = db.insert_event(event);
        }
        let mut alerts = db.recent_alerts(12).unwrap_or_default();
        for seed in analyzed.new_alerts {
            if let Ok(Some(alert)) = db.insert_alert_if_new(
                &seed.fingerprint,
                chrono::Utc::now().timestamp(),
                &seed.severity,
                &seed.category,
                &seed.title,
                &seed.body,
                120,
            ) {
                if seed.notify
                    && crate::notify::should_notify(
                        settings.notifications_enabled,
                        &settings.notify_min_severity,
                        &alert.severity,
                    )
                {
                    crate::notify::toast(&alert.title, &alert.body);
                }
                alerts.insert(0, alert);
            }
        }

        if self.last_sample_write.elapsed() >= Duration::from_secs(15) {
            let _ = db.insert_sample(&HistoryPoint {
                timestamp: chrono::Utc::now().timestamp(),
                cpu: cpu.utilization,
                gpu: gpu.utilization.unwrap_or(0.0),
                memory: memory.utilization,
                disk: disks
                    .iter()
                    .map(|d| d.read_bps + d.write_bps)
                    .sum::<u64>() as f32
                    / (1024.0 * 1024.0),
                network_down: network.download_bps as f32 / (1024.0 * 1024.0),
                network_up: network.upload_bps as f32 / (1024.0 * 1024.0),
            });
            self.last_sample_write = Instant::now();
        }
        if self.last_prune.elapsed() >= Duration::from_secs(3600) {
            let _ = db.prune(settings.history_retention_days);
            self.last_prune = Instant::now();
        }

        let mut ranked = crate::analysis::grouped_process_rows(&processes);
        ranked.sort_by(|a, b| {
            b.cpu
                .total_cmp(&a.cpu)
                .then(b.memory_bytes.cmp(&a.memory_bytes))
        });
        ranked.truncate(8);

        Snapshot {
            timestamp: chrono::Utc::now().timestamp_millis(),
            paused: settings.pause_monitoring,
            cpu,
            gpu,
            memory,
            disks,
            network,
            processes,
            top_processes: ranked,
            interpretation: analyzed.interpretation,
            health: analyzed.health,
            events: analyzed.events,
            alerts,
            history: HistoryWindow {
                cpu: self.analysis.sparkline_cpu(),
                gpu: self.analysis.sparkline_gpu(),
                memory: self.analysis.sparkline_memory(),
                disk: self.analysis.sparkline_disk(),
                network_down: self.analysis.sparkline_net_down(),
                network_up: self.analysis.sparkline_net_up(),
            },
        }
    }

    pub fn hardware(&self) -> HardwareInfo {
        let mut hw = self.wmi.hardware.clone();
        if hw.hostname.is_empty() {
            hw.hostname = System::host_name().unwrap_or_default();
        }
        hw.uptime_sec = System::uptime();
        if hw.os_name.is_empty() {
            hw.os_name = System::name().unwrap_or_else(|| "Windows".into());
            hw.os_version = System::os_version().unwrap_or_default();
        }
        if hw.cpu.is_none() {
            hw.cpu = Some(cpu_device_from_sys(&self.sys));
        }
        let live_gpus = self.gpu.inventory();
        if hw.gpus.is_empty() {
            hw.gpus = live_gpus;
        } else {
            overlay_live_vram(&mut hw.gpus, &live_gpus);
        }
        if hw.memory_modules.is_empty() {
            hw.memory_modules = vec![ram_device_from_sys(&self.sys)];
        }
        if hw.storage.is_empty() {
            hw.storage = storage_from_disks(&self.disks);
        }
        hw
    }

    pub fn connections(&self) -> Vec<NetConnection> {
        self.last_connections.clone()
    }

    pub fn process_details(&self, pid: u32, snapshot: &Snapshot) -> Option<ProcessDetails> {
        let process = snapshot.processes.iter().find(|p| p.pid == pid)?.clone();
        let children: Vec<ProcessRow> = snapshot
            .processes
            .iter()
            .filter(|p| p.parent_pid == Some(pid))
            .cloned()
            .collect();
        let connections: Vec<NetConnection> = self
            .last_connections
            .iter()
            .filter(|c| c.pid == pid)
            .cloned()
            .collect();
        let group_name = crate::analysis::normalize_name(&process.name);
        let group: Vec<&ProcessRow> = snapshot
            .processes
            .iter()
            .filter(|p| crate::analysis::normalize_name(&p.name) == group_name)
            .collect();
        let group_count = group.len();
        let group_memory_bytes = group.iter().map(|p| p.memory_bytes).sum();
        let group_cpu = group.iter().map(|p| p.cpu).sum();
        let (expected, hint_text) = process_expected(&process.name, group_count);
        let meta = process
            .path
            .as_deref()
            .map(enrich_details)
            .unwrap_or_default();
        let cmd = self
            .sys
            .process(sysinfo::Pid::from_u32(pid))
            .map(|p| {
                p.cmd()
                    .iter()
                    .map(|s| s.to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .filter(|s| !s.is_empty());

        Some(ProcessDetails {
            process,
            publisher: meta.publisher,
            product: meta.product,
            file_version: meta.file_version,
            has_signature: meta.has_signature,
            signature_status: meta.signature_status,
            command_line: cmd,
            children,
            connections,
            hint: process_hint(&group_name)
                .map(|s| s.to_string())
                .or(Some(hint_text.clone())),
            expected,
            group_count,
            group_memory_bytes,
            group_cpu,
        })
    }

    pub fn kill_process(&mut self, pid: u32) -> Result<(), String> {
        match self.sys.process(sysinfo::Pid::from_u32(pid)) {
            Some(proc) => {
                if proc.kill() {
                    Ok(())
                } else {
                    Err("Windows refused to end this process. It may require elevation.".into())
                }
            }
            None => Err("Process is no longer running.".into()),
        }
    }
}

fn process_kind() -> ProcessRefreshKind {
    ProcessRefreshKind::nothing()
        .with_cpu()
        .with_memory()
        .with_disk_usage()
        .with_exe(UpdateKind::OnlyIfNotSet)
        .with_cmd(UpdateKind::OnlyIfNotSet)
}

fn sample_cpu(
    sys: &System,
    components: &Components,
    wmi: &WmiSnapshot,
    pdh_thermal_c: Option<f32>,
) -> CpuMetrics {
    let cpus = sys.cpus();
    let per_core: Vec<f32> = cpus.iter().map(|c| c.cpu_usage()).collect();
    let brand = cpus
        .first()
        .map(|c| c.brand().trim().to_string())
        .unwrap_or_default();
    let frequency_mhz = cpus.first().map(|c| c.frequency()).filter(|f| *f > 0);
    let temperature_c = wmi
        .cpu_temperature_c
        .or(pdh_thermal_c)
        .or_else(|| temp_from_components(components));
    let utilization = sys.global_cpu_usage();
    CpuMetrics {
        utilization,
        per_core,
        frequency_mhz,
        brand,
        core_count: System::physical_core_count().unwrap_or(cpus.len()),
        logical_count: cpus.len(),
        temperature_c,
        package_power_w: wmi.package_power_w,
        activity: if utilization < 12.0 {
            "Mostly idle".into()
        } else if utilization < 45.0 {
            "Light background work".into()
        } else if utilization < 80.0 {
            "Actively computing".into()
        } else {
            "Saturated".into()
        },
    }
}

fn temp_from_components(components: &Components) -> Option<f32> {
    let mut cpuish = None;
    let mut thermal = None;
    for c in components.iter() {
        let Some(raw) = c.temperature() else {
            continue;
        };
        let Some(t) = crate::telemetry::pdh::plausible_cpu_temp(raw) else {
            continue;
        };
        let label = c.label().to_lowercase();
        if label.contains("gpu") || label.contains("nvidia") || label.contains("radeon") {
            continue;
        }
        if label.contains("cpu")
            || label.contains("package")
            || label.contains("tctl")
            || label.contains("tdie")
        {
            cpuish = Some(t);
            break;
        }
        if label.contains("tz") || label.contains("acpi") || label.contains("thermal") {
            thermal = Some(thermal.map_or(t, |prev: f32| prev.max(t)));
        }
    }
    cpuish.or(thermal)
}

fn sample_memory(sys: &System) -> MemoryMetrics {
    let total = sys.total_memory();
    let used = sys.used_memory();
    let available = sys.available_memory();
    let utilization = if total == 0 {
        0.0
    } else {
        (used as f32 / total as f32) * 100.0
    };
    MemoryMetrics {
        used_bytes: used,
        total_bytes: total,
        available_bytes: available,
        committed_bytes: None,
        cached_bytes: Some(sys.available_memory().saturating_sub(sys.free_memory())),
        swap_used_bytes: sys.used_swap(),
        swap_total_bytes: sys.total_swap(),
        utilization,
        activity: if utilization < 55.0 {
            "Plenty of headroom".into()
        } else if utilization < 80.0 {
            "Comfortably occupied".into()
        } else if utilization < 92.0 {
            "Getting tight".into()
        } else {
            "Pressure is high".into()
        },
    }
}

fn sample_disks(
    disks: &Disks,
    processes: &[ProcessRow],
    pdh: &PdhSample,
    wmi_disks: &HashMap<String, DiskPerf>,
) -> Vec<DiskMetrics> {
    let total_read: u64 = processes.iter().map(|p| p.disk_read_bps).sum();
    let total_write: u64 = processes.iter().map(|p| p.disk_write_bps).sum();
    let mut out: Vec<DiskMetrics> = disks
        .iter()
        .filter(|d| d.total_space() > 0)
        .map(|d| {
            let total = d.total_space();
            let available = d.available_space();
            let used = total.saturating_sub(available);
            let utilization = if total == 0 {
                0.0
            } else {
                used as f32 / total as f32
            };
            let mount = d.mount_point().to_string_lossy().into_owned();
            let key = volume_key(&mount);
            let perf = pdh.disks.get(&key).or_else(|| wmi_disks.get(&key));
            let read_bps = perf.and_then(|p| p.read_bps).unwrap_or(0);
            let write_bps = perf.and_then(|p| p.write_bps).unwrap_or(0);
            DiskMetrics {
                name: d.name().to_string_lossy().into_owned(),
                mount,
                kind: format!("{:?}", d.kind()),
                fs: d.file_system().to_string_lossy().into_owned(),
                total_bytes: total,
                used_bytes: used,
                available_bytes: available,
                utilization,
                read_bps,
                write_bps,
                active_time_percent: perf.and_then(|p| p.active_time_percent),
                response_ms: perf.and_then(|p| p.response_ms),
                queue_length: perf.and_then(|p| p.queue_length),
                removable: d.is_removable(),
                activity: io_activity_label(read_bps.saturating_add(write_bps)),
            }
        })
        .collect();

    let pdh_ready = !pdh.disks.is_empty();
    if !pdh_ready {
        if let Some(first) = out.iter_mut().find(|d| d.mount.to_ascii_uppercase().starts_with("C:")) {
            first.read_bps = total_read;
            first.write_bps = total_write;
            first.activity = io_activity_label(total_read + total_write);
        } else if let Some(first) = out.first_mut() {
            first.read_bps = total_read;
            first.write_bps = total_write;
            first.activity = io_activity_label(total_read + total_write);
        }
    }
    out
}

fn io_activity_label(total_bps: u64) -> String {
    if total_bps < 80_000 {
        "Idle".into()
    } else if total_bps < 2_000_000 {
        "Light file activity".into()
    } else if total_bps < 15_000_000 {
        "Active disk I/O".into()
    } else {
        "Heavy file activity".into()
    }
}

fn sample_network(networks: &Networks, connection_count: usize) -> NetworkMetrics {
    let mut interfaces = Vec::new();
    let mut download = 0u64;
    let mut upload = 0u64;
    for (name, data) in networks.iter() {
        let n = name.to_lowercase();
        if n.contains("loopback") || n.contains("isatap") || n.contains("teredo") {
            continue;
        }
        let down = data.received();
        let up = data.transmitted();
        download += down;
        upload += up;
        interfaces.push(NetInterface {
            name: name.clone(),
            download_bps: down,
            upload_bps: up,
            total_received: data.total_received(),
            total_transmitted: data.total_transmitted(),
        });
    }
    interfaces.sort_by(|a, b| (b.download_bps + b.upload_bps).cmp(&(a.download_bps + a.upload_bps)));
    NetworkMetrics {
        download_bps: download,
        upload_bps: upload,
        interfaces,
        connection_count,
        activity: if download + upload < 50_000 {
            "Quiet".into()
        } else if download + upload < 1_000_000 {
            "Light traffic".into()
        } else if upload > download * 2 {
            "Uploading".into()
        } else {
            "Downloading".into()
        },
        explanation: String::new(),
    }
}

fn network_explanation(net: &NetworkMetrics, processes: &[ProcessRow]) -> String {
    if net.download_bps + net.upload_bps < 80_000 {
        return "The network is quiet. Nothing here looks like a transfer.".into();
    }
    let top = processes.iter().max_by_key(|p| p.connection_count);
    if let Some(p) = top {
        if p.connection_count > 0 {
            if net.upload_bps > 2_000_000 {
                return format!(
                    "Large upload in progress. {} currently holds the most connections.",
                    p.name
                );
            }
            return format!(
                "{} is receiving traffic normally with {} active sockets.",
                p.name, p.connection_count
            );
        }
    }
    "Traffic is visible on an interface, but Windows did not attribute it to a single process.".into()
}

fn sample_processes(
    sys: &System,
    gpu_map: &HashMap<u32, (f32, u64)>,
    conn_counts: &HashMap<u32, usize>,
) -> Vec<ProcessRow> {
    let logical = sys.cpus().len().max(1) as f32;
    let mut rows = Vec::with_capacity(sys.processes().len());
    for (pid, proc) in sys.processes() {
        let pid_u = pid.as_u32();
        let name = proc.name().to_string_lossy().into_owned();
        if name.is_empty() {
            continue;
        }
        let disk = proc.disk_usage();
        let (gpu, gpu_mem) = gpu_map.get(&pid_u).copied().unwrap_or((0.0, 0));
        let display_name = crate::analysis::display_name(&name);
        rows.push(ProcessRow {
            pid: pid_u,
            parent_pid: proc.parent().map(|p| p.as_u32()),
            name,
            display_name,
            cpu: proc.cpu_usage() / logical,
            memory_bytes: proc.memory(),
            gpu: if gpu > 0.0 { Some(gpu) } else { None },
            gpu_memory_bytes: if gpu_mem > 0 { Some(gpu_mem) } else { None },
            gpu_engine: if gpu > 0.0 {
                Some("3D / Compute".into())
            } else {
                None
            },
            disk_bps: disk.read_bytes.saturating_add(disk.written_bytes),
            disk_read_bps: disk.read_bytes,
            disk_write_bps: disk.written_bytes,
            connection_count: conn_counts.get(&pid_u).copied().unwrap_or(0),
            path: proc.exe().map(|p| p.to_string_lossy().into_owned()),
            status: format!("{:?}", proc.status()),
            start_time: proc.start_time(),
            run_time_sec: proc.run_time(),
        });
    }
    rows
}

fn sample_connections(sys: &System) -> Vec<NetConnection> {
    let af = netstat2::AddressFamilyFlags::IPV4 | netstat2::AddressFamilyFlags::IPV6;
    let proto = netstat2::ProtocolFlags::TCP | netstat2::ProtocolFlags::UDP;
    let Ok(sockets) = netstat2::get_sockets_info(af, proto) else {
        return Vec::new();
    };
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for socket in sockets {
        let pid = socket.associated_pids.iter().copied().next().unwrap_or(0);
        let name = sys
            .process(sysinfo::Pid::from_u32(pid))
            .map(|p| p.name().to_string_lossy().into_owned())
            .unwrap_or_else(|| "System".into());
        let (protocol, local, remote, state) = match socket.protocol_socket_info {
            netstat2::ProtocolSocketInfo::Tcp(tcp) => (
                "TCP".to_string(),
                format!("{}:{}", tcp.local_addr, tcp.local_port),
                format!("{}:{}", tcp.remote_addr, tcp.remote_port),
                format!("{:?}", tcp.state),
            ),
            netstat2::ProtocolSocketInfo::Udp(udp) => (
                "UDP".to_string(),
                format!("{}:{}", udp.local_addr, udp.local_port),
                "*:*".into(),
                "Listen".into(),
            ),
        };
        let key = (pid, protocol.clone(), local.clone(), remote.clone());
        if !seen.insert(key) {
            continue;
        }
        out.push(NetConnection {
            protocol,
            local,
            remote,
            state,
            pid,
            process_name: name,
        });
    }
    out
}

fn connection_counts(conns: &[NetConnection]) -> HashMap<u32, usize> {
    let mut map = HashMap::new();
    for c in conns {
        *map.entry(c.pid).or_insert(0) += 1;
    }
    map
}

fn overlay_live_vram(gpus: &mut [HwDevice], live: &[HwDevice]) {
    for gpu in gpus.iter_mut() {
        let src = live.iter().find(|l| gpu_names_match(&gpu.name, &l.name));
        let Some(src) = src else {
            continue;
        };
        gpu.extras.retain(|e| e.label != "Reported VRAM" && e.label != "VRAM");
        let value = src
            .detail
            .strip_suffix(" VRAM")
            .unwrap_or(src.detail.as_str())
            .to_string();
        if value.contains("GB") || value.contains("MB") {
            gpu.extras.insert(
                0,
                HwField {
                    label: "VRAM".into(),
                    value,
                },
            );
        }
        if gpu.manufacturer.is_none() {
            gpu.manufacturer = src.manufacturer.clone();
        }
    }
}

fn gpu_names_match(a: &str, b: &str) -> bool {
    let na = a.to_lowercase();
    let nb = b.to_lowercase();
    if na.contains(&nb) || nb.contains(&na) {
        return true;
    }
    let digits = |s: &str| -> Vec<String> {
        s.split(|c: char| !c.is_ascii_digit())
            .filter(|t| t.len() >= 3)
            .map(|t| t.to_string())
            .collect()
    };
    let da = digits(&na);
    let db = digits(&nb);
    !da.is_empty() && da.iter().any(|d| db.contains(d))
}

fn cpu_device_from_sys(sys: &System) -> HwDevice {
    let cpus = sys.cpus();
    let brand = cpus
        .first()
        .map(|c| c.brand().trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "CPU".into());
    let logical = cpus.len();
    let physical = System::physical_core_count().unwrap_or(logical);
    let freq = cpus.first().map(|c| c.frequency()).filter(|f| *f > 0);
    HwDevice {
        name: brand,
        manufacturer: None,
        detail: format!("{physical} cores / {logical} threads"),
        extras: freq
            .map(|mhz| crate::types::HwField {
                label: "Clock".into(),
                value: format!("{mhz} MHz"),
            })
            .into_iter()
            .collect(),
    }
}

fn ram_device_from_sys(sys: &System) -> HwDevice {
    let total = sys.total_memory();
    HwDevice {
        name: format!("{:.0} GB installed", total as f64 / (1024.0 * 1024.0 * 1024.0)),
        manufacturer: None,
        detail: format!(
            "{:.1} GB in use right now",
            sys.used_memory() as f64 / (1024.0 * 1024.0 * 1024.0)
        ),
        extras: vec![],
    }
}

fn storage_from_disks(disks: &Disks) -> Vec<HwDevice> {
    disks
        .iter()
        .filter(|d| d.total_space() > 0)
        .map(|d| {
            let total = d.total_space();
            let mount = d.mount_point().to_string_lossy().into_owned();
            let name = d.name().to_string_lossy().into_owned();
            HwDevice {
                name: if name.is_empty() { mount.clone() } else { name },
                manufacturer: None,
                detail: format!(
                    "{:.1} GB · {}",
                    total as f64 / (1000.0 * 1000.0 * 1000.0),
                    mount
                ),
                extras: vec![],
            }
        })
        .collect()
}
