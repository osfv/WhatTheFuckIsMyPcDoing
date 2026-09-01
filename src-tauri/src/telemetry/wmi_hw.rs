#![allow(non_snake_case)]

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use wmi::Variant;

use crate::telemetry::pdh::{plausible_cpu_temp, tenths_kelvin_to_c, volume_key, DiskPerf};
use crate::types::{HardwareInfo, HwDevice, HwField};

#[derive(Clone, Debug, Default)]
pub struct WmiSnapshot {
    pub hardware: HardwareInfo,
    pub gpu_name: Option<String>,
    pub gpu_driver: Option<String>,
    #[allow(dead_code)]
    pub gpu_ram: Option<u64>,
    pub cpu_temperature_c: Option<f32>,
    pub package_power_w: Option<f32>,
    pub disk_perf: HashMap<String, DiskPerf>,
}

pub fn start_wmi_thread() -> Receiver<WmiSnapshot> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("wmi-inventory".into())
        .spawn(move || {
            let com = match wmi::COMLibrary::new() {
                Ok(c) => c,
                Err(err) => {
                    tracing::warn!("COM init failed: {err}");
                    return;
                }
            };
            let wmi_con = match wmi::WMIConnection::new(com) {
                Ok(c) => c,
                Err(err) => {
                    tracing::warn!("WMI connection failed: {err}");
                    return;
                }
            };
            let wmi_root = wmi::WMIConnection::with_namespace_path("root\\wmi", com).ok();
            let lhm =
                wmi::WMIConnection::with_namespace_path("root\\LibreHardwareMonitor", com).ok();
            let ohm =
                wmi::WMIConnection::with_namespace_path("root\\OpenHardwareMonitor", com).ok();
            let mut n = 0u32;
            let mut snap = WmiSnapshot::default();
            loop {
                if n % 20 == 0 {
                    snap = collect(&wmi_con, wmi_root.as_ref());
                }
                let sensors = collect_sensors(
                    &wmi_con,
                    wmi_root.as_ref(),
                    lhm.as_ref(),
                    ohm.as_ref(),
                );
                snap.cpu_temperature_c = sensors.temp;
                snap.package_power_w = sensors.power;
                snap.disk_perf = sensors.disks;
                if tx.send(snap.clone()).is_err() {
                    break;
                }
                n = n.wrapping_add(1);
                thread::sleep(Duration::from_secs(2));
            }
        })
        .ok();
    rx
}

