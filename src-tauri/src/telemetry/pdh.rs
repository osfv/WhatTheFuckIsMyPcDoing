//! Windows Performance Data Helper counters for disk busy/queue/response and ACPI thermal zones.

use std::collections::HashMap;

use windows::core::w;
use windows::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
    PdhOpenQueryW, PDH_CSTATUS_NEW_DATA, PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE_ITEM_W,
    PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_INSUFFICIENT_BUFFER, PDH_MORE_DATA,
};

const PDH_OK: u32 = 0;

#[derive(Clone, Debug, Default)]
pub struct DiskPerf {
    pub active_time_percent: Option<f32>,
    pub queue_length: Option<f32>,
    pub response_ms: Option<f32>,
    pub read_bps: Option<u64>,
    pub write_bps: Option<u64>,
}

#[derive(Clone, Debug, Default)]
pub struct PdhSample {
    pub disks: HashMap<String, DiskPerf>,
    pub thermal_c: Option<f32>,
}

struct Wildcard {
    idle: PDH_HCOUNTER,
    queue: PDH_HCOUNTER,
    sec_per_xfer: PDH_HCOUNTER,
    read: PDH_HCOUNTER,
    write: PDH_HCOUNTER,
    thermal: Option<PDH_HCOUNTER>,
}

pub struct PdhSampler {
    query: PDH_HQUERY,
    counters: Option<Wildcard>,
    primed: bool,
}

unsafe impl Send for PdhSampler {}

impl PdhSampler {
    pub fn new() -> Option<Self> {
        let query = open_query()?;
        let mut sampler = Self {
            query,
            counters: None,
            primed: false,
        };
        if let Err(err) = sampler.install() {
            tracing::warn!("PDH counters not installed: {err}");
            return None;
        }
        Some(sampler)
    }

    fn install(&mut self) -> Result<(), String> {
        let idle = add_counter(self.query, w!(r"\LogicalDisk(*)\% Idle Time"))
            .ok_or_else(|| "idle counter".to_string())?;
        let queue = add_counter(self.query, w!(r"\LogicalDisk(*)\Avg. Disk Queue Length"))
            .ok_or_else(|| "queue counter".to_string())?;
        let sec_per_xfer = add_counter(self.query, w!(r"\LogicalDisk(*)\Avg. Disk sec/Transfer"))
            .ok_or_else(|| "response counter".to_string())?;
        let read = add_counter(self.query, w!(r"\LogicalDisk(*)\Disk Read Bytes/sec"))
            .ok_or_else(|| "read counter".to_string())?;
        let write = add_counter(self.query, w!(r"\LogicalDisk(*)\Disk Write Bytes/sec"))
            .ok_or_else(|| "write counter".to_string())?;
        let thermal = add_counter(
            self.query,
            w!(r"\Thermal Zone Information(*)\High Precision Temperature"),
        );
        if thermal.is_some() {
            tracing::info!("PDH thermal zone counters are present");
        }
        self.counters = Some(Wildcard {
            idle,
            queue,
            sec_per_xfer,
            read,
            write,
            thermal,
        });
        Ok(())
    }

    pub fn sample(&mut self) -> PdhSample {
        let Some(counters) = self.counters.as_ref() else {
            return PdhSample::default();
        };
        let status = unsafe { PdhCollectQueryData(self.query) };
        if status != PDH_OK {
            return PdhSample::default();
        }
        if !self.primed {
            self.primed = true;
            return PdhSample::default();
        }

        let idle = by_drive(read_array(counters.idle));
        let queue = by_drive(read_array(counters.queue));
        let sec_per_xfer = by_drive(read_array(counters.sec_per_xfer));
        let read = by_drive(read_array(counters.read));
        let write = by_drive(read_array(counters.write));

        let mut disks = HashMap::new();
        for (key, idle_pct) in idle {
            let active = Some((100.0 - idle_pct).clamp(0.0, 100.0) as f32);
            disks.insert(
                key.clone(),
                DiskPerf {
                    active_time_percent: active,
                    queue_length: queue.get(&key).copied().map(|v| v.max(0.0) as f32),
                    response_ms: sec_per_xfer.get(&key).copied().and_then(|sec| {
                        if (0.0..=30.0).contains(&sec) {
                            Some((sec * 1000.0) as f32)
                        } else {
                            None
                        }
                    }),
                    read_bps: read.get(&key).copied().map(|v| v.max(0.0) as u64),
                    write_bps: write.get(&key).copied().map(|v| v.max(0.0) as u64),
                },
            );
        }

        let mut thermal_c = None;
        if let Some(counter) = counters.thermal {
            for raw in read_array(counter).into_values() {
                if let Some(c) = tenths_kelvin_to_c(raw) {
                    thermal_c = Some(thermal_c.map_or(c, |prev: f32| prev.max(c)));
                }
            }
        }

        PdhSample { disks, thermal_c }
    }
}

