import { useState } from "react";
import { motion } from "motion/react";
import { Sparkline } from "../components/ui/Sparkline";
import { AnimatedNumber } from "../components/ui/AnimatedNumber";
import { Rise } from "../components/ui/motion";
import { formatBytes, formatRate, formatTemp, pressureLabel, whyLabel } from "../lib/format";
import { goToCategory, useAppStore } from "../store/app-store";
import type { NavMetric } from "../store/app-store";

export function OverviewPage() {
  const snapshot = useAppStore((s) => s.snapshot);
  const unit = useAppStore((s) => s.settings.temperatureUnit);
  const setPage = useAppStore((s) => s.setPage);
  const [healthOpen, setHealthOpen] = useState(false);
  if (!snapshot?.cpu || !snapshot.interpretation || !snapshot.health) return <Booting />;

  const { interpretation, health, events = [], cpu, gpu, memory, disks = [], network, history, topProcesses = [] } =
    snapshot;
  const diskIo = disks.reduce((sum, d) => sum + d.readBps + d.writeBps, 0);
  const sysDisk = disks.find((d) => d.mount.startsWith("C:")) ?? disks[0];
  const diskRead = disks.reduce((sum, d) => sum + d.readBps, 0);
  const diskWrite = disks.reduce((sum, d) => sum + d.writeBps, 0);
  const vramPct =
    gpu.memoryUsedBytes != null && gpu.memoryTotalBytes
      ? (gpu.memoryUsedBytes / gpu.memoryTotalBytes) * 100
      : null;

  return (
    <div className="flex flex-col gap-5 max-w-[1400px]">
      <Rise>
        <section className="card p-6 md:p-7 overflow-hidden relative">
          <div className="hero-wash" aria-hidden />
          <div className="flex flex-wrap items-start justify-between gap-4 relative">
            <div className="max-w-3xl">
              <motion.h1
                key={interpretation.headline}
                initial={{ opacity: 0, y: 8 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.35, ease: [0.16, 1, 0.3, 1] }}
                className="text-[32px] md:text-[40px] font-semibold tracking-tight leading-[1.1] pb-1"
              >
                {interpretation.headline || "Reading your machine..."}
              </motion.h1>
              <p className="mt-3 text-[15px] text-ink-soft max-w-[62ch] leading-relaxed">
                {interpretation.summary}
              </p>
            </div>
            <button
              className="text-right no-drag bg-transparent border-0 p-0"
              onClick={() => setHealthOpen((v) => !v)}
              aria-expanded={healthOpen}
            >
              <div className={`chip ${healthTone(health.label)}`}>{health.label}</div>
              <AnimatedNumber value={health.score} className="num text-3xl mt-2 block" />
              <div className="text-xs text-ink-mute mt-1">System health. Click for why.</div>
            </button>
          </div>
          {healthOpen && (
            <div className="relative mt-4 rounded-[10px] border border-line bg-bg-elevated p-4 max-w-xl">
              <h2 className="text-[13px] font-semibold">How this score is calculated</h2>
              <p className="text-[13px] text-ink-soft mt-1 leading-relaxed">{health.summary}</p>
              <ul className="mt-3 flex flex-col gap-1.5">
                {health.reasons?.map((reason) => (
                  <li key={reason} className="text-[13px] text-ink-soft">
                    {reason}
                  </li>
                ))}
              </ul>
              <p className="text-[12px] text-ink-mute mt-3">
                Starts at 100. Capacity, heat, and sustained load take points off. A busy game is not treated like a stuck CPU.
              </p>
            </div>
          )}
        </section>
      </Rise>

      <section className="grid grid-cols-2 xl:grid-cols-3 2xl:grid-cols-6 gap-3">
        <MetricTile
          delay={0.02}
          label="CPU"
          value={cpu.utilization}
          suffix="%"
          detail={cpu.activity}
          extra={cpu.temperatureC != null ? `CPU ${formatTemp(cpu.temperatureC, unit)}` : cpu.brand}
          values={history?.cpu ?? []}
          tone={interpretation.pressure.cpu}
          onClick={() => useAppStore.getState().toggleResourcePanel("cpu")}
        />
        <MetricTile
          delay={0.06}
          label="GPU"
          value={gpu.utilization}
          suffix="%"
          fallback="Unavailable on this system"
          detail={
            gpu.temperatureC != null ? `${formatTemp(gpu.temperatureC, unit)} · ${gpu.activity}` : gpu.activity
          }
          extra={gpu.temperatureC != null ? `GPU ${formatTemp(gpu.temperatureC, unit)}` : gpu.name || "Unavailable on this system"}
          values={history.gpu}
          tone={interpretation.pressure.gpu}
          onClick={() => useAppStore.getState().toggleResourcePanel("gpu")}
        />
        <MetricTile
          delay={0.1}
          label="RAM"
          value={memory.utilization}
          suffix="%"
          detail={memory.activity}
          extra={`${formatBytes(memory.usedBytes)} of ${formatBytes(memory.totalBytes)}`}
          values={history?.memory ?? []}
          tone={interpretation.pressure.memory}
          onClick={() => useAppStore.getState().toggleResourcePanel("memory")}
        />
        <MetricTile
          delay={0.14}
          label="VRAM"
          value={vramPct}
          suffix="%"
          fallback="Unavailable on this system"
          detail={gpu.available ? gpu.name : "Unavailable on this system"}
          extra={
            gpu.memoryUsedBytes != null && gpu.memoryTotalBytes
              ? `${formatBytes(gpu.memoryUsedBytes)} of ${formatBytes(gpu.memoryTotalBytes)}`
              : gpu.unavailableReason ?? "Unavailable on this system"
          }
          values={history.gpu}
          tone={interpretation.pressure.gpu}
          onClick={() => useAppStore.getState().toggleResourcePanel("gpu")}
        />
        <MetricTile
          delay={0.18}
          label="Disk"
          display={formatRate(diskIo)}
          detail={sysDisk?.activity ?? "No volumes reported"}
          extra={`Read ${formatRate(diskRead)} · Write ${formatRate(diskWrite)}`}
          values={history.disk}
          tone={interpretation.pressure.disk}
          max={80}
          onClick={() => {
            const hasIo = (snapshot.processes ?? []).some((p) => p.diskBps > 0);
            setPage("storage", { metric: "disk" });
            if (hasIo) useAppStore.getState().openResourcePanel("disk");
          }}
        />
        <MetricTile
          delay={0.22}
          label="Network"
          display={formatRate(network.downloadBps + network.uploadBps)}
          detail={network.activity}
          extra={`Down ${formatRate(network.downloadBps)} · Up ${formatRate(network.uploadBps)}`}
          values={history.networkDown}
          tone={interpretation.pressure.network}
          max={20}
          onClick={() => setPage("network", { metric: "network" })}
        />
      </section>

      <section className="grid grid-cols-1 xl:grid-cols-[1.4fr_1fr] gap-3">
        <Rise delay={0.12} className="card p-5">
          <h2 className="text-[15px] font-semibold mb-3">What is happening</h2>
          <div className="flex flex-col gap-3">
            {events.slice(0, 5).map((event) => (
              <article
                key={event.id}
                className="rounded-[10px] border border-line bg-bg-elevated p-4 cursor-pointer"
                onClick={() => goToCategory(event.category, event.process)}
              >
                <div className="flex items-center justify-between gap-3">
                  <h3 className="font-medium">{event.title}</h3>
                  <span className={`chip ${sev(event.severity)}`}>{event.impact}</span>
                </div>
                <p className="mt-2 text-[13px] text-ink-soft leading-relaxed">{event.body}</p>
                {event.why && (
                  <p className="mt-2 text-[13px] text-ink">
                    <span className="text-ink-mute">{whyLabel(event.why)}. </span>
                    {event.why}
                  </p>
                )}
                {event.recommendation && (
                  <p className="mt-1 text-[13px] text-ink-soft">
                    <span className="text-ink-mute">Recommendation. </span>
                    {event.recommendation}
                  </p>
                )}
              </article>
            ))}
            {events.length === 0 && (
              <p className="text-ink-soft text-sm">Nothing interesting is happening right now.</p>
            )}
          </div>
        </Rise>
        <div className="flex flex-col gap-3">
          <Rise delay={0.16} className="card p-5">
            <h2 className="text-[15px] font-semibold mb-2">Health</h2>
            <p className="text-[13px] text-ink-soft leading-relaxed">{health.summary}</p>
            <ul className="mt-3 flex flex-col gap-1.5">
              {health.reasons?.map((reason) => (
                <li key={reason} className="text-[13px] text-ink-soft">
                  {reason}
                </li>
              ))}
            </ul>
            <div className="mt-4 grid grid-cols-2 gap-2 text-[12px]">
              {Object.entries(interpretation.pressure ?? {}).map(([k, v]) => (
                <button
                  key={k}
                  className="flex justify-between gap-2 bg-transparent border-0 p-0 text-inherit cursor-pointer"
                  onClick={() => jumpPressure(k, setPage)}
                >
                  <span className="capitalize text-ink-mute">{k}</span>
                  <span>{pressureLabel(v)}</span>
                </button>
              ))}
            </div>
          </Rise>
          <Rise delay={0.2} className="card p-5">
            <h2 className="text-[15px] font-semibold mb-3">Using the most right now</h2>
            <ul className="flex flex-col gap-2">
              {topProcesses.slice(0, 6).map((p) => (
                <li key={p.pid}>
                  <button
                    className="flex w-full items-center justify-between gap-3 text-[13px] bg-transparent border-0 p-0 text-left"
                    onClick={() => setPage("processes", { pid: p.pid, processQuery: p.displayName || p.name })}
                  >
                    <span className="truncate">{p.displayName || p.name}</span>
                    <span className="num text-ink-soft">
                      {Number.isFinite(p.cpu) ? p.cpu.toFixed(1) : "0.0"}% · {formatBytes(p.memoryBytes)}
                    </span>
                  </button>
                </li>
              ))}
              {topProcesses.length === 0 && (
                <li className="text-[13px] text-ink-soft">Nothing interesting is happening right now.</li>
              )}
            </ul>
          </Rise>
        </div>
      </section>
    </div>
  );
}

