pub struct Hint {
    pub why: &'static str,
    pub normal: &'static str,
    pub benign: bool,
}

pub fn hint_for(name: &str) -> Option<Hint> {
    let key = crate::analysis::normalize_name(name);
    Some(match key.as_str() {
        "searchindexer" | "searchprotocolhost" | "searchfilterhost" | "searchhost" => Hint {
            why: "Windows Search is indexing recently changed files.",
            normal: "Periodic disk activity from Search is expected, especially after installs or large file copies.",
            benign: true,
        },
        "msmpeng" | "antimalware service executable" | "nissrv" | "securityhealthservice" => Hint {
            why: "Microsoft Defender is scanning files or keeping real-time protection running.",
            normal: "Defender regularly uses CPU and disk. That is the point of the scanner.",
            benign: true,
        },
        "chrome" | "msedge" | "firefox" | "brave" | "opera" | "vivaldi" => Hint {
            why: "Browsers keep a process per tab, extension and GPU helper. Memory use adds up fast.",
            normal: "Dozens of child processes are expected behavior, not a virus.",
            benign: true,
        },
        "code" | "cursor" | "devenv" | "idea64" | "pycharm64" | "webstorm64" => Hint {
            why: "Language servers, extensions and file watchers sit beside the editor UI.",
            normal: "Editors spawn helpers. High RAM is common on large projects.",
            benign: true,
        },
        "discord" | "slack" | "teams" | "ms-teams" | "telegram" => Hint {
            why: "Chat apps keep a renderer, GPU process and voice engine resident.",
            normal: "Idle CPU should stay low. Spikes during calls or updates are normal.",
            benign: true,
        },
        "onedrive" | "dropbox" | "googledrivesync" | "box.exe" => Hint {
            why: "Cloud sync uploads and downloads changed files in the background.",
            normal: "Large uploads after you add files are expected.",
            benign: true,
        },
        "steam" | "steamwebhelper" | "epicgameslauncher" | "origin" | "eadesktop" => Hint {
            why: "Game launchers update titles and stay resident for overlay and chat.",
            normal: "Downloads can saturate disk and network. Idle should be quiet.",
            benign: true,
        },
        "dwm" => Hint {
            why: "Desktop Window Manager composites the Windows desktop and GPU desktop effects.",
            normal: "DWM should stay modest unless a full-screen app or broken GPU driver is involved.",
            benign: true,
        },
        "explorer" => Hint {
            why: "Windows Explorer is the shell: taskbar, desktop and file windows.",
            normal: "One explorer.exe is expected. Spikes happen when opening heavy folders.",
            benign: true,
        },
        "svchost" => Hint {
            why: "Service Host is a shared process for many Windows services. The name alone is not a diagnosis.",
            normal: "Multiple svchost.exe instances are required on modern Windows.",
            benign: true,
        },
        "system" | "registry" | "secure system" | "memory compression" => Hint {
            why: "This is a kernel-side Windows component, not a regular app.",
            normal: "You cannot end it. Activity here usually reflects work requested by other processes.",
            benign: true,
        },
        "csrss" | "smss" | "wininit" | "services" | "lsass" | "winlogon" => Hint {
            why: "Core Windows process. If it is running from System32, leave it alone.",
            normal: "These should never run from Downloads, Temp, or AppData.",
            benign: true,
        },
        "tiworker" | "trustedinstaller" | "usocoreworker" | "moosetcoreworker" | "wuauclt" => Hint {
            why: "Windows Update is installing or staging patches.",
            normal: "CPU and disk will rise during updates, then settle.",
            benign: true,
        },
        "runtimebroker" | "dllhost" | "conhost" | "fontdrvhost" | "sihost" => Hint {
            why: "Windows helper that brokers app permissions, COM, or console I/O.",
            normal: "Short bursts are normal. Sustained high load is worth inspecting.",
            benign: true,
        },
        "nvidia web helper" | "nvcontainer" | "nvdisplay.container" | "nvidia share" => Hint {
            why: "NVIDIA container for driver services, overlay or broadcast.",
            normal: "Should be quiet unless recording or changing driver settings.",
            benign: true,
        },
        "audiodg" => Hint {
            why: "Windows audio engine. Effects and sample conversion run here.",
            normal: "CPU use rises with exclusive-mode devices or heavy EQ.",
            benign: true,
        },
        "valorant-win64-shipping" | "cs2" | "fortniteclient-win64-shipping" | "overwatch" | "gta5" | "r5apex" => Hint {
            why: "A game is using the GPU. That is what the GPU is for.",
            normal: "High GPU and fan speed while playing is expected, not a fault.",
            benign: true,
        },
        _ => return None,
    })
}