impl Drop for PdhSampler {
    fn drop(&mut self) {
        if !self.query.is_invalid() {
            unsafe {
                PdhCloseQuery(self.query);
            }
            self.query = PDH_HQUERY::default();
        }
    }
}

fn open_query() -> Option<PDH_HQUERY> {
    let mut query = PDH_HQUERY::default();
    let status = unsafe { PdhOpenQueryW(windows::core::PCWSTR::null(), 0, &mut query) };
    if status == PDH_OK {
        Some(query)
    } else {
        tracing::warn!("PdhOpenQueryW failed: {status:#x}");
        None
    }
}

fn add_counter(query: PDH_HQUERY, path: windows::core::PCWSTR) -> Option<PDH_HCOUNTER> {
    let mut counter = PDH_HCOUNTER::default();
    let status = unsafe { PdhAddEnglishCounterW(query, path, 0, &mut counter) };
    if status == PDH_OK {
        Some(counter)
    } else {
        None
    }
}

fn read_array(counter: PDH_HCOUNTER) -> HashMap<String, f64> {
    unsafe {
        let mut buf_size = 0u32;
        let mut item_count = 0u32;
        let status = PdhGetFormattedCounterArrayW(
            counter,
            PDH_FMT_DOUBLE,
            &mut buf_size,
            &mut item_count,
            None,
        );
        if !pdh_size_status(status) || buf_size == 0 {
            return HashMap::new();
        }
        let words = (buf_size as usize + 7) / 8;
        let mut buf = vec![0u64; words];
        let status = PdhGetFormattedCounterArrayW(
            counter,
            PDH_FMT_DOUBLE,
            &mut buf_size,
            &mut item_count,
            Some(buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W),
        );
        if status != PDH_OK || item_count == 0 {
            return HashMap::new();
        }
        let items = std::slice::from_raw_parts(
            buf.as_ptr() as *const PDH_FMT_COUNTERVALUE_ITEM_W,
            item_count as usize,
        );
        let mut map = HashMap::new();
        for item in items {
            if item.FmtValue.CStatus != PDH_CSTATUS_VALID_DATA
                && item.FmtValue.CStatus != PDH_CSTATUS_NEW_DATA
            {
                continue;
            }
            let name = item.szName.to_string().unwrap_or_default();
            if name.is_empty() || name == "_Total" {
                continue;
            }
            let value = item.FmtValue.Anonymous.doubleValue;
            if value.is_finite() {
                map.insert(name, value);
            }
        }
        map
    }
}

fn pdh_size_status(status: u32) -> bool {
    status == PDH_OK || status == PDH_MORE_DATA || status == PDH_INSUFFICIENT_BUFFER
}

fn by_drive(map: HashMap<String, f64>) -> HashMap<String, f64> {
    let mut out = HashMap::new();
    for (instance, value) in map {
        if let Some(key) = drive_letter(&instance) {
            out.insert(key, value);
        }
    }
    out
}

fn drive_letter(instance: &str) -> Option<String> {
    let t = instance.trim();
    let b = t.as_bytes();
    if b.len() >= 2 && b[1] == b':' && b[0].is_ascii_alphabetic() {
        Some(format!("{}:", (b[0] as char).to_ascii_uppercase()))
    } else {
        None
    }
}

pub fn volume_key(mount: &str) -> String {
    drive_letter(mount).unwrap_or_else(|| mount.trim().trim_end_matches(['\\', '/']).to_string())
}

pub fn tenths_kelvin_to_c(raw: f64) -> Option<f32> {
    if !raw.is_finite() || raw <= 0.0 {
        return None;
    }
    if let Some(c) = plausible_cpu_temp(((raw / 10.0) - 273.15) as f32) {
        return Some(c);
    }
    if let Some(c) = plausible_cpu_temp((raw / 10.0) as f32) {
        return Some(c);
    }
    plausible_cpu_temp(raw as f32)
}

pub fn plausible_cpu_temp(c: f32) -> Option<f32> {
    (5.0..=120.0).contains(&c).then_some(c)
}