function jumpPressure(key: string, setPage: (page: "performance" | "storage" | "network", focus?: { metric?: NavMetric }) => void) {
  if (key === "disk") {
    setPage("storage", { metric: "disk" });
    const procs = useAppStore.getState().snapshot?.processes ?? [];
    if (procs.some((p) => p.diskBps > 0)) useAppStore.getState().openResourcePanel("disk");
    return;
  }
  if (key === "network") {
    setPage("network", { metric: "network" });
    return;
  }
  if (key === "thermal") {
    setPage("performance", { metric: "cpu" });
    useAppStore.getState().openResourcePanel("cpu");
    return;
  }
  if (key === "cpu" || key === "memory" || key === "gpu") {
    useAppStore.getState().toggleResourcePanel(key);
    return;
  }
  setPage("performance", { metric: key as NavMetric });
}

function MetricTile({
  label,
  value,
  suffix,
  fallback,
  display,
  detail,
  extra,
  values,
  tone,
  max,
  delay,
  onClick,
}: {
  label: string;
  value?: number | null;
  suffix?: string;
  fallback?: string;
  display?: string;
  detail: string;
  extra: string;
  values: number[];
  tone: string;
  max?: number;
  delay: number;
  onClick?: () => void;
}) {
  return (
    <Rise delay={delay}>
      <motion.article
        className="card p-4 min-h-[148px] flex flex-col cursor-pointer"
        whileHover={{ y: -3 }}
        transition={{ type: "spring", stiffness: 420, damping: 28 }}
        onClick={onClick}
        role={onClick ? "button" : undefined}
        tabIndex={onClick ? 0 : undefined}
        onKeyDown={(e) => {
          if (onClick && (e.key === "Enter" || e.key === " ")) {
            e.preventDefault();
            onClick();
          }
        }}
      >
        <div className="flex items-center justify-between">
          <span className="text-[12px] text-ink-mute">{label}</span>
          <span className={`chip ${toneChip(tone)}`}>{pressureLabel(tone)}</span>
        </div>
        <div className="num text-[26px] mt-2 tracking-tight min-h-[32px]">
          {display ??
            (value == null || Number.isNaN(value) ? (
              fallback
            ) : (
              <AnimatedNumber value={value} suffix={suffix} digits={suffix === "%" ? 0 : 1} />
            ))}
        </div>
        <p className="text-[12px] text-ink-soft mt-1 line-clamp-2 min-h-[32px]">{detail}</p>
        <div className="mt-auto pt-2">
          <Sparkline values={values ?? []} max={max} />
          <p className="text-[11px] text-ink-mute mt-1 truncate">{extra}</p>
        </div>
      </motion.article>
    </Rise>
  );
}

function Booting() {
  return (
    <Rise>
      <div className="card p-8 max-w-xl">
        <h1 className="text-3xl font-semibold tracking-tight">Looking around...</h1>
        <p className="mt-3 text-ink-soft">The first sample takes a second. After that this page will speak English.</p>
      </div>
    </Rise>
  );
}

function healthTone(label: string) {
  if (label === "Excellent" || label === "Good") return "good";
  if (label === "Needs Attention") return "warn";
  return "bad";
}
function sev(s: string) {
  if (s === "critical" || s === "warning") return "warn";
  if (s === "notice") return "info";
  return "";
}
function toneChip(tone: string) {
  if (tone === "normal") return "good";
  if (tone === "elevated") return "info";
  if (tone === "high") return "warn";
  if (tone === "critical") return "bad";
  return "";
}
