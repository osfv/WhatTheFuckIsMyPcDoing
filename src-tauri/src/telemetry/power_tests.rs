use super::*;

fn row(pid: u32, name: &str, cpu: f32, gpu: Option<f32>, disk_bps: u64) -> ProcessRow {
    ProcessRow {
        pid,
        name: name.into(),
        display_name: crate::analysis::display_name(name),
        cpu,
        gpu,
        disk_bps,
        ..Default::default()
    }
}

fn discharging() -> BatteryStatus {
    battery_status(&mock_battery("discharging").unwrap())
}

#[test]
fn power_status_without_battery() {
    let raw = from_power_status(1, 128, 255, u32::MAX);
    assert!(!raw.present);
    assert_eq!(raw.ac_online, Some(true));
    assert_eq!(raw.percent, None);
    let status = battery_status(&raw);
    assert_eq!(status.state, "none");
    assert_eq!(status.headline, "No battery on this PC");
}

#[test]
fn power_status_unknown_flag_is_not_a_battery() {
    assert!(!from_power_status(255, 255, 255, u32::MAX).present);
}

#[test]
fn power_status_discharging() {
    let raw = from_power_status(0, 1, 74, 7200);
    assert!(raw.present);
    assert_eq!(raw.charging, Some(false));
    assert_eq!(raw.percent, Some(74.0));
    let status = battery_status(&raw);
    assert_eq!(status.state, "discharging");
    assert_eq!(status.time_remaining_sec, Some(7200));
    assert!(
        status.summary.contains("Windows estimates about 2 h left"),
        "{}",
        status.summary
    );
}

#[test]
fn power_status_charging_flag() {
    let raw = from_power_status(1, 8 | 2, 30, u32::MAX);
    assert_eq!(raw.charging, Some(true));
    assert_eq!(raw.os_time_remaining_sec, None);
    assert_eq!(battery_status(&raw).state, "charging");
}

#[test]
fn wmi_fixture_two_packs_sums_capacity() {
    let raw = from_power_status(0, 1, 50, u32::MAX);
    let status = vec![
        WmiBatteryStatus {
            DischargeRate: Some(6000),
            RemainingCapacity: Some(20_000),
            ChargeRate: Some(0),
            ..Default::default()
        },
        WmiBatteryStatus {
            DischargeRate: Some(4000),
            RemainingCapacity: Some(5_000),
            ChargeRate: Some(0),
            ..Default::default()
        },
    ];
    let full = vec![
        WmiFullCapacity {
            FullChargedCapacity: Some(40_000),
        },
        WmiFullCapacity {
            FullChargedCapacity: Some(10_000),
        },
    ];
    let statics = vec![
        WmiStaticData {
            DesignedCapacity: Some(50_000),
            DeviceName: Some("  DELL 7FHHV  ".into()),
            ManufactureName: Some("SMP".into()),
        },
        WmiStaticData {
            DesignedCapacity: Some(12_500),
            DeviceName: None,
            ManufactureName: None,
        },
    ];
    let cycles = vec![
        WmiCycleCount {
            CycleCount: Some(0),
        },
        WmiCycleCount {
            CycleCount: Some(140),
        },
    ];
    let merged = merge_wmi(raw, &status, &full, &statics, &cycles);
    assert_eq!(merged.remaining_mwh, Some(25_000));
    assert_eq!(merged.discharge_rate_mw, Some(10_000));
    assert_eq!(merged.charge_rate_mw, None);
    assert_eq!(merged.full_mwh, Some(50_000));
    assert_eq!(merged.design_mwh, Some(62_500));
    assert_eq!(merged.cycle_count, Some(140));
    assert_eq!(merged.name.as_deref(), Some("DELL 7FHHV"));
    assert_eq!(merged.manufacturer.as_deref(), Some("SMP"));

    let s = battery_status(&merged);
    assert_eq!(s.state, "discharging");
    assert_eq!(s.rate_w, Some(10.0));
    assert_eq!(s.time_remaining_sec, Some(9000));
    assert_eq!(s.health_percent, Some(80.0));
    assert_eq!(s.health_label, "Healthy");
    assert!(s.summary.contains("2 h 30 min"), "{}", s.summary);
}

#[test]
fn wmi_rows_without_battery_are_ignored() {
    let raw = from_power_status(1, 128, 255, u32::MAX);
    let full = [WmiFullCapacity {
        FullChargedCapacity: Some(1),
    }];
    assert_eq!(merge_wmi(raw.clone(), &[], &full, &[], &[]), raw);
}

