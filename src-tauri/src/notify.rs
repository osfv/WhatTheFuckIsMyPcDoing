#[cfg(windows)]
use std::os::windows::process::CommandExt;

fn sanitize(value: &str) -> String {
    value
        .chars()
        .filter(|c| *c != '<' && *c != '>' && *c != '"' && *c != '\'' && *c != '`')
        .take(180)
        .collect()
}

pub fn toast(title: &str, body: &str) {
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let title = sanitize(title);
        let body = sanitize(body);
        if title.is_empty() {
            return;
        }
        let script = format!(
            r#"
[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom, ContentType = WindowsRuntime] | Out-Null
$xml = New-Object Windows.Data.Xml.Dom.XmlDocument
$xml.LoadXml(@"
<toast><visual><binding template='ToastGeneric'><text>{title}</text><text>{body}</text></binding></visual></toast>
"@)
$toast = [Windows.UI.Notifications.ToastNotification]::new($xml)
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('What The F*ck Is My PC Doing').Show($toast)
"#
        );
        let _ = std::process::Command::new("powershell")
            .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = (title, body);
    }
}

pub fn should_notify(enabled: bool, min_severity: &str, severity: &str) -> bool {
    if !enabled {
        return false;
    }
    match min_severity {
        "off" => false,
        "critical" => severity == "critical",
        "warning" => severity == "critical" || severity == "warning",
        _ => severity == "critical" || severity == "warning",
    }
}
