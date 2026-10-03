import { useEffect, type ComponentType } from "react";
import { AnimatePresence, MotionConfig, motion, useReducedMotion } from "motion/react";
import { TitleBar } from "./components/layout/TitleBar";
import { Sidebar } from "./components/layout/Sidebar";
import { OverviewPage } from "./pages/OverviewPage";
import { ProcessesPage } from "./pages/ProcessesPage";
import { PerformancePage } from "./pages/PerformancePage";
import { NetworkPage } from "./pages/NetworkPage";
import { StoragePage } from "./pages/StoragePage";
import { HardwarePage } from "./pages/HardwarePage";
import { BatteryPage } from "./pages/BatteryPage";
import { ActivityPage } from "./pages/ActivityPage";
import { AlertsPage } from "./pages/AlertsPage";
import { HistoryPage } from "./pages/HistoryPage";
import { SettingsPage } from "./pages/SettingsPage";
import { OnboardingPage } from "./pages/OnboardingPage";
import { CommandPalette } from "./components/CommandPalette";
import { ResourceBreakdown } from "./components/ResourceBreakdown";
import { getSettings, getSnapshot, onNavigate, onSettingsUpdated, onSnapshot } from "./lib/api";
import type { PageId } from "./lib/types";
import { useAppStore } from "./store/app-store";

const pages: Record<PageId, ComponentType> = {
  overview: OverviewPage,
  processes: ProcessesPage,
  performance: PerformancePage,
  network: NetworkPage,
  storage: StoragePage,
  hardware: HardwarePage,
  battery: BatteryPage,
  activity: ActivityPage,
  alerts: AlertsPage,
  history: HistoryPage,
  settings: SettingsPage,
};

export default function App() {
  const page = useAppStore((s) => s.page);
  const ready = useAppStore((s) => s.settings.onboardingComplete);
  const reduced = useAppStore((s) => s.settings.reducedMotion);
  const reduce = useReducedMotion();

  useEffect(() => {
    const { setSettings, setSnapshot, setPage } = useAppStore.getState();
    void getSettings().then(setSettings).catch(() => undefined);
    void getSnapshot().then(setSnapshot).catch(() => undefined);
    let unsubSnap: (() => void) | undefined;
    let unsubNav: (() => void) | undefined;
    let unsubSettings: (() => void) | undefined;
    void onSnapshot(setSnapshot).then((u) => {
      unsubSnap = u;
    });
    void onNavigate((p) => setPage(p as PageId)).then((u) => {
      unsubNav = u;
    });
    void onSettingsUpdated(setSettings).then((u) => {
      unsubSettings = u;
    });
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        useAppStore.getState().setPaletteOpen(true);
        return;
      }
      if (e.key === "Escape") {
        const s = useAppStore.getState();
        if (s.paletteOpen) {
          s.setPaletteOpen(false);
          return;
        }
        if (s.resourcePanel) {
          s.closeResourcePanel();
        }
      }
      if (e.key === "/" && !(e.target instanceof HTMLInputElement) && !(e.target instanceof HTMLTextAreaElement)) {
        e.preventDefault();
        setPage("processes");
      }
      if (e.key >= "1" && e.key <= "9" && (e.ctrlKey || e.metaKey)) {
        const order: PageId[] = [
          "overview",
          "processes",
          "performance",
          "network",
          "storage",
          "hardware",
          "activity",
          "alerts",
          "history",
        ];
        const next = order[Number(e.key) - 1];
        if (next) setPage(next);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      unsubSnap?.();
      unsubNav?.();
      unsubSettings?.();
      window.removeEventListener("keydown", onKey);
    };
  }, []);

  return (
    <MotionConfig reducedMotion={reduced ? "always" : "user"}>
      {ready ? <Dashboard page={page} reduce={!!reduce} /> : <Welcome />}
      <ResourceBreakdown />
    </MotionConfig>
  );
}

function Welcome() {
  return (
    <div className="app-shell onboarding">
      <TitleBar />
      <main className="main">
        <OnboardingPage />
      </main>
    </div>
  );
}

function Dashboard({ page, reduce }: { page: PageId; reduce: boolean }) {
  const Page = pages[page];
  return (
    <div className="app-shell">
      <TitleBar />
      <Sidebar />
      <main className="main">
        <AnimatePresence mode="wait">
          <motion.div
            key={page}
            className="h-full min-h-0"
            initial={reduce ? false : { opacity: 0, y: 16, scale: 0.985 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={reduce ? undefined : { opacity: 0, y: -12, scale: 0.99 }}
            transition={{ duration: reduce ? 0 : 0.32, ease: [0.16, 1, 0.3, 1] }}
            transformTemplate={(t, generated) =>
              Number(t.y) === 0 && (t.scale === 1 || t.scale === undefined) ? "none" : generated
            }
          >
            <Page />
          </motion.div>
        </AnimatePresence>
      </main>
      <CommandPalette />
    </div>
  );
}