#[test]
fn wmi_fills_percent_and_ac_when_power_status_is_blank() {
    let raw = RawBattery::default();
    let status = [WmiBatteryStatus {
        RemainingCapacity: Some(15_000),
        PowerOnline: Some(true),
        Charging: Some(false),
        ..Default::default()
    }];
    let full = [WmiFullCapacity {
        FullChargedCapacity: Some(60_000),
    }];
    let merged = merge_wmi(raw, &status, &full, &[], &[]);
    assert!(merged.present);
    assert_eq!(merged.percent, Some(25.0));
    assert_eq!(battery_status(&merged).state, "plugged");
}

#[test]
fn worn_battery_and_time_to_full() {
    let raw = RawBattery {
        present: true,
        ac_online: Some(true),
        charging: Some(true),
        percent: Some(50.0),
        remaining_mwh: Some(20_000),
        full_mwh: Some(40_000),
        design_mwh: Some(60_000),
        charge_rate_mw: Some(40_000),
        ..Default::default()
    };
    let s = battery_status(&raw);
    assert_eq!(s.state, "charging");
    assert_eq!(s.time_to_full_sec, Some(1800));
    assert_eq!(s.time_remaining_sec, None);
    assert_eq!(s.health_label, "Worn");
    assert!(s.summary.contains("Full in about 30 min"), "{}", s.summary);
    assert!(s.summary.contains("holds 67%"), "{}", s.summary);
}

#[test]
fn plugged_in_full() {
    let raw = RawBattery {
        present: true,
        ac_online: Some(true),
        charging: Some(false),
        percent: Some(100.0),
        ..Default::default()
    };
    assert_eq!(battery_status(&raw).state, "full");
}

#[test]
fn mock_fixtures_parse() {
    assert!(!mock_battery("none").unwrap().present);
    let d = mock_battery("discharging").unwrap();
    assert!(d.mocked && d.present);
    assert_eq!(battery_status(&d).state, "discharging");
    assert_eq!(
        battery_status(&mock_battery("charging").unwrap()).state,
        "charging"
    );
    assert!(mock_battery("").is_none());
    let json = r#"{"present":true,"acOnline":false,"percent":12,"dischargeRateMw":9000,"remainingMwh":6000}"#;
    let parsed: RawBattery = serde_json::from_str(json).unwrap();
    let s = battery_status(&parsed);
    assert!(s.headline.starts_with("Battery is low"), "{}", s.headline);
    assert_eq!(s.time_remaining_sec, Some(2400));
}

#[test]
fn model_groups_chrome_and_ranks_it_first() {
    let rows = vec![
        row(10, "chrome.exe", 20.0, Some(10.0), 0),
        row(11, "chrome.exe", 15.0, None, 1024 * 1024),
        row(20, "Code.exe", 5.0, None, 0),
        row(0, "System Idle Process", 60.0, None, 0),
    ];
    let (apps, source) = estimate_apps(&rows, &Sensors::default(), &PowerModel::default());
    assert_eq!(source, "model");
    assert_eq!(apps.len(), 2);
    let report = build_report(battery_status(&RawBattery::default()), apps, source, None);
    let chrome = &report.apps[0];
    assert_eq!(chrome.key, "chrome");
    assert!((chrome.cpu_w - 5.25).abs() < 1e-3, "{}", chrome.cpu_w);
    assert!((chrome.gpu_w - 1.2).abs() < 1e-3);
    assert!((chrome.disk_w - 0.02).abs() < 1e-3);
    assert!(chrome.share > 0.8);
    assert!(
        report.headline.ends_with("is using the most power"),
        "{}",
        report.headline
    );
    assert!(
        report.headline.starts_with("Google Chrome"),
        "{}",
        report.headline
    );
}