pub fn is_known_benign(name: &str) -> bool {
    hint_for(name).map(|h| h.benign).unwrap_or(false)
}

pub fn is_game(name: &str) -> bool {
    let key = crate::analysis::normalize_name(name);
    matches!(
        key.as_str(),
        "valorant-win64-shipping"
            | "cs2"
            | "fortniteclient-win64-shipping"
            | "overwatch"
            | "gta5"
            | "r5apex"
            | "rdr2"
            | "witcher3"
            | "cyberpunk2077"
            | "eldenring"
            | "leagueclient"
            | "dota2"
            | "hl2"
            | "hlvr"
            | "wow"
            | "wowclassic"
            | "destiny2"
            | "rocketleague"
            | "rainbowsix"
            | "cod.exe"
            | "modernwarfare"
            | "starfield"
            | "baldursgate3"
            | "bg3"
            | "factorio"
            | "rustclient"
            | "pubg"
            | "tslgame"
    ) || key.ends_with("-win64-shipping")
        || key.ends_with("_win64_shipping")
}

pub fn display_name(raw: &str) -> String {
    let key = crate::analysis::normalize_name(raw);
    match key.as_str() {
        "chrome" | "chromium" => "Google Chrome".into(),
        "msedge" => "Microsoft Edge".into(),
        "msedgewebview2" => "Edge WebView".into(),
        "firefox" => "Firefox".into(),
        "brave" => "Brave".into(),
        "opera" => "Opera".into(),
        "vivaldi" => "Vivaldi".into(),
        "cursor" => "Cursor".into(),
        "code" => "Visual Studio Code".into(),
        "devenv" => "Visual Studio".into(),
        "idea64" => "IntelliJ IDEA".into(),
        "pycharm64" => "PyCharm".into(),
        "webstorm64" => "WebStorm".into(),
        "discord" => "Discord".into(),
        "slack" => "Slack".into(),
        "teams" | "ms-teams" => "Microsoft Teams".into(),
        "telegram" => "Telegram".into(),
        "onedrive" => "OneDrive".into(),
        "dropbox" => "Dropbox".into(),
        "googledrivesync" => "Google Drive".into(),
        "steam" => "Steam".into(),
        "steamwebhelper" => "Steam".into(),
        "epicgameslauncher" => "Epic Games Launcher".into(),
        "explorer" => "Windows Explorer".into(),
        "dwm" => "Desktop Window Manager".into(),
        "svchost" => "Service Host".into(),
        "searchindexer" | "searchhost" => "Windows Search".into(),
        "msmpeng" => "Antimalware Service Executable".into(),
        "securityhealthservice" => "Windows Security".into(),
        "runtimebroker" => "Runtime Broker".into(),
        "dllhost" => "COM Surrogate".into(),
        "conhost" => "Console Window Host".into(),
        "sihost" => "Shell Infrastructure Host".into(),
        "csrss" => "Client Server Runtime".into(),
        "lsass" => "Local Security Authority".into(),
        "services" => "Services and Controller".into(),
        "winlogon" => "Windows Logon".into(),
        "wininit" => "Windows Start-Up".into(),
        "system" => "System".into(),
        "registry" => "Registry".into(),
        "memory compression" => "Memory Compression".into(),
        "audiodg" => "Windows Audio Device Graph".into(),
        "nvidia web helper" | "nvcontainer" | "nvdisplay.container" => "NVIDIA Container".into(),
        "tiworker" | "trustedinstaller" => "Windows Modules Installer".into(),
        other => titleize(other),
    }
}

fn titleize(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => name.to_string(),
    }
}

pub struct PathClass {
    pub suspicious: bool,
}

pub fn classify_path(path: &str, name: &str) -> PathClass {
    let p = path.to_lowercase();
    let n = crate::analysis::normalize_name(name);
    let system_names = [
        "svchost", "lsass", "csrss", "smss", "wininit", "services", "winlogon", "explorer",
        "dwm", "taskhostw", "dllhost",
    ];
    if system_names.contains(&n.as_str()) {
        let ok = p.contains(r"\windows\system32\")
            || p.contains(r"\windows\syswow64\")
            || p.contains(r"\windows\explorer.exe")
            || p.contains(r"\windows\systemapps\");
        return PathClass { suspicious: !ok };
    }
    PathClass { suspicious: false }
}
