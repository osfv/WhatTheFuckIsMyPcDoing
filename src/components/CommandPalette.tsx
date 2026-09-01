import { useEffect, useMemo, useRef, useState } from "react";
import type { PageId } from "../lib/types";
import { useAppStore } from "../store/app-store";

const pages: { id: PageId; label: string; hint: string }[] = [
  { id: "overview", label: "Overview", hint: "What the PC is doing" },
  { id: "processes", label: "Processes", hint: "Who is using CPU and RAM" },
  { id: "performance", label: "Performance", hint: "Charts and cores" },
  { id: "network", label: "Network", hint: "Down, up, sockets" },
  { id: "storage", label: "Storage", hint: "Capacity and disk activity" },
  { id: "hardware", label: "Hardware", hint: "Inventory" },
  { id: "activity", label: "Activity", hint: "What happened" },
  { id: "alerts", label: "Alerts", hint: "Sustained problems" },
  { id: "history", label: "History", hint: "Earlier load" },
  { id: "settings", label: "Settings", hint: "Local preferences" },
];

const settingJumps = [
  { id: "general", label: "Settings: General" },
  { id: "monitoring", label: "Settings: Monitoring" },
  { id: "alerts", label: "Settings: Alerts" },
  { id: "history", label: "Settings: History" },
  { id: "appearance", label: "Settings: Appearance" },
  { id: "discord", label: "Settings: Discord" },
  { id: "privacy", label: "Settings: Privacy" },
  { id: "advanced", label: "Settings: Advanced" },
];

export function CommandPalette() {
  const open = useAppStore((s) => s.paletteOpen);
  const setOpen = useAppStore((s) => s.setPaletteOpen);
  const setPage = useAppStore((s) => s.setPage);
  const processes = useAppStore((s) => s.snapshot?.processes);
  const [q, setQ] = useState("");
  const [active, setActive] = useState(0);
  const input = useRef<HTMLInputElement>(null);

  const items = useMemo(() => {
    const needle = q.trim().toLowerCase();
    const nav = pages
      .filter((p) => !needle || `${p.label} ${p.hint}`.toLowerCase().includes(needle))
      .map((p) => ({
        key: `page-${p.id}`,
        label: p.label,
        hint: p.hint,
        run: () => setPage(p.id),
      }));
    const settings = settingJumps
      .filter((s) => !needle || s.label.toLowerCase().includes(needle) || needle.startsWith("set"))
      .map((s) => ({
        key: `set-${s.id}`,
        label: s.label,
        hint: "Jump to section",
        run: () => setPage("settings", { settingsSection: s.id }),
      }));
    const procs = (processes ?? [])
      .filter((p) => {
        if (!needle) return false;
        return `${p.displayName} ${p.name} ${p.pid}`.toLowerCase().includes(needle);
      })
      .slice(0, 8)
      .map((p) => ({
        key: `proc-${p.pid}`,
        label: p.displayName || p.name,
        hint: `PID ${p.pid}`,
        run: () => setPage("processes", { pid: p.pid, processQuery: p.displayName || p.name }),
      }));
    return [...nav, ...settings, ...procs];
  }, [q, processes, setPage]);

  useEffect(() => {
    if (!open) return;
    setQ("");
    setActive(0);
    const t = window.setTimeout(() => input.current?.focus(), 20);
    return () => window.clearTimeout(t);
  }, [open]);

  useEffect(() => {
    setActive(0);
  }, [q]);

  useEffect(() => {
    setActive((i) => Math.min(i, Math.max(0, items.length - 1)));
  }, [items]);

  if (!open) return null;

  const run = (index: number) => {
    const item = items[index];
    if (!item) return;
    item.run();
    setOpen(false);
  };

  return (
    <div
      className="palette-backdrop"
      data-tauri-drag-region="false"
      onClick={() => setOpen(false)}
      role="presentation"
    >
      <div
        className="palette card"
        role="dialog"
        aria-label="Command palette"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={(e) => {
          if (e.key === "ArrowDown") {
            e.preventDefault();
            setActive((i) => Math.min(items.length - 1, i + 1));
          }
          if (e.key === "ArrowUp") {
            e.preventDefault();
            setActive((i) => Math.max(0, i - 1));
          }
          if (e.key === "Enter") {
            e.preventDefault();
            run(active);
          }
          if (e.key === "Escape") setOpen(false);
        }}
      >
        <input
          ref={input}
          className="field"
          placeholder="Go to a page, setting, or process"
          value={q}
          onChange={(e) => setQ(e.target.value)}
          aria-label="Command search"
        />
        <ul className="palette-list">
          {items.map((item, i) => (
            <li key={item.key}>
              <button
                className={`palette-item ${i === active ? "active" : ""}`}
                onMouseEnter={() => setActive(i)}
                onClick={() => run(i)}
              >
                <span>{item.label}</span>
                <span className="text-ink-mute text-[12px]">{item.hint}</span>
              </button>
            </li>
          ))}
          {items.length === 0 && (
            <li className="text-[13px] text-ink-soft px-3 py-4">Nothing matches.</li>
          )}
        </ul>
        <p className="text-[11px] text-ink-mute px-1 pt-2">Ctrl+K. Esc closes.</p>
      </div>
    </div>
  );
}
