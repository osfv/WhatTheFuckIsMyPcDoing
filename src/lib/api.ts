import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppSettings,
  HardwareInfo,
  HistoryPoint,
  NetConnection,
  ProcessDetails,
  Snapshot,
  ActivityEvent,
  Alert,
} from "./types";

export async function getSnapshot(): Promise<Snapshot> {
  return invoke("get_snapshot");
}

export async function getSettings(): Promise<AppSettings> {
  return invoke("get_settings");
}

export async function saveSettings(settings: AppSettings): Promise<AppSettings> {
  return invoke("save_app_settings", { settings });
}

export async function completeOnboarding(settings: AppSettings): Promise<AppSettings> {
  return invoke("complete_onboarding", { settings });
}

export async function getHardware(): Promise<HardwareInfo> {
  return invoke("get_hardware");
}

export async function getConnections(): Promise<NetConnection[]> {
  return invoke("get_connections");
}

export async function getProcessDetails(pid: number): Promise<ProcessDetails> {
  return invoke("get_process_details", { pid });
}

export async function killProcess(pid: number): Promise<void> {
  return invoke("kill_process", { pid });
}

export async function revealPath(path: string): Promise<void> {
  return invoke("reveal_path", { path });
}

export async function searchWeb(query: string): Promise<void> {
  return invoke("search_web", { query });
}

export async function getHistory(hours: number): Promise<HistoryPoint[]> {
  return invoke("get_history", { hours });
}

export async function getEventHistory(hours: number): Promise<ActivityEvent[]> {
  return invoke("get_event_history", { hours });
}

export async function getAlerts(): Promise<Alert[]> {
  return invoke("get_alerts");
}

export async function acknowledgeAlert(id: number): Promise<void> {
  return invoke("acknowledge_alert", { id });
}

export async function acknowledgeAllAlerts(): Promise<void> {
  return invoke("acknowledge_all_alerts");
}

export async function pauseMonitoring(paused: boolean): Promise<AppSettings> {
  return invoke("pause_monitoring", { paused });
}

export function onSnapshot(handler: (snapshot: Snapshot) => void): Promise<UnlistenFn> {
  return listen<Snapshot>("snapshot", (event) => handler(event.payload));
}

export function onNavigate(handler: (page: string) => void): Promise<UnlistenFn> {
  return listen<string>("navigate", (event) => handler(event.payload));
}

export function onSettingsUpdated(handler: (settings: AppSettings) => void): Promise<UnlistenFn> {
  return listen<AppSettings>("settings-updated", (event) => handler(event.payload));
}
