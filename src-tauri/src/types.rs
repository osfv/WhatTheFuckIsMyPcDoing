use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub timestamp: i64,
    pub paused: bool,
    pub cpu: CpuMetrics,
    pub gpu: GpuMetrics,
    pub memory: MemoryMetrics,
    pub disks: Vec<DiskMetrics>,
    pub network: NetworkMetrics,
    pub processes: Vec<ProcessRow>,
    pub top_processes: Vec<ProcessRow>,
    pub interpretation: Interpretation,
    pub health: HealthSummary,
    pub events: Vec<ActivityEvent>,
    pub alerts: Vec<Alert>,
    pub history: HistoryWindow,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HistoryWindow {
    pub cpu: Vec<f32>,
    pub gpu: Vec<f32>,
    pub memory: Vec<f32>,
    pub disk: Vec<f32>,
    pub network_down: Vec<f32>,
    pub network_up: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CpuMetrics {
    pub utilization: f32,
    pub per_core: Vec<f32>,
    pub frequency_mhz: Option<u64>,
    pub brand: String,
    pub core_count: usize,
    pub logical_count: usize,
    pub temperature_c: Option<f32>,
    pub package_power_w: Option<f32>,
    pub activity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GpuMetrics {
    pub name: String,
    pub vendor: String,
    pub utilization: Option<f32>,
    pub memory_used_bytes: Option<u64>,
    pub memory_total_bytes: Option<u64>,
    pub temperature_c: Option<f32>,
    pub core_clock_mhz: Option<u32>,
    pub memory_clock_mhz: Option<u32>,
    pub power_w: Option<f32>,
    pub fan_speed_percent: Option<u32>,
    pub driver_version: Option<String>,
    pub available: bool,
    pub unavailable_reason: Option<String>,
    pub activity: String,
    pub adapters: Vec<GpuAdapter>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GpuAdapter {
    pub name: String,
    pub vendor: String,
    pub utilization: Option<f32>,
    pub memory_used_bytes: Option<u64>,
    pub memory_total_bytes: Option<u64>,
    pub temperature_c: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MemoryMetrics {
    pub used_bytes: u64,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub committed_bytes: Option<u64>,
    pub cached_bytes: Option<u64>,
    pub swap_used_bytes: u64,
    pub swap_total_bytes: u64,
    pub utilization: f32,
    pub activity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DiskMetrics {
    pub name: String,
    pub mount: String,
    pub kind: String,
    pub fs: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub utilization: f32,
    pub read_bps: u64,
    pub write_bps: u64,
    pub active_time_percent: Option<f32>,
    pub response_ms: Option<f32>,
    pub queue_length: Option<f32>,
    pub removable: bool,
    pub activity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetworkMetrics {
    pub download_bps: u64,
    pub upload_bps: u64,
    pub interfaces: Vec<NetInterface>,
    pub connection_count: usize,
    pub activity: String,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetInterface {
    pub name: String,
    pub download_bps: u64,
    pub upload_bps: u64,
    pub total_received: u64,
    pub total_transmitted: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProcessRow {
    pub pid: u32,
    pub parent_pid: Option<u32>,
    pub name: String,
    pub display_name: String,
    pub cpu: f32,
    pub memory_bytes: u64,
    pub gpu: Option<f32>,
    pub gpu_memory_bytes: Option<u64>,
    pub gpu_engine: Option<String>,
    pub disk_bps: u64,
    pub disk_read_bps: u64,
    pub disk_write_bps: u64,
    pub connection_count: usize,
    pub path: Option<String>,
    pub status: String,
    pub start_time: u64,
    pub run_time_sec: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProcessDetails {
    pub process: ProcessRow,
    pub publisher: Option<String>,
    pub product: Option<String>,
    pub file_version: Option<String>,
    pub has_signature: bool,
    pub signature_status: String,
    pub command_line: Option<String>,
    pub children: Vec<ProcessRow>,
    pub connections: Vec<NetConnection>,
    pub hint: Option<String>,
    pub expected: bool,
    pub group_count: usize,
    pub group_memory_bytes: u64,
    pub group_cpu: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetConnection {
    pub protocol: String,
    pub local: String,
    pub remote: String,
    pub state: String,
    pub pid: u32,
    pub process_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Interpretation {
    pub headline: String,
    pub summary: String,
    pub mood: String,
    pub pressure: ResourcePressure,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResourcePressure {
    pub cpu: String,
    pub gpu: String,
    pub memory: String,
    pub disk: String,
    pub network: String,
    pub thermal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HealthSummary {
    pub score: u8,
    pub label: String,
    pub summary: String,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEvent {
    pub id: String,
    pub timestamp: i64,
    pub title: String,
    pub body: String,
    pub why: Option<String>,
    pub impact: String,
    pub recommendation: Option<String>,
    pub category: String,
    pub kind: String,
    pub process: Option<String>,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Alert {
    pub id: i64,
    pub timestamp: i64,
    pub title: String,
    pub body: String,
    pub severity: String,
    pub category: String,
    pub acknowledged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HardwareInfo {
    pub cpu: Option<HwDevice>,
    pub gpus: Vec<HwDevice>,
    pub motherboard: Option<HwDevice>,
    pub memory_modules: Vec<HwDevice>,
    pub storage: Vec<HwDevice>,
    pub network_adapters: Vec<HwDevice>,
    pub monitors: Vec<HwDevice>,
    pub os_name: String,
    pub os_version: String,
    pub hostname: String,
    pub uptime_sec: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HwDevice {
    pub name: String,
    pub manufacturer: Option<String>,
    pub detail: String,
    pub extras: Vec<HwField>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HwField {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPoint {
    pub timestamp: i64,
    pub cpu: f32,
    pub gpu: f32,
    pub memory: f32,
    pub disk: f32,
    pub network_down: f32,
    pub network_up: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub onboarding_complete: bool,
    pub start_on_login: bool,
    pub minimize_to_tray: bool,
    pub sample_interval_ms: u64,
    pub history_retention_days: u32,
    pub notifications_enabled: bool,
    #[serde(default = "default_notify_min_severity")]
    pub notify_min_severity: String,
    pub accent_color: String,
    pub temperature_unit: String,
    pub reduced_motion: bool,
    pub pause_monitoring: bool,
    pub monitor_cpu: bool,
    pub monitor_gpu: bool,
    pub monitor_memory: bool,
    pub monitor_disk: bool,
    pub monitor_network: bool,
    pub monitor_processes: bool,
    pub cpu_alert_percent: f32,
    pub cpu_alert_seconds: u64,
    pub gpu_temp_alert_c: f32,
    pub gpu_temp_alert_seconds: u64,
    pub ram_alert_percent: f32,
    #[serde(default = "default_ram_alert_seconds")]
    pub ram_alert_seconds: u64,
    pub disk_full_percent: f32,
    #[serde(default)]
    pub discord_presence: bool,
    #[serde(default)]
    pub discord_application_id: String,
    #[serde(default)]
    pub github_url: String,
}

fn default_notify_min_severity() -> String {
    "warning".into()
}

fn default_ram_alert_seconds() -> u64 {
    120
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            onboarding_complete: false,
            start_on_login: false,
            minimize_to_tray: true,
            sample_interval_ms: 1000,
            history_retention_days: 7,
            notifications_enabled: true,
            notify_min_severity: default_notify_min_severity(),
            accent_color: "#FF6B57".into(),
            temperature_unit: "celsius".into(),
            reduced_motion: false,
            pause_monitoring: false,
            monitor_cpu: true,
            monitor_gpu: true,
            monitor_memory: true,
            monitor_disk: true,
            monitor_network: true,
            monitor_processes: true,
            cpu_alert_percent: 90.0,
            cpu_alert_seconds: 300,
            gpu_temp_alert_c: 85.0,
            gpu_temp_alert_seconds: 480,
            ram_alert_percent: 92.0,
            ram_alert_seconds: default_ram_alert_seconds(),
            disk_full_percent: 90.0,
            discord_presence: false,
            discord_application_id: String::new(),
            github_url: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BatteryStatus {
    pub present: bool,
    pub state: String,
    pub percent: Option<f32>,
    pub ac_online: Option<bool>,
    pub rate_w: Option<f32>,
    pub time_remaining_sec: Option<u64>,
    pub time_to_full_sec: Option<u64>,
    pub remaining_wh: Option<f32>,
    pub full_wh: Option<f32>,
    pub design_wh: Option<f32>,
    pub health_percent: Option<f32>,
    pub health_label: String,
    pub cycle_count: Option<u32>,
    pub name: Option<String>,
    pub manufacturer: Option<String>,
    pub headline: String,
    pub summary: String,
    pub mocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppPower {
    pub key: String,
    pub name: String,
    pub pid: u32,
    pub watts: f32,
    pub share: f32,
    pub cpu_w: f32,
    pub gpu_w: f32,
    pub disk_w: f32,
    pub cpu: f32,
    pub gpu: f32,
    pub disk_bps: u64,
    pub main_component: String,
    pub battery_percent_per_hour: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PowerReport {
    pub battery: BatteryStatus,
    pub apps: Vec<AppPower>,
    pub headline: String,
    pub summary: String,
    pub source: String,
    pub source_label: String,
    pub estimated_app_w: f32,
    pub system_draw_w: Option<f32>,
    pub other_w: Option<f32>,
    pub updated_at: i64,
}
