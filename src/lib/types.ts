export type PageId =
  | "overview"
  | "processes"
  | "performance"
  | "network"
  | "storage"
  | "hardware"
  | "activity"
  | "alerts"
  | "history"
  | "settings";

export interface Snapshot {
  timestamp: number;
  paused: boolean;
  cpu: CpuMetrics;
  gpu: GpuMetrics;
  memory: MemoryMetrics;
  disks: DiskMetrics[];
  network: NetworkMetrics;
  processes: ProcessRow[];
  topProcesses: ProcessRow[];
  interpretation: Interpretation;
  health: HealthSummary;
  events: ActivityEvent[];
  alerts: Alert[];
  history: HistoryWindow;
}

export interface HistoryWindow {
  cpu: number[];
  gpu: number[];
  memory: number[];
  disk: number[];
  networkDown: number[];
  networkUp: number[];
}

export interface CpuMetrics {
  utilization: number;
  perCore: number[];
  frequencyMhz: number | null;
  brand: string;
  coreCount: number;
  logicalCount: number;
  temperatureC: number | null;
  packagePowerW: number | null;
  activity: string;
}

export interface GpuMetrics {
  name: string;
  vendor: string;
  utilization: number | null;
  memoryUsedBytes: number | null;
  memoryTotalBytes: number | null;
  temperatureC: number | null;
  coreClockMhz: number | null;
  memoryClockMhz: number | null;
  powerW: number | null;
  fanSpeedPercent: number | null;
  driverVersion: string | null;
  available: boolean;
  unavailableReason: string | null;
  activity: string;
  adapters: GpuAdapter[];
}

export interface GpuAdapter {
  name: string;
  vendor: string;
  utilization: number | null;
  memoryUsedBytes: number | null;
  memoryTotalBytes: number | null;
  temperatureC: number | null;
}

export interface MemoryMetrics {
  usedBytes: number;
  totalBytes: number;
  availableBytes: number;
  committedBytes: number | null;
  cachedBytes: number | null;
  swapUsedBytes: number;
  swapTotalBytes: number;
  utilization: number;
  activity: string;
}

export interface DiskMetrics {
  name: string;
  mount: string;
  kind: string;
  fs: string;
  totalBytes: number;
  usedBytes: number;
  availableBytes: number;
  utilization: number;
  readBps: number;
  writeBps: number;
  activeTimePercent: number | null;
  responseMs: number | null;
  queueLength: number | null;
  removable: boolean;
  activity: string;
}

export interface NetworkMetrics {
  downloadBps: number;
  uploadBps: number;
  interfaces: NetInterface[];
  connectionCount: number;
  activity: string;
  explanation: string;
}

export interface NetInterface {
  name: string;
  downloadBps: number;
  uploadBps: number;
  totalReceived: number;
  totalTransmitted: number;
}

export interface ProcessRow {
  pid: number;
  parentPid: number | null;
  name: string;
  displayName: string;
  cpu: number;
  memoryBytes: number;
  gpu: number | null;
  gpuMemoryBytes: number | null;
  gpuEngine: string | null;
  diskBps: number;
  diskReadBps: number;
  diskWriteBps: number;
  connectionCount: number;
  path: string | null;
  status: string;
  startTime: number;
  runTimeSec: number;
}

export interface ProcessDetails {
  process: ProcessRow;
  publisher: string | null;
  product: string | null;
  fileVersion: string | null;
  hasSignature: boolean;
  signatureStatus: string;
  commandLine: string | null;
  children: ProcessRow[];
  connections: NetConnection[];
  hint: string | null;
  expected: boolean;
  groupCount: number;
  groupMemoryBytes: number;
  groupCpu: number;
}

export interface NetConnection {
  protocol: string;
  local: string;
  remote: string;
  state: string;
  pid: number;
  processName: string;
}

export interface Interpretation {
  headline: string;
  summary: string;
  mood: string;
  pressure: ResourcePressure;
}

export interface ResourcePressure {
  cpu: string;
  gpu: string;
  memory: string;
  disk: string;
  network: string;
  thermal: string;
}

export interface HealthSummary {
  score: number;
  label: string;
  summary: string;
  reasons: string[];
}

export interface ActivityEvent {
  id: string;
  timestamp: number;
  title: string;
  body: string;
  why: string | null;
  impact: string;
  recommendation: string | null;
  category: string;
  kind: string;
  process: string | null;
  severity: string;
}

export interface Alert {
  id: number;
  timestamp: number;
  title: string;
  body: string;
  severity: string;
  category: string;
  acknowledged: boolean;
}

export interface HardwareInfo {
  cpu: HwDevice | null;
  gpus: HwDevice[];
  motherboard: HwDevice | null;
  memoryModules: HwDevice[];
  storage: HwDevice[];
  networkAdapters: HwDevice[];
  monitors: HwDevice[];
  osName: string;
  osVersion: string;
  hostname: string;
  uptimeSec: number;
}

export interface HwDevice {
  name: string;
  manufacturer: string | null;
  detail: string;
  extras: HwField[];
}

export interface HwField {
  label: string;
  value: string;
}

export interface HistoryPoint {
  timestamp: number;
  cpu: number;
  gpu: number;
  memory: number;
  disk: number;
  networkDown: number;
  networkUp: number;
}

export interface AppSettings {
  onboardingComplete: boolean;
  startOnLogin: boolean;
  minimizeToTray: boolean;
  sampleIntervalMs: number;
  historyRetentionDays: number;
  notificationsEnabled: boolean;
  notifyMinSeverity: string;
  accentColor: string;
  temperatureUnit: string;
  reducedMotion: boolean;
  pauseMonitoring: boolean;
  monitorCpu: boolean;
  monitorGpu: boolean;
  monitorMemory: boolean;
  monitorDisk: boolean;
  monitorNetwork: boolean;
  monitorProcesses: boolean;
  cpuAlertPercent: number;
  cpuAlertSeconds: number;
  gpuTempAlertC: number;
  gpuTempAlertSeconds: number;
  ramAlertPercent: number;
  ramAlertSeconds: number;
  diskFullPercent: number;
  discordPresence: boolean;
  discordApplicationId: string;
  githubUrl: string;
}

export const defaultSettings = (): AppSettings => ({
  onboardingComplete: false,
  startOnLogin: false,
  minimizeToTray: true,
  sampleIntervalMs: 1000,
  historyRetentionDays: 7,
  notificationsEnabled: true,
  notifyMinSeverity: "warning",
  accentColor: "#FF6B57",
  temperatureUnit: "celsius",
  reducedMotion: false,
  pauseMonitoring: false,
  monitorCpu: true,
  monitorGpu: true,
  monitorMemory: true,
  monitorDisk: true,
  monitorNetwork: true,
  monitorProcesses: true,
  cpuAlertPercent: 90,
  cpuAlertSeconds: 300,
  gpuTempAlertC: 85,
  gpuTempAlertSeconds: 480,
  ramAlertPercent: 92,
  ramAlertSeconds: 120,
  diskFullPercent: 90,
  discordPresence: false,
  discordApplicationId: "",
  githubUrl: "",
});