fn collect(wmi: &wmi::WMIConnection, wmi_root: Option<&wmi::WMIConnection>) -> WmiSnapshot {
    let mut hardware = HardwareInfo {
        os_name: "Windows".into(),
        os_version: String::new(),
        hostname: sysinfo::System::host_name().unwrap_or_default(),
        uptime_sec: sysinfo::System::uptime(),
        ..Default::default()
    };

    if let Some(os) = query_rows::<Win32OperatingSystem>(wmi, "Win32_OperatingSystem")
        .into_iter()
        .next()
    {
        hardware.os_name = os.Caption.unwrap_or(hardware.os_name);
        hardware.os_version = os.Version.unwrap_or_default();
    }

    if let Some(cpu) = query_rows::<Win32Processor>(wmi, "Win32_Processor")
        .into_iter()
        .next()
    {
        hardware.cpu = Some(HwDevice {
            name: cpu.Name.unwrap_or_else(|| "CPU".into()).trim().to_string(),
            manufacturer: cpu.Manufacturer,
            detail: format!(
                "{} cores / {} threads",
                cpu.NumberOfCores.unwrap_or(0),
                cpu.NumberOfLogicalProcessors.unwrap_or(0)
            ),
            extras: vec![
                opt_field("Max clock", cpu.MaxClockSpeed.map(|m| format!("{m} MHz"))),
                opt_field("Socket", cpu.SocketDesignation),
            ]
            .into_iter()
            .flatten()
            .collect(),
        });
    }

    if let Some(board) = query_rows::<Win32BaseBoard>(wmi, "Win32_BaseBoard")
        .into_iter()
        .next()
    {
        hardware.motherboard = Some(HwDevice {
            name: board.Product.unwrap_or_else(|| "Motherboard".into()),
            manufacturer: board.Manufacturer.clone(),
            detail: board.Manufacturer.unwrap_or_default(),
            extras: vec![opt_field("Serial", board.SerialNumber)]
                .into_iter()
                .flatten()
                .collect(),
        });
    }

    {
        let rows = query_rows::<Win32PhysicalMemory>(wmi, "Win32_PhysicalMemory");
        hardware.memory_modules = rows
            .into_iter()
            .map(|m| {
                let cap = m.Capacity.unwrap_or(0);
                HwDevice {
                    name: format!("{:.0} GB", cap as f64 / (1024.0 * 1024.0 * 1024.0)),
                    manufacturer: m.Manufacturer.clone(),
                    detail: format!(
                        "{} MHz {}",
                        m.Speed.unwrap_or(0),
                        m.PartNumber.unwrap_or_default().trim()
                    ),
                    extras: vec![
                        opt_field("Slot", m.DeviceLocator),
                        opt_field("Form factor", m.FormFactor.map(|f| f.to_string())),
                    ]
                    .into_iter()
                    .flatten()
                    .collect(),
                }
            })
            .collect();
    }

    {
        let rows = query_rows::<Win32DiskDrive>(wmi, "Win32_DiskDrive");
        hardware.storage = rows
            .into_iter()
            .filter(|d| d.Size.unwrap_or(0) > 0)
            .map(|d| HwDevice {
                name: d.Model.unwrap_or_else(|| "Disk".into()),
                manufacturer: None,
                detail: format!(
                    "{:.1} GB",
                    d.Size.unwrap_or(0) as f64 / (1000.0 * 1000.0 * 1000.0)
                ),
                extras: vec![
                    opt_field("Interface", d.InterfaceType),
                    opt_field("Serial", d.SerialNumber.map(|s| s.trim().to_string())),
                ]
                .into_iter()
                .flatten()
                .collect(),
            })
            .collect();
    }

    {
        let rows = query_rows::<Win32NetworkAdapter>(wmi, "Win32_NetworkAdapter");
        hardware.network_adapters = rows
            .into_iter()
            .filter(|n| n.PhysicalAdapter.unwrap_or(false) && n.NetEnabled.unwrap_or(false))
            .map(|n| HwDevice {
                name: n.Name.unwrap_or_else(|| "Adapter".into()),
                manufacturer: n.Manufacturer,
                detail: n.MACAddress.unwrap_or_default(),
                extras: vec![opt_field(
                    "Link speed",
                    n.Speed.map(|s| format!("{:.0} Mbps", s as f64 / 1_000_000.0)),
                )]
                .into_iter()
                .flatten()
                .collect(),
            })
            .collect();
    }

    let mut gpu_name = None;
    let mut gpu_driver = None;
    let mut gpu_ram = None;
    {
        let rows = query_rows::<Win32VideoController>(wmi, "Win32_VideoController");
        hardware.gpus = rows
            .into_iter()
            .filter(|g| {
                g.Name
                    .as_ref()
                    .map(|n| !n.to_lowercase().contains("basic render"))
                    .unwrap_or(false)
            })
            .map(|g| {
                if gpu_name.is_none() {
                    gpu_name = g.Name.clone();
                    gpu_driver = g.DriverVersion.clone();
                    gpu_ram = g.AdapterRAM.filter(|b| *b > 0).map(|b| b as u64);
                }
                HwDevice {
                    name: g.Name.unwrap_or_else(|| "GPU".into()),
                    manufacturer: g.AdapterCompatibility.clone(),
                    detail: g.DriverVersion.clone().unwrap_or_else(|| {
                        "Driver version unavailable on this system".into()
                    }),
                    extras: vec![
                        opt_field(
                            "Reported VRAM",
                            g.AdapterRAM.filter(|b| *b > 0).map(|b| {
                                format!("{:.1} GB", b as f64 / (1024.0 * 1024.0 * 1024.0))
                            }),
                        ),
                        opt_field(
                            "Active mode",
                            match (g.CurrentHorizontalResolution, g.CurrentVerticalResolution) {
                                (Some(w), Some(h)) if w > 0 && h > 0 => Some(format!("{w} x {h}")),
                                _ => None,
                            },
                        ),
                    ]
                    .into_iter()
                    .flatten()
                    .collect(),
                }
            })
            .collect();
    }

    if let Some(mon) = wmi_root {
        hardware.monitors = query_rows::<WmiMonitorId>(mon, "WmiMonitorID")
            .into_iter()
            .map(|m| {
                let name = decode_wmi_ushort(m.UserFriendlyName)
                    .or_else(|| decode_wmi_ushort(m.SerialNumberID))
                    .unwrap_or_else(|| "Display".into());
                HwDevice {
                    name,
                    manufacturer: decode_wmi_ushort(m.ManufacturerName),
                    detail: "Connected display".into(),
                    extras: vec![],
                }
            })
            .collect();
    }

    WmiSnapshot {
        hardware,
        gpu_name,
        gpu_driver,
        gpu_ram,
        ..Default::default()
    }
}

