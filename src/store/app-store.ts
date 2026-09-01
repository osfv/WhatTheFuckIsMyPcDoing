import { create } from "zustand";
import type { AppSettings, PageId, ProcessDetails, Snapshot } from "../lib/types";
import { defaultSettings } from "../lib/types";

export type NavMetric = "cpu" | "gpu" | "memory" | "disk" | "network" | "storage";
export type ResourcePanelKind = "cpu" | "memory" | "gpu" | "disk";

export interface NavFocus {
  processQuery?: string;
  pid?: number | null;
  metric?: NavMetric;
  eventId?: string | null;
  alertId?: number | null;
  settingsSection?: string;
}

interface AppStore {
  page: PageId;
  snapshot: Snapshot | null;
  settings: AppSettings;
  selectedPid: number | null;
  inspector: ProcessDetails | null;
  query: string;
  processGroup: "grouped" | "tree" | "flat";
  processFilter: "all" | "apps" | "system";
  reducedMotion: boolean;
  focus: NavFocus;
  paletteOpen: boolean;
  resourcePanel: ResourcePanelKind | null;
  setPage: (page: PageId, focus?: NavFocus) => void;
  openResourcePanel: (kind: ResourcePanelKind) => void;
  closeResourcePanel: () => void;
  toggleResourcePanel: (kind: ResourcePanelKind) => void;
  setSnapshot: (snapshot: Snapshot) => void;
  setSettings: (settings: AppSettings) => void;
  setSelectedPid: (pid: number | null) => void;
  setInspector: (details: ProcessDetails | null) => void;
  setQuery: (query: string) => void;
  setProcessGroup: (mode: "grouped" | "tree" | "flat") => void;
  setProcessFilter: (filter: "all" | "apps" | "system") => void;
  setPaletteOpen: (open: boolean) => void;
  acknowledgeLocal: (id: number) => void;
  acknowledgeAllLocal: () => void;
}

export const useAppStore = create<AppStore>((set) => ({
  page: "overview",
  snapshot: null,
  settings: defaultSettings(),
  selectedPid: null,
  inspector: null,
  query: "",
  processGroup: "grouped",
  processFilter: "all",
  reducedMotion: false,
  focus: {},
  paletteOpen: false,
  resourcePanel: null,
  openResourcePanel: (resourcePanel) => set({ resourcePanel }),
  closeResourcePanel: () => set({ resourcePanel: null }),
  toggleResourcePanel: (kind) =>
    set((s) => ({ resourcePanel: s.resourcePanel === kind ? null : kind })),
  setPage: (page, focus = {}) =>
    set((s) => ({
      page,
      focus,
      query: focus.processQuery ?? s.query,
      selectedPid: page === "processes" ? (focus.pid ?? s.selectedPid) : null,
      inspector: page === "processes" ? s.inspector : null,
    })),
  setSnapshot: (snapshot) => set({ snapshot }),
  setSettings: (settings) => {
    const next = { ...defaultSettings(), ...settings };
    document.documentElement.style.setProperty("--accent", next.accentColor);
    document.documentElement.classList.toggle("reduce-motion", next.reducedMotion);
    set({ settings: next, reducedMotion: next.reducedMotion });
  },
  setSelectedPid: (selectedPid) =>
    set((s) => ({
      selectedPid,
      inspector: selectedPid == null ? null : s.inspector,
    })),
  setInspector: (inspector) => set({ inspector }),
  setQuery: (query) => set({ query }),
  setProcessGroup: (processGroup) => set({ processGroup }),
  setProcessFilter: (processFilter) => set({ processFilter }),
  setPaletteOpen: (paletteOpen) => set({ paletteOpen }),
  acknowledgeLocal: (id) =>
    set((s) => {
      if (!s.snapshot) return s;
      return {
        snapshot: {
          ...s.snapshot,
          alerts: s.snapshot.alerts.map((a) => (a.id === id ? { ...a, acknowledged: true } : a)),
        },
      };
    }),
  acknowledgeAllLocal: () =>
    set((s) => {
      if (!s.snapshot) return s;
      return {
        snapshot: {
          ...s.snapshot,
          alerts: s.snapshot.alerts.map((a) => ({ ...a, acknowledged: true })),
        },
      };
    }),
}));

export function goToCategory(category: string, process?: string | null) {
  const { setPage } = useAppStore.getState();
  if (process) {
    setPage("processes", { processQuery: process });
    return;
  }
  switch (category) {
    case "cpu":
    case "gpu":
    case "thermal":
      setPage("performance", { metric: category === "thermal" ? "cpu" : category });
      break;
    case "memory":
      setPage("performance", { metric: "memory" });
      break;
    case "disk":
      setPage("performance", { metric: "disk" });
      break;
    case "storage":
      setPage("storage", { metric: "storage" });
      break;
    case "network":
      setPage("network", { metric: "network" });
      break;
    case "security":
      setPage("processes");
      break;
    default:
      setPage("activity");
  }
}
