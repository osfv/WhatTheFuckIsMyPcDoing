export function formatBytes(bytes: number, digits = 1): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  let value = bytes / 1024 ** i;
  if (value >= 1000 && i < units.length - 1) {
    i += 1;
    value = bytes / 1024 ** i;
  }
  const d = i === 0 ? 0 : value >= 100 ? 0 : value >= 10 ? 1 : digits;
  const shown = i === 0 ? String(Math.round(bytes)) : value.toFixed(d);
  return `${shown} ${units[i]}`;
}

export function formatRate(bps: number): string {
  if (!Number.isFinite(bps) || bps <= 0) return "0 B/s";
  return `${formatBytes(bps, bps >= 1024 * 1024 ? 1 : 0)}/s`;
}

export function formatPercent(value: number | null | undefined, digits = 0): string {
  if (value == null || Number.isNaN(value)) return "Unavailable on this system";
  return `${value.toFixed(digits)}%`;
}

export function formatMhz(value: number | null | undefined): string {
  if (value == null || value <= 0) return "Unavailable on this system";
  if (value >= 1000) return `${(value / 1000).toFixed(2)} GHz`;
  return `${Math.round(value)} MHz`;
}

export function formatTemp(celsius: number | null | undefined, unit: string): string {
  if (celsius == null) return "Unavailable on this system";
  if (unit === "fahrenheit") return `${Math.round((celsius * 9) / 5 + 32)}°F`;
  return `${Math.round(celsius)}°C`;
}

export function formatDuration(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return "0s";
  if (seconds < 60) return `${Math.round(seconds)}s`;
  const m = Math.floor(seconds / 60);
  if (m < 60) return `${m}m`;
  const h = Math.floor(m / 60);
  const rm = m % 60;
  if (h < 48) return rm ? `${h}h ${rm}m` : `${h}h`;
  return `${Math.floor(h / 24)}d`;
}

export function formatClock(ts: number): string {
  const ms = ts > 1e12 ? ts : ts * 1000;
  return new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

export function formatDateTime(ts: number): string {
  const ms = ts > 1e12 ? ts : ts * 1000;
  return new Date(ms).toLocaleString([], {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function processGroupKey(name: string): string {
  return name.replace(/\.(exe|EXE)$/g, "").toLowerCase().trim();
}

export function processLabel(p: { name: string; displayName?: string | null }, count = 1): string {
  const raw = p.displayName?.trim();
  const base = raw && raw.length > 0 ? raw : p.name.replace(/\.exe$/i, "");
  return count > 1 ? `${base} (${count})` : base;
}

export function pressureLabel(value: string): string {
  switch (value) {
    case "normal":
      return "Normal";
    case "elevated":
      return "Elevated";
    case "high":
      return "High";
    case "critical":
      return "Critical";
    default:
      return "Unknown";
  }
}

export function whyLabel(why: string | null | undefined): string {
  if (!why || why === "Unable to determine the exact cause.") return "Possible cause";
  return "Why";
}

export function safeNum(value: number | null | undefined, fallback = 0): number {
  return value == null || Number.isNaN(value) ? fallback : value;
}