struct SensorSample {
    temp: Option<f32>,
    power: Option<f32>,
    disks: HashMap<String, DiskPerf>,
}

fn collect_sensors(
    wmi: &wmi::WMIConnection,
    wmi_root: Option<&wmi::WMIConnection>,
    lhm: Option<&wmi::WMIConnection>,
    ohm: Option<&wmi::WMIConnection>,
) -> SensorSample {
    let mut temp = None;
    let mut power = None;

    if let Some(con) = lhm {
        let (t, p) = read_hardware_monitor(con);
        temp = t;
        power = p;
    }
    if temp.is_none() || power.is_none() {
        if let Some(con) = ohm {
            let (t, p) = read_hardware_monitor(con);
            temp = temp.or(t);
            power = power.or(p);
        }
    }
    if temp.is_none() {
        temp = read_thermal_zone(wmi);
    }
    if temp.is_none() {
        temp = wmi_root.and_then(read_acpi_thermal);
    }
    if temp.is_none() {
        temp = read_temperature_probe(wmi);
    }

    SensorSample {
        temp,
        power,
        disks: read_logical_disk(wmi),
    }
}

fn read_hardware_monitor(con: &wmi::WMIConnection) -> (Option<f32>, Option<f32>) {
    let Ok(rows) = con.raw_query::<HashMap<String, Variant>>(
        "SELECT Name, Identifier, SensorType, Value, Parent FROM Sensor",
    ) else {
        return (None, None);
    };

    let mut package_temp = None;
    let mut cpu_temps = Vec::new();
    let mut package_power = None;

    for row in rows {
        let sensor_type = variant_string(row.get("SensorType")).unwrap_or_default();
        let name = variant_string(row.get("Name")).unwrap_or_default();
        let ident = variant_string(row.get("Identifier")).unwrap_or_default();
        let parent = variant_string(row.get("Parent")).unwrap_or_default();
        let Some(value) = variant_f64(row.get("Value")) else {
            continue;
        };
        if !is_cpu_sensor(&ident, &name, &parent) {
            continue;
        }
        let type_l = sensor_type.to_lowercase();
        let name_l = name.to_lowercase();
        if type_l == "temperature" {
            let Some(c) = plausible_cpu_temp(value as f32) else {
                continue;
            };
            if name_l.contains("package") || name_l == "cpu" {
                package_temp = Some(c);
            } else {
                cpu_temps.push(c);
            }
        } else if type_l == "power"
            && (name_l.contains("package") || name_l.contains("cpu package"))
            && value.is_finite()
            && (0.5..=400.0).contains(&value)
        {
            package_power = Some(value as f32);
        }
    }

    let temp = package_temp.or_else(|| {
        cpu_temps
            .iter()
            .copied()
            .max_by(|a, b| a.total_cmp(b))
    });
    (temp, package_power)
}

