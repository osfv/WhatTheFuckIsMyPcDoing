import { useEffect, useMemo, useRef } from "react";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { Meter } from "./ui/Meter";
import { formatBytes, formatPercent, formatRate } from "../lib/format";
import { groupProcesses, type GroupedProcess } from "../lib/process-groups";
import type { Snapshot } from "../lib/types";
import { ease } from "./ui/motion";
import { useAppStore, type ResourcePanelKind } from "../store/app-store";

const LIMIT = 16;
const OTHER_FLOOR = 64 * 1024 * 1024;
const HOLD_MS = 5000;

const copy: Record<ResourcePanelKind, { title: string; hint: string; empty: string }> = {
  memory: {
    title: "What is using RAM",
    hint: "Grouped like Chrome. Click a row to inspect that process.",
    empty: "Nothing interesting is happening right now.",
  },
  cpu: {
    title: "What is using CPU",
    hint: "Grouped like Chrome. Click a row to inspect that process.",
    empty: "Nothing interesting is happening right now.",
  },
  gpu: {
    title: "What is using VRAM",
    hint: "Process VRAM reported by the GPU. Click a row to inspect it.",
    empty: "No process reported VRAM. The GPU still holds memory for the desktop compositor.",
  },
  disk: {
    title: "What is hitting disk",
    hint: "Live read and write from processes. Click a row to inspect that process.",
    empty: "Nothing interesting is happening right now.",
  },
};