#[test]
fn calibrated_against_battery_drain() {
    let rows = vec![
        row(10, "chrome.exe", 40.0, None, 0),
        row(20, "Code.exe", 10.0, None, 0),
    ];
    let sensors = Sensors {
        cpu_total_percent: 50.0,
        system_draw_w: Some(20.0),
        ..Default::default()
    };
    let (apps, source) = estimate_apps(&rows, &sensors, &PowerModel::default());
    assert_eq!(source, "calibrated");
    let total: f32 = apps.iter().map(AppEstimate::watts).sum();
    assert!((total - 17.0).abs() < 1e-3);
    let report = build_report(discharging(), apps, source, Some(20.0));
    assert_eq!(
        report.headline,
        "Google Chrome is the reason your battery drains"
    );
    assert!((report.apps[0].share - 0.8).abs() < 1e-3);
    assert!(report.summary.contains("80%"), "{}", report.summary);
    assert!(
        report.summary.contains("of your battery per hour"),
        "{}",
        report.summary
    );
    assert!((report.other_w.unwrap() - 3.0).abs() < 1e-3);
    let pph = report.apps[0].battery_percent_per_hour.unwrap();
    assert!((pph - 13.6 / 50.0 * 100.0).abs() < 0.01);
}

#[test]
fn sensors_are_never_scaled_up_by_drain() {
    let rows = vec![row(10, "chrome.exe", 10.0, None, 0)];
    let sensors = Sensors {
        cpu_total_percent: 10.0,
        cpu_package_w: Some(4.0),
        system_draw_w: Some(30.0),
        ..Default::default()
    };
    let (apps, source) = estimate_apps(&rows, &sensors, &PowerModel::default());
    assert_eq!(source, "calibrated");
    assert!((apps[0].watts() - 4.0).abs() < 1e-3);
}

#[test]
fn package_power_is_split_by_cpu_share() {
    let rows = vec![
        row(10, "game.exe", 30.0, Some(90.0), 0),
        row(20, "Code.exe", 10.0, None, 0),
    ];
    let sensors = Sensors {
        cpu_total_percent: 40.0,
        cpu_package_w: Some(20.0),
        gpu_w: Some(80.0),
        ..Default::default()
    };
    let (apps, source) = estimate_apps(&rows, &sensors, &PowerModel::default());
    assert_eq!(source, "sensors");
    let game = apps.iter().find(|a| a.key == "game").unwrap();
    assert!((game.cpu_w - 15.0).abs() < 1e-3);
    assert!((game.gpu_w - 80.0).abs() < 1e-3);
    let report = build_report(battery_status(&RawBattery::default()), apps, source, None);
    assert_eq!(report.apps[0].main_component, "GPU");
    assert_eq!(report.apps[0].battery_percent_per_hour, None);
}

#[test]
fn plugged_in_headline_mentions_wall_power() {
    let rows = vec![row(10, "chrome.exe", 60.0, None, 0)];
    let (apps, source) = estimate_apps(&rows, &Sensors::default(), &PowerModel::default());
    let battery = battery_status(&mock_battery("charging").unwrap());
    let report = build_report(battery, apps, source, None);
    assert_eq!(
        report.headline,
        "Google Chrome is the hungriest app, but you are plugged in"
    );
    assert!(!report.summary.contains("per hour"));
}

#[test]
fn quiet_system_headline() {
    let rows = vec![row(10, "explorer.exe", 1.0, None, 0)];
    let (apps, source) = estimate_apps(&rows, &Sensors::default(), &PowerModel::default());
    let report = build_report(discharging(), apps, source, None);
    assert_eq!(report.headline, "Nothing is draining your battery hard");
}

#[test]
fn spread_drain_headline() {
    let rows = vec![
        row(1, "a.exe", 20.0, None, 0),
        row(2, "b.exe", 20.0, None, 0),
        row(3, "c.exe", 20.0, None, 0),
    ];
    let (apps, source) = estimate_apps(&rows, &Sensors::default(), &PowerModel::default());
    let report = build_report(discharging(), apps, source, None);
    assert_eq!(report.headline, "Battery drain is spread across a few apps");
}

#[test]
fn smoother_eases_and_drops_gone_apps() {
    let mk = |w: f32| AppEstimate {
        key: "x".into(),
        name: "X".into(),
        pid: 1,
        cpu_w: w,
        gpu_w: 0.0,
        disk_w: 0.0,
        cpu: 0.0,
        gpu: 0.0,
        disk_bps: 0,
    };
    let mut s = PowerSmoother::default();
    assert_eq!(s.push(vec![mk(0.0)])[0].cpu_w, 0.0);
    let eased = s.push(vec![mk(8.0)])[0].cpu_w;
    assert!((eased - 2.0).abs() < 1e-3);
    let mut left = s.push(vec![]);
    assert!((left[0].cpu_w - 1.5).abs() < 1e-3);
    for _ in 0..40 {
        left = s.push(vec![]);
    }
    assert!(left.is_empty());
}