fn is_cpu_sensor(ident: &str, name: &str, parent: &str) -> bool {
    let blob = format!("{ident} {name} {parent}").to_lowercase();
    if blob.contains("gpu") || blob.contains("nvidia") || blob.contains("radeon") {
        return false;
    }
    blob.contains("cpu")
        || blob.contains("intelcpu")
        || blob.contains("amdcpu")
        || blob.contains("processor")
}

fn read_thermal_zone(wmi: &wmi::WMIConnection) -> Option<f32> {
    let rows = wmi
        .raw_query::<HashMap<String, Variant>>(
            "SELECT Name, HighPrecisionTemperature, Temperature FROM Win32_PerfFormattedData_Counters_ThermalZoneInformation",
        )
        .unwrap_or_default();
    let mut best = None;
    for row in rows {
        let raw = variant_f64(row.get("HighPrecisionTemperature"))
            .or_else(|| variant_f64(row.get("Temperature")));
        let Some(c) = raw.and_then(tenths_kelvin_to_c) else {
            continue;
        };
        let name = variant_string(row.get("Name")).unwrap_or_default().to_lowercase();
        if name.contains("cpu") || name.contains("processor") || name.contains("tz00") {
            return Some(c);
        }
        best = Some(best.map_or(c, |prev: f32| prev.max(c)));
    }
    best
}

fn read_acpi_thermal(root: &wmi::WMIConnection) -> Option<f32> {
    let rows = query_rows_quiet::<MsAcpiThermal>(root);
    let mut best = None;
    for row in rows {
        let Some(raw) = row.CurrentTemperature else {
            continue;
        };
        let Some(c) = tenths_kelvin_to_c(raw as f64) else {
            continue;
        };
        let name = row.InstanceName.unwrap_or_default().to_lowercase();
        if name.contains("cpu") || name.contains("processor") || name.contains("tz00") {
            return Some(c);
        }
        best = Some(best.map_or(c, |prev: f32| prev.max(c)));
    }
    best
}

fn read_temperature_probe(wmi: &wmi::WMIConnection) -> Option<f32> {
    query_rows_quiet::<Win32TemperatureProbe>(wmi)
        .into_iter()
        .filter_map(|row| {
            let raw = row.CurrentReading?;
            tenths_kelvin_to_c(raw as f64).or_else(|| plausible_cpu_temp(raw as f32))
        })
        .max_by(|a, b| a.total_cmp(b))
}

fn read_logical_disk(wmi: &wmi::WMIConnection) -> HashMap<String, DiskPerf> {
    let Ok(rows) = wmi.raw_query::<HashMap<String, Variant>>(
        "SELECT Name, PercentIdleTime, PercentDiskTime, AvgDiskQueueLength, AvgDisksecPerTransfer, DiskReadBytesPersec, DiskWriteBytesPersec FROM Win32_PerfFormattedData_PerfDisk_LogicalDisk",
    ) else {
        return HashMap::new();
    };
    let mut out = HashMap::new();
    for row in rows {
        let name = variant_string(row.get("Name")).unwrap_or_default();
        if name.is_empty() || name == "_Total" {
            continue;
        }
        let key = volume_key(&name);
        if !key.contains(':') {
            continue;
        }
        let idle = variant_f64(row.get("PercentIdleTime"));
        let disk_time = variant_f64(row.get("PercentDiskTime"));
        let active = idle
            .map(|v| (100.0 - v).clamp(0.0, 100.0) as f32)
            .or_else(|| disk_time.map(|v| v.clamp(0.0, 100.0) as f32));
        let queue = variant_f64(row.get("AvgDiskQueueLength")).map(|v| v.max(0.0) as f32);
        let response_ms = variant_f64(row.get("AvgDisksecPerTransfer")).and_then(|sec| {
            if (0.0..=30.0).contains(&sec) {
                Some((sec * 1000.0) as f32)
            } else {
                None
            }
        });
        let read_bps = variant_f64(row.get("DiskReadBytesPersec")).map(|v| v.max(0.0) as u64);
        let write_bps = variant_f64(row.get("DiskWriteBytesPersec")).map(|v| v.max(0.0) as u64);
        out.insert(
            key,
            DiskPerf {
                active_time_percent: active,
                queue_length: queue,
                response_ms,
                read_bps,
                write_bps,
            },
        );
    }
    out
}