export function ResourceBreakdown() {
  const kind = useAppStore((s) => s.resourcePanel);
  const close = useAppStore((s) => s.closeResourcePanel);
  const snapshot = useAppStore((s) => s.snapshot);
  const reduce = useReducedMotion();
  const hold = useRef(new Map<string, HeldRow>());
  const order = useRef<string[]>([]);
  const panelKind = useRef<ResourcePanelKind | null>(null);
  const seenKeys = useRef(new Set<string>());

  useEffect(() => {
    if (!kind) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        close();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [kind, close]);

  if (panelKind.current !== kind) {
    hold.current.clear();
    order.current = [];
    seenKeys.current.clear();
    panelKind.current = kind;
  }

  const model = useMemo(() => buildModel(kind, snapshot, hold.current, order.current), [kind, snapshot]);

  useEffect(() => {
    for (const row of model.rows) seenKeys.current.add(row.key);
  }, [model.rows]);

  return (
    <AnimatePresence>
      {kind && (
        <motion.div key={kind} className="resource-layer" role="presentation">
          <motion.button
            type="button"
            aria-label="Close resource panel"
            className="resource-backdrop"
            initial={reduce ? false : { opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={reduce ? undefined : { opacity: 0 }}
            transition={{ duration: reduce ? 0 : 0.22, ease }}
            onClick={close}
          />
          <motion.aside
            role="dialog"
            aria-modal="true"
            aria-labelledby="resource-panel-title"
            className="resource-drawer"
            initial={reduce ? false : { x: 28, opacity: 0 }}
            animate={{ x: 0, opacity: 1 }}
            exit={reduce ? undefined : { x: 24, opacity: 0 }}
            transition={{ duration: reduce ? 0 : 0.32, ease }}
          >
            <header className="flex items-start justify-between gap-3">
              <div className="min-w-0">
                <h2 id="resource-panel-title" className="text-[15px] font-semibold tracking-tight">
                  {model.title}
                </h2>
                <p className="text-[12px] text-ink-soft mt-1 leading-relaxed">{model.hint}</p>
              </div>
              <button
                type="button"
                className="resource-close no-drag"
                onClick={close}
                aria-label="Close"
              >
                <svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true">
                  <path d="M2.2 2.2l7.6 7.6M9.8 2.2l-7.6 7.6" stroke="#E8EAED" strokeWidth="1.75" strokeLinecap="round" fill="none" />
                </svg>
              </button>
            </header>

            <div className="mt-4">
              <div className="flex items-baseline justify-between gap-3">
                <span className="text-[12px] text-ink-mute">{model.headline}</span>
                <span className="num text-[13px] text-ink-soft">{model.totalLabel}</span>
              </div>
              {model.meter != null && <Meter value={model.meter} className="mt-2 h-2" />}
            </div>

            {model.unavailable ? (
              <p className="mt-5 text-[13px] text-ink-soft">Unavailable on this system</p>
            ) : (
              <ul className="mt-4 flex-1 flex flex-col gap-2 min-h-0 overflow-auto pr-1">
                {model.rows.map((row) => {
                  const enter = !reduce && !seenKeys.current.has(row.key);
                  return (
                    <motion.li
                      key={row.key}
                      initial={enter ? { opacity: 0, x: 8 } : false}
                      animate={{ opacity: 1, x: 0 }}
                      transition={{ duration: reduce ? 0 : 0.28, ease }}
                    >
                      <button
                        type="button"
                        className="w-full text-left bg-transparent border-0 p-0"
                        onClick={() => openProcess(row, kind)}
                      >
                        <span className="flex items-center justify-between gap-3 text-[13px]">
                          <span className="truncate">
                            {row.name}
                            {row.count > 1 ? ` (${row.count})` : ""}
                          </span>
                          <span className="num text-ink-soft shrink-0">{row.value}</span>
                        </span>
                        <Meter value={row.meter} className="mt-1 h-1.5" />
                      </button>
                    </motion.li>
                  );
                })}
                {model.other && (
                  <li className="flex justify-between gap-3 text-[13px] text-ink-soft pt-1">
                    <span>{model.other.label}</span>
                    <span className="num shrink-0">{model.other.value}</span>
                  </li>
                )}
                {model.rows.length === 0 && (
                  <li className="text-[13px] text-ink-soft">{model.empty}</li>
                )}
              </ul>
            )}
          </motion.aside>
        </motion.div>
      )}
    </AnimatePresence>
  );
}

function openProcess(row: GroupedProcess, kind: ResourcePanelKind) {
  const pid =
    kind === "cpu" ? row.cpuPid : kind === "gpu" ? row.gpuPid : kind === "disk" ? row.diskPid : row.memoryPid;
  useAppStore.getState().closeResourcePanel();
  useAppStore.getState().setPage("processes", { pid, processQuery: row.name });
}

interface RowView extends GroupedProcess {
  value: string;
  meter: number;
}

interface Candidate extends RowView {
  sortValue: number;
  qualified: boolean;
}

interface HeldRow {
  lastQual: number;
  row: RowView;
  sortValue: number;
}

function buildModel(
  kind: ResourcePanelKind | null,
  snapshot: Snapshot | null,
  hold: Map<string, HeldRow>,
  order: string[],
) {
  const processes = snapshot?.processes ?? [];
  const grouped = groupProcesses(processes);
  if (!kind) {
    return emptyModel();
  }

  if (kind === "memory") {
    const total = snapshot?.memory?.totalBytes ?? 0;
    const used = snapshot?.memory?.usedBytes ?? 0;
    const ranked: Candidate[] = [...grouped]
      .sort((a, b) => b.memoryBytes - a.memoryBytes)
      .map((g, i) => ({
        ...g,
        value: total > 0 ? `${formatBytes(g.memoryBytes)} · ${pct(g.memoryBytes, total)}` : formatBytes(g.memoryBytes),
        meter: total > 0 ? (g.memoryBytes / total) * 100 : 0,
        sortValue: g.memoryBytes,
        qualified: i < LIMIT,
      }));
    const rows = stabilizeRows(ranked, hold, order, 50 * 1024 * 1024);
    const listed = rows.reduce((s, r) => s + r.memoryBytes, 0);
    const otherBytes = Math.max(0, used - listed);
    return {
      ...copy.memory,
      headline: "Working set",
      totalLabel: total > 0 ? `${formatBytes(used)} of ${formatBytes(total)}` : formatBytes(used),
      meter: total > 0 ? (used / total) * 100 : null,
      rows,
      other:
        otherBytes > OTHER_FLOOR
          ? { label: "Windows cache and unlisted", value: formatBytes(otherBytes) }
          : null,
      unavailable: false,
    };
  }

  if (kind === "cpu") {
    const util = snapshot?.cpu?.utilization;
    const ranked: Candidate[] = [...grouped]
      .sort((a, b) => b.cpu - a.cpu)
      .map((g, i) => ({
        ...g,
        value: `${Number.isFinite(g.cpu) ? g.cpu.toFixed(1) : "0.0"}%`,
        meter: Math.max(0, Math.min(100, g.cpu)),
        sortValue: g.cpu,
        qualified: g.cpu > 0.05 && i < LIMIT,
      }));
    const rows = stabilizeRows(ranked, hold, order, 8);
    return {
      ...copy.cpu,
      headline: "Live share of the CPU",
      totalLabel: util != null && Number.isFinite(util) ? formatPercent(util) : "",
      meter: util != null && Number.isFinite(util) ? util : null,
      rows,
      other: null,
      unavailable: false,
    };
  }

  if (kind === "gpu") {
    const gpu = snapshot?.gpu;
    const vramRows = [...grouped]
      .filter((g) => g.gpuMemoryBytes > 0)
      .sort((a, b) => b.gpuMemoryBytes - a.gpuMemoryBytes);
    const utilRows = [...grouped].filter((g) => g.gpu > 0.05).sort((a, b) => b.gpu - a.gpu);
    const hasVram = vramRows.length > 0;
    const source = hasVram ? vramRows : [...grouped].sort((a, b) => b.gpu - a.gpu);
    const total = gpu?.memoryTotalBytes ?? 0;
    const used = gpu?.memoryUsedBytes ?? 0;
    const ranked: Candidate[] = source.map((g, i) => ({
      ...g,
      value: hasVram
        ? `${formatBytes(g.gpuMemoryBytes)}${g.gpu > 0 ? ` · ${g.gpu.toFixed(0)}%` : ""}`
        : `${g.gpu.toFixed(1)}%`,
      meter: hasVram && total > 0 ? (g.gpuMemoryBytes / total) * 100 : Math.max(0, Math.min(100, g.gpu)),
      sortValue: hasVram ? g.gpuMemoryBytes : g.gpu,
      qualified: i < LIMIT && (hasVram ? g.gpuMemoryBytes > 0 : g.gpu > 0.05),
    }));
    const rows = stabilizeRows(ranked, hold, order, hasVram ? 64 * 1024 * 1024 : 8);
    const listed = rows.reduce((s, r) => s + r.gpuMemoryBytes, 0);
    const otherBytes = Math.max(0, used - listed);
    const noGpuData = !hasVram && utilRows.length === 0 && rows.length === 0;
    const gpuMissing = Boolean(gpu && gpu.available === false && noGpuData);
    return {
      title: hasVram ? copy.gpu.title : "What is using GPU",
      hint: hasVram ? copy.gpu.hint : "Process GPU load. Click a row to inspect that process.",
      empty: gpuMissing ? "Unavailable on this system" : copy.gpu.empty,
      headline: hasVram ? "Process VRAM" : "GPU engine load",
      totalLabel:
        gpu?.memoryTotalBytes != null
          ? `${formatBytes(used)} of ${formatBytes(gpu.memoryTotalBytes)}`
          : gpu?.available === false
            ? "Unavailable on this system"
            : "",
      meter: total > 0 ? (used / total) * 100 : null,
      rows,
      other:
        hasVram && otherBytes > OTHER_FLOOR
          ? { label: "Windows, driver, and unlisted apps", value: formatBytes(otherBytes) }
          : null,
      unavailable: gpuMissing,
    };
  }

  const diskAll = [...grouped].sort((a, b) => b.diskBps - a.diskBps);
  const peak = Math.max(1, ...diskAll.map((g) => g.diskBps), 1);
  const ranked: Candidate[] = diskAll.map((g, i) => ({
    ...g,
    value: formatRate(g.diskBps),
    meter: (g.diskBps / peak) * 100,
    sortValue: g.diskBps,
    qualified: g.diskBps > 0 && i < LIMIT,
  }));
  const rows = stabilizeRows(ranked, hold, order, 2_000_000);
  return {
    ...copy.disk,
    headline: "Read plus write",
    totalLabel: formatRate(diskAll.reduce((s, r) => s + r.diskBps, 0)),
    meter: null as number | null,
    rows,
    other: null,
    unavailable: false,
  };
}

function stabilizeRows(
  ranked: Candidate[],
  hold: Map<string, HeldRow>,
  order: string[],
  margin: number,
): RowView[] {
  const now = Date.now();

  for (const c of ranked) {
    const prev = hold.get(c.key);
    if (c.qualified) {
      hold.set(c.key, { lastQual: now, row: viewOf(c), sortValue: c.sortValue });
    } else if (prev) {
      prev.row = viewOf(c);
      prev.sortValue = c.sortValue;
    }
  }

  for (const [key, held] of [...hold.entries()]) {
    if (now - held.lastQual >= HOLD_MS) hold.delete(key);
  }

  const visible = new Set(hold.keys());
  let next = order.filter((key) => visible.has(key));
  const newcomers = [...visible].filter((key) => !next.includes(key));
  newcomers.sort((a, b) => (hold.get(b)?.sortValue ?? 0) - (hold.get(a)?.sortValue ?? 0));

  if (next.length === 0) {
    next = newcomers;
  } else {
    next.push(...newcomers);
    next = nudgeOrder(next, hold, margin);
  }

  if (next.length > LIMIT) next = next.slice(0, LIMIT);
  order.length = 0;
  order.push(...next);

  return next.map((key) => hold.get(key)!.row);
}

function nudgeOrder(keys: string[], hold: Map<string, HeldRow>, margin: number): string[] {
  const next = [...keys];
  for (let i = 0; i < next.length - 1; i++) {
    const a = hold.get(next[i])?.sortValue ?? 0;
    const b = hold.get(next[i + 1])?.sortValue ?? 0;
    if (b > a + margin) {
      const tmp = next[i];
      next[i] = next[i + 1];
      next[i + 1] = tmp;
    }
  }
  return next;
}

function viewOf(c: Candidate): RowView {
  return c;
}

function emptyModel() {
  return {
    title: "",
    hint: "",
    empty: "",
    headline: "",
    totalLabel: "",
    meter: null as number | null,
    rows: [] as RowView[],
    other: null as { label: string; value: string } | null,
    unavailable: false,
  };
}

function pct(part: number, total: number): string {
  if (total <= 0) return "0%";
  const n = (part / total) * 100;
  return `${n >= 10 ? n.toFixed(0) : n.toFixed(1)}%`;
}
