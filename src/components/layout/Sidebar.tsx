import { LayoutGroup, motion, useReducedMotion } from "motion/react";
import {
  ListChecks,
  BatteryHigh,
  Bell,
  Cpu,
  GearSix,
  HardDrives,
  Heartbeat,
  Network,
  Pulse,
  SquaresFour,
  ClockCounterClockwise,
} from "@phosphor-icons/react";
import type { IconProps } from "@phosphor-icons/react";
import type { ComponentType } from "react";
import type { PageId } from "../../lib/types";
import { useAppStore } from "../../store/app-store";

const items: { id: PageId; label: string; icon: ComponentType<IconProps> }[] = [
  { id: "overview", label: "Overview", icon: SquaresFour },
  { id: "processes", label: "Processes", icon: Pulse },
  { id: "performance", label: "Performance", icon: Cpu },
  { id: "network", label: "Network", icon: Network },
  { id: "storage", label: "Storage", icon: HardDrives },
  { id: "hardware", label: "Hardware", icon: Heartbeat },
  { id: "battery", label: "Battery", icon: BatteryHigh },
  { id: "activity", label: "Activity", icon: ListChecks },
  { id: "alerts", label: "Alerts", icon: Bell },
  { id: "history", label: "History", icon: ClockCounterClockwise },
];

export function Sidebar() {
  const page = useAppStore((s) => s.page);
  const setPage = useAppStore((s) => s.setPage);
  const unread = useAppStore((s) => unreadCount(s.snapshot?.alerts));

  return (
    <aside className="sidebar">
      <div className="flex items-center gap-2 px-2 mb-5">
        <Logo />
        <div className="brand-text leading-tight">
          <div className="text-[13px] font-semibold tracking-tight">WTF Is My PC Doing</div>
          <div className="text-[11px] text-ink-mute">Local. Private. Honest.</div>
        </div>
      </div>
      <LayoutGroup id="nav">
      <nav className="flex flex-col gap-0.5" aria-label="Primary">
        {items.map((item) => {
          const Icon = item.icon;
          const active = page === item.id;
          return (
            <button
              key={item.id}
              className={`nav-btn ${active ? "active" : ""}`}
              onClick={() => setPage(item.id)}
              aria-current={active ? "page" : undefined}
              title={item.label}
            >
              {active && (
                <motion.span
                  layoutId="nav-active"
                  className="absolute inset-0 rounded-[8px]"
                  style={{
                    background: "color-mix(in srgb, var(--accent) 16%, transparent)",
                    boxShadow: "inset 2px 0 0 var(--accent)",
                  }}
                  transition={{ type: "spring", stiffness: 380, damping: 32 }}
                />
              )}
              <span className="relative z-10 flex items-center gap-2.5 flex-1 min-w-0">
                <Icon size={18} weight={active ? "fill" : "regular"} />
                <span className="nav-label flex-1 text-left">{item.label}</span>
                {item.id === "alerts" && unread > 0 && (
                  <span className="nav-badge chip bad">{unread}</span>
                )}
              </span>
            </button>
          );
        })}
      </nav>
      <div className="mt-auto pt-3">
        <button
          className={`nav-btn ${page === "settings" ? "active" : ""}`}
          onClick={() => setPage("settings")}
          title="Settings"
        >
          {page === "settings" && (
            <motion.span
              layoutId="nav-active"
              className="absolute inset-0 rounded-[8px]"
              style={{
                background: "color-mix(in srgb, var(--accent) 16%, transparent)",
                boxShadow: "inset 2px 0 0 var(--accent)",
              }}
              transition={{ type: "spring", stiffness: 380, damping: 32 }}
            />
          )}
          <span className="relative z-10 flex items-center gap-2.5">
            <GearSix size={18} weight={page === "settings" ? "fill" : "regular"} />
            <span className="nav-label">Settings</span>
          </span>
        </button>
      </div>
      </LayoutGroup>
    </aside>
  );
}

function unreadCount(alerts: { acknowledged: boolean }[] | undefined) {
  if (!alerts) return 0;
  let n = 0;
  for (const a of alerts) {
    if (!a.acknowledged) n += 1;
  }
  return n;
}

function Logo() {
  const reduce = useReducedMotion();
  return (
    <svg width="28" height="28" viewBox="0 0 28 28" aria-hidden>
      <rect width="28" height="28" rx="8" fill="#0E0F12" />
      <motion.path
        d="M4 14 H9.2 L11.4 8 L15.6 20 L17.6 14 H24"
        fill="none"
        stroke="#FF6B57"
        strokeWidth="2.2"
        strokeLinecap="round"
        strokeLinejoin="round"
        initial={false}
        animate={reduce ? { opacity: 1 } : { opacity: [0.7, 1, 0.7] }}
        transition={{ duration: 1.6, repeat: Infinity, ease: "easeInOut" }}
      />
    </svg>
  );
}