fn variant_f64(value: Option<&Variant>) -> Option<f64> {
    match value? {
        Variant::I1(v) => Some(*v as f64),
        Variant::I2(v) => Some(*v as f64),
        Variant::I4(v) => Some(*v as f64),
        Variant::I8(v) => Some(*v as f64),
        Variant::UI1(v) => Some(*v as f64),
        Variant::UI2(v) => Some(*v as f64),
        Variant::UI4(v) => Some(*v as f64),
        Variant::UI8(v) => Some(*v as f64),
        Variant::R4(v) => Some(*v as f64),
        Variant::R8(v) => Some(*v),
        Variant::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn variant_string(value: Option<&Variant>) -> Option<String> {
    match value? {
        Variant::String(s) => {
            let s = s.trim().to_string();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        }
        _ => None,
    }
}

fn query_rows<T>(wmi: &wmi::WMIConnection, class: &str) -> Vec<T>
where
    T: serde::de::DeserializeOwned,
{
    match wmi.query::<T>() {
        Ok(rows) => rows,
        Err(err) => {
            tracing::warn!("WMI {class} query failed: {err}");
            Vec::new()
        }
    }
}

fn query_rows_quiet<T>(wmi: &wmi::WMIConnection) -> Vec<T>
where
    T: serde::de::DeserializeOwned,
{
    wmi.query::<T>().unwrap_or_default()
}

fn opt_field(label: &str, value: Option<String>) -> Option<HwField> {
    value.filter(|v| !v.trim().is_empty()).map(|value| HwField {
        label: label.into(),
        value,
    })
}

fn decode_wmi_ushort(raw: Option<Vec<u16>>) -> Option<String> {
    let raw = raw?;
    let s = String::from_utf16(
        &raw.into_iter()
            .take_while(|c| *c != 0)
            .collect::<Vec<_>>(),
    )
    .ok()?;
    let s = s.trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_OperatingSystem")]
struct Win32OperatingSystem {
    Caption: Option<String>,
    Version: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_Processor")]
struct Win32Processor {
    Name: Option<String>,
    Manufacturer: Option<String>,
    NumberOfCores: Option<u32>,
    NumberOfLogicalProcessors: Option<u32>,
    MaxClockSpeed: Option<u32>,
    SocketDesignation: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_BaseBoard")]
struct Win32BaseBoard {
    Manufacturer: Option<String>,
    Product: Option<String>,
    SerialNumber: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_PhysicalMemory")]
struct Win32PhysicalMemory {
    Capacity: Option<u64>,
    Speed: Option<u32>,
    Manufacturer: Option<String>,
    PartNumber: Option<String>,
    DeviceLocator: Option<String>,
    FormFactor: Option<u16>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_DiskDrive")]
struct Win32DiskDrive {
    Model: Option<String>,
    Size: Option<u64>,
    InterfaceType: Option<String>,
    SerialNumber: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_NetworkAdapter")]
struct Win32NetworkAdapter {
    Name: Option<String>,
    Manufacturer: Option<String>,
    MACAddress: Option<String>,
    Speed: Option<u64>,
    PhysicalAdapter: Option<bool>,
    NetEnabled: Option<bool>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_VideoController")]
struct Win32VideoController {
    Name: Option<String>,
    AdapterRAM: Option<i64>,
    DriverVersion: Option<String>,
    AdapterCompatibility: Option<String>,
    CurrentHorizontalResolution: Option<u32>,
    CurrentVerticalResolution: Option<u32>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "WmiMonitorID")]
struct WmiMonitorId {
    UserFriendlyName: Option<Vec<u16>>,
    ManufacturerName: Option<Vec<u16>>,
    SerialNumberID: Option<Vec<u16>>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "MSAcpi_ThermalZoneTemperature")]
struct MsAcpiThermal {
    InstanceName: Option<String>,
    CurrentTemperature: Option<u32>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_TemperatureProbe")]
struct Win32TemperatureProbe {
    CurrentReading: Option<u32>,
}
