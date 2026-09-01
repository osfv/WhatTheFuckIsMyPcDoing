use std::collections::HashMap;

use nvml_wrapper::enum_wrappers::device::{Clock, TemperatureSensor};
use nvml_wrapper::enums::device::UsedGpuMemory;
use nvml_wrapper::Nvml;

use crate::types::{GpuAdapter, GpuMetrics, HwDevice};

pub struct GpuCollector {
    nvml: Option<Nvml>,
    last_nvml_proc_ts: u64,
}

impl GpuCollector {
    pub fn new() -> Self {
        let nvml = Nvml::init().ok();
        if nvml.is_some() {
            tracing::info!("NVML initialized");
        } else {
            tracing::info!("NVML not available; GPU utilization may be limited");
        }
        Self {
            nvml,
            last_nvml_proc_ts: 0,
        }
    }

    pub fn sample(&mut self) -> GpuMetrics {
        if let Some(metrics) = self.sample_nvml() {
            return metrics;
        }
        sample_dxgi_fallback()
    }

    pub fn inventory(&self) -> Vec<HwDevice> {
        let metrics = self.sample_nvml().unwrap_or_else(sample_dxgi_fallback);
        if metrics.adapters.is_empty() {
            if metrics.name.is_empty() || metrics.name == "Unavailable on this system" {
                return Vec::new();
            }
            return vec![HwDevice {
                name: metrics.name,
                manufacturer: if metrics.vendor.is_empty() {
                    None
                } else {
                    Some(metrics.vendor)
                },
                detail: metrics
                    .unavailable_reason
                    .unwrap_or_else(|| "Live GPU telemetry".into()),
                extras: vec![],
            }];
        }
        metrics
            .adapters
            .into_iter()
            .map(|a| HwDevice {
                name: a.name,
                manufacturer: if a.vendor.is_empty() {
                    None
                } else {
                    Some(a.vendor)
                },
                detail: match a.memory_total_bytes {
                    Some(bytes) if bytes > 0 => {
                        format!("{:.1} GB VRAM", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
                    }
                    _ => "VRAM unavailable on this system".into(),
                },
                extras: a
                    .utilization
                    .map(|u| crate::types::HwField {
                        label: "Utilization".into(),
                        value: format!("{u:.0}%"),
                    })
                    .into_iter()
                    .collect(),
            })
            .collect()
    }

    pub fn process_gpu(&mut self) -> HashMap<u32, (f32, u64)> {
        let mut map = HashMap::new();
        let Some(nvml) = self.nvml.as_ref() else {
            return map;
        };
        let Ok(count) = nvml.device_count() else {
            return map;
        };
        for i in 0..count {
            let Ok(device) = nvml.device_by_index(i) else {
                continue;
            };
            if let Ok(procs) = device.running_compute_processes() {
                for p in procs {
                    if let UsedGpuMemory::Used(bytes) = p.used_gpu_memory {
                        map.entry(p.pid).or_insert((0.0, 0)).1 += bytes;
                    }
                }
            }
            if let Ok(procs) = device.running_graphics_processes() {
                for p in procs {
                    if let UsedGpuMemory::Used(bytes) = p.used_gpu_memory {
                        map.entry(p.pid).or_insert((0.0, 0)).1 += bytes;
                    }
                }
            }
            if let Ok(stats) = device.process_utilization_stats(self.last_nvml_proc_ts) {
                for s in &stats {
                    map.entry(s.pid).or_insert((0.0, 0)).0 = s.sm_util as f32;
                    self.last_nvml_proc_ts = self.last_nvml_proc_ts.max(s.timestamp);
                }
            }
        }
        map
    }

    fn sample_nvml(&self) -> Option<GpuMetrics> {
        let nvml = self.nvml.as_ref()?;
        let count = nvml.device_count().ok()?;
        if count == 0 {
            return None;
        }
        let mut adapters = Vec::new();
        let mut primary: Option<GpuMetrics> = None;
        for i in 0..count {
            let Ok(device) = nvml.device_by_index(i) else {
                continue;
            };
            let name = device.name().unwrap_or_else(|_| "NVIDIA GPU".into());
            let util = device.utilization_rates().ok().map(|u| u.gpu as f32);
            let mem = device.memory_info().ok();
            let temp = device.temperature(TemperatureSensor::Gpu).ok().map(|t| t as f32);
            let core = device.clock_info(Clock::Graphics).ok();
            let mem_clock = device.clock_info(Clock::Memory).ok();
            let power = device.power_usage().ok().map(|mw| mw as f32 / 1000.0);
            let fan = device.fan_speed(0).ok();
            adapters.push(GpuAdapter {
                name: name.clone(),
                vendor: "NVIDIA".into(),
                utilization: util,
                memory_used_bytes: mem.as_ref().map(|m| m.used),
                memory_total_bytes: mem.as_ref().map(|m| m.total),
                temperature_c: temp,
            });
            let candidate = GpuMetrics {
                name,
                vendor: "NVIDIA".into(),
                utilization: util,
                memory_used_bytes: mem.as_ref().map(|m| m.used),
                memory_total_bytes: mem.as_ref().map(|m| m.total),
                temperature_c: temp,
                core_clock_mhz: core,
                memory_clock_mhz: mem_clock,
                power_w: power,
                fan_speed_percent: fan,
                driver_version: nvml.sys_driver_version().ok(),
                available: true,
                unavailable_reason: None,
                activity: gpu_activity(util.unwrap_or(0.0)),
                adapters: vec![],
            };
            let better = match &primary {
                None => true,
                Some(cur) => util.unwrap_or(0.0) >= cur.utilization.unwrap_or(0.0),
            };
            if better {
                primary = Some(candidate);
            }
        }
        let mut metrics = primary?;
        metrics.adapters = adapters;
        Some(metrics)
    }
}

fn sample_dxgi_fallback() -> GpuMetrics {
    match query_dxgi() {
        Some(adapters) if !adapters.is_empty() => {
            let main = adapters[0].clone();
            GpuMetrics {
                name: main.name.clone(),
                vendor: main.vendor.clone(),
                utilization: None,
                memory_used_bytes: main.memory_used_bytes,
                memory_total_bytes: main.memory_total_bytes,
                temperature_c: None,
                core_clock_mhz: None,
                memory_clock_mhz: None,
                power_w: None,
                fan_speed_percent: None,
                driver_version: None,
                available: main.memory_total_bytes.is_some(),
                unavailable_reason: Some(
                    "GPU utilization is unavailable on this system. VRAM totals come from DXGI.".into(),
                ),
                activity: "Utilization unavailable on this GPU vendor".into(),
                adapters,
            }
        }
        _ => GpuMetrics {
            name: "Unavailable on this system".into(),
            vendor: String::new(),
            available: false,
            unavailable_reason: Some(
                "No supported GPU telemetry interface was found (NVML/DXGI).".into(),
            ),
            activity: "Unavailable on this system".into(),
            ..Default::default()
        },
    }
}

fn query_dxgi() -> Option<Vec<GpuAdapter>> {
    use windows::core::Interface;
    use windows::Win32::Graphics::Dxgi::{
        CreateDXGIFactory1, IDXGIAdapter3, IDXGIFactory1, DXGI_MEMORY_SEGMENT_GROUP_LOCAL,
    };

    unsafe {
        let factory: IDXGIFactory1 = CreateDXGIFactory1().ok()?;
        let mut adapters = Vec::new();
        let mut index = 0;
        loop {
            let adapter = match factory.EnumAdapters1(index) {
                Ok(a) => a,
                Err(_) => break,
            };
            index += 1;
            let Ok(desc) = adapter.GetDesc1() else {
                continue;
            };
            if (desc.Flags as u32) & 2 != 0 {
                continue;
            }
            let name = String::from_utf16_lossy(&desc.Description)
                .trim_end_matches('\0')
                .to_string();
            if name.to_lowercase().contains("basic render") {
                continue;
            }
            let vendor = vendor_from_id(desc.VendorId);
            let mut used = None;
            let mut total = desc.DedicatedVideoMemory as u64;
            if let Ok(a3) = adapter.cast::<IDXGIAdapter3>() {
                let mut info = windows::Win32::Graphics::Dxgi::DXGI_QUERY_VIDEO_MEMORY_INFO::default();
                if a3
                    .QueryVideoMemoryInfo(0, DXGI_MEMORY_SEGMENT_GROUP_LOCAL, &mut info)
                    .is_ok()
                {
                    used = Some(info.CurrentUsage);
                    if info.Budget > total {
                        total = info.Budget;
                    }
                }
            }
            adapters.push(GpuAdapter {
                name,
                vendor,
                utilization: None,
                memory_used_bytes: used,
                memory_total_bytes: if total > 0 { Some(total) } else { None },
                temperature_c: None,
            });
        }
        if adapters.is_empty() {
            None
        } else {
            Some(adapters)
        }
    }
}

fn vendor_from_id(id: u32) -> String {
    match id {
        0x10DE => "NVIDIA".into(),
        0x1002 => "AMD".into(),
        0x8086 => "Intel".into(),
        _ => "Unknown".into(),
    }
}

fn gpu_activity(util: f32) -> String {
    if util < 8.0 {
        "Mostly idle".into()
    } else if util < 40.0 {
        "Light compositor / video work".into()
    } else if util < 80.0 {
        "Actively rendering".into()
    } else {
        "Saturated".into()
    }
}
