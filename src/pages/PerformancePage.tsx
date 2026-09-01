import { useEffect, useMemo, useState } from "react";
import { AnimatePresence, motion } from "motion/react";
import { Meter } from "../components/ui/Meter";
import { AreaChart } from "../components/ui/Sparkline";
import { detectVendor, VendorMark } from "../components/ui/VendorMark";
import { getHistory } from "../lib/api";
import { formatBytes, formatMhz, formatPercent, formatRate, formatTemp, processGroupKey } from "../lib/format";
import type { HistoryPoint, ProcessRow } from "../lib/types";
import { useAppStore } from "../store/app-store";

const ranges = [
  { id: "5m", label: "5m", live: true, take: 300 },
  { id: "15m", label: "15m", live: true, take: 900 },
  { id: "1h", label: "1h", hours: 1 },
  { id: "6h", label: "6h", hours: 6 },
  { id: "24h", label: "24h", hours: 24 },
] as const;

export function PerformancePage() {
  const snapshot = useAppStore((s) => s.snapshot);
  const unit = useAppStore((s) => s.settings.temperatureUnit);
  const intervalMs = useAppStore((s) => s.settings.sampleIntervalMs);
  const metric = useAppStore((s) => s.focus.metric);
  const [range, setRange] = useState<(typeof ranges)[number]["id"]>("15m");
  const [stored, setStored] = useState<HistoryPoint[]>([]);
  const [vramOpen, setVramOpen] = useState(false);

  const selected = ranges.find((r) => r.id === range) ?? ranges[1];

  useEffect(() => {
    const selected = ranges.find((r) => r.id === range) ?? ranges[1];
    if ("live" in selected && selected.live) return;
    if (!("hours" in selected)) return;
    getHistory(selected.hours)
      .then(setStored)
      .catch(() => setStored([]));
  }, [range]);

  useEffect(() => {
    if (metric === "gpu") setVramOpen(true);
    if (metric === "cpu") useAppStore.getState().openResourcePanel("cpu");
    if (metric === "memory") useAppStore.getState().openResourcePanel("memory");
    if (metric === "disk") {
      const procs = useAppStore.getState().snapshot?.processes ?? [];
      if (procs.some((p) => p.diskBps > 0)) useAppStore.getState().openResourcePanel("disk");
    }
  }, [metric]);

  useEffect(() => {
    if (!metric) return;
    const id = metric === "storage" ? "perf-disk" : `perf-${metric}`;
    document.getElementById(id)?.scrollIntoView({ block: "start" });
  }, [metric]);

  const live = snapshot?.history;
  const series = useMemo(() => {
    if ("live" in selected && selected.live && live) {
      const take = selected.take;
      return {
        cpu: live.cpu.slice(-take),
        gpu: live.gpu.slice(-take),
        memory: live.memory.slice(-take),
        disk: live.disk.slice(-take),
        networkDown: live.networkDown.slice(-take),
        networkUp: live.networkUp.slice(-take),
        times: undefined as number[] | undefined,
        fromDb: false,
      };
    }
    return {
      cpu: stored.map((p) => p.cpu),
      gpu: stored.map((p) => p.gpu),
      memory: stored.map((p) => p.memory),
      disk: stored.map((p) => p.disk),
      networkDown: stored.map((p) => p.networkDown),
      networkUp: stored.map((p) => p.networkUp),
      times: stored.map((p) => p.timestamp),
      fromDb: true,
    };
  }, [selected, live, stored]);

  if (!snapshot) return <p className="text-ink-soft">Waiting for the first sample.</p>;
  const { cpu, gpu, memory, disks, network, processes } = snapshot;
  const vendor = detectVendor(gpu.vendor, gpu.name);
  const cpuVendor = detectVendor(cpu.brand);
  const vramUsers = groupVram(processes);
  const listedVram = vramUsers.reduce((sum, p) => sum + p.bytes, 0);
  const otherVram = Math.max(0, (gpu.memoryUsedBytes ?? 0) - listedVram);

  return (
    <div className="flex flex-col gap-5 max-w-[1400px]">
      <header className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Performance</h1>
          <p className="text-sm text-ink-soft mt-1">
            {series.fromDb
              ? "This range comes from stored samples, about one every 15 seconds. Empty stretches mean the app was not running."
              : "These charts are live samples from this session. Longer ranges use stored history."}
          </p>
        </div>
        <div className="flex gap-1">
          {ranges.map((r) => (
            <button key={r.id} className={`btn ${range === r.id ? "primary" : ""}`} onClick={() => setRange(r.id)}>
              {r.label}
            </button>
          ))}
        </div>
      </header>

      <section id="perf-cpu" className="card p-5 scroll-mt-4">
        <div className="flex items-start gap-3">
          {cpuVendor && <VendorMark vendor={cpuVendor} size={32} />}
          <div>
            <h2 className="font-semibold">
              <button
                type="button"
                className="bg-transparent border-0 p-0 text-inherit font-semibold underline decoration-white/15 underline-offset-2 hover:decoration-accent"
                onClick={() => useAppStore.getState().toggleResourcePanel("cpu")}
              >
                CPU
              </button>
            </h2>
            <p className="text-[13px] text-ink-soft mt-1">
              {cpu.brand || "Processor"} · {cpu.coreCount} cores / {cpu.logicalCount} threads · {formatMhz(cpu.frequencyMhz)}
              {cpu.temperatureC != null ? ` · ${formatTemp(cpu.temperatureC, unit)}` : ""}
            </p>
          </div>
        </div>
        <div className="mt-4">
          <AreaChart
            series={[{ name: "Utilization", values: series.cpu, color: "#ff6b57" }]}
            times={series.times}
            intervalMs={intervalMs}
          />
        </div>
        <div className="mt-4 grid gap-2 [grid-template-columns:repeat(auto-fill,minmax(6.5rem,1fr))]">
          {Array.from({ length: Math.max(cpu.logicalCount || 0, cpu.perCore.length) }, (_, i) => {
            const core = cpu.perCore[i];
            const pct = Number.isFinite(core) ? core : 0;
            return (
              <div key={i} className="bg-bg-elevated rounded-[8px] p-2 min-h-[64px] min-w-0">
                <div className="text-[11px] text-ink-mute">Core {i}</div>
                <div className="num text-sm">{pct.toFixed(0)}%</div>
                <Meter value={pct} className="mt-1" />
              </div>
            );
          })}
        </div>
        <dl className="mt-4 grid grid-cols-2 md:grid-cols-4 gap-3 text-[13px]">
          <KV k="Load" v={formatPercent(cpu.utilization)} />
          <KV k="Temperature" v={formatTemp(cpu.temperatureC, unit)} />
          <KV
            k="Package power"
            v={cpu.packagePowerW != null ? `${cpu.packagePowerW.toFixed(1)} W` : "Unavailable on this system"}
          />
          <KV k="Clock" v={formatMhz(cpu.frequencyMhz)} />
        </dl>
      </section>

      <section id="perf-gpu" className="card p-5 scroll-mt-4">
        <div className="flex items-start gap-3">
          {vendor && <VendorMark vendor={vendor} size={32} />}
          <div>
            <h2 className="font-semibold">
              <button
                type="button"
                className="bg-transparent border-0 p-0 text-inherit font-semibold underline decoration-white/15 underline-offset-2 hover:decoration-accent"
                onClick={() => useAppStore.getState().toggleResourcePanel("gpu")}
              >
                GPU
              </button>
            </h2>
            <p className="text-[13px] text-ink-soft mt-1">
              {gpu.name || "Unavailable on this system"}
              {gpu.temperatureC != null ? ` · GPU ${formatTemp(gpu.temperatureC, unit)}` : ""}
            </p>
          </div>
        </div>
        <div className="mt-4">
          <AreaChart
            series={[{ name: "Utilization", values: series.gpu, color: "#7eb6ff" }]}
            times={series.times}
            intervalMs={intervalMs}
          />
        </div>
        <dl className="mt-4 grid grid-cols-2 md:grid-cols-4 gap-3 text-[13px]">
          <KV k="Load" v={gpu.utilization != null ? formatPercent(gpu.utilization) : "Unavailable on this system"} />
          <KV k="Temperature" v={formatTemp(gpu.temperatureC, unit)} />
          <div>
            <dt className="text-ink-mute text-[11px]">VRAM</dt>
            <dd>
              <button
                type="button"
                className="text-left underline decoration-white/15 underline-offset-2 hover:decoration-accent"
                onClick={() => setVramOpen((open) => !open)}
              >
                {gpu.memoryUsedBytes != null && gpu.memoryTotalBytes
                  ? `${formatBytes(gpu.memoryUsedBytes)} / ${formatBytes(gpu.memoryTotalBytes)}`
                  : "Unavailable on this system"}
              </button>
            </dd>
          </div>
          <KV k="Core clock" v={formatMhz(gpu.coreClockMhz)} />
          <KV k="Memory clock" v={formatMhz(gpu.memoryClockMhz)} />
          <KV k="Power" v={gpu.powerW != null ? `${gpu.powerW.toFixed(0)} W` : "Unavailable on this system"} />
          <KV k="Fan" v={gpu.fanSpeedPercent != null ? `${gpu.fanSpeedPercent}%` : "Unavailable on this system"} />
          <KV k="Driver" v={gpu.driverVersion ?? "Unavailable on this system"} />
        </dl>
        <AnimatePresence>
          {vramOpen && (
            <motion.div
              initial={{ opacity: 0, height: 0 }}
              animate={{ opacity: 1, height: "auto" }}
              exit={{ opacity: 0, height: 0 }}
              transition={{ duration: 0.32, ease: [0.16, 1, 0.3, 1] }}
              className="overflow-hidden"
            >
              <div className="mt-4 rounded-[10px] border border-line bg-bg-elevated p-4">
                <div className="flex items-center justify-between gap-3">
                  <h3 className="text-[13px] font-semibold">What is using VRAM</h3>
                  <span className="text-[12px] text-ink-mute">
                    {gpu.memoryTotalBytes != null
                      ? `${formatBytes(gpu.memoryUsedBytes ?? 0)} of ${formatBytes(gpu.memoryTotalBytes)}`
                      : "Unavailable on this system"}
                  </span>
                </div>
                {gpu.memoryTotalBytes != null && (
                  <Meter
                    value={((gpu.memoryUsedBytes ?? 0) / gpu.memoryTotalBytes) * 100}
                    className="mt-3 h-2"
                  />
                )}
                <ul className="mt-3 flex flex-col gap-2">
                  {vramUsers.map((p, i) => (
                    <motion.li
                      key={p.key}
                      initial={{ opacity: 0, x: 8 }}
                      animate={{ opacity: 1, x: 0 }}
                      transition={{ delay: 0.03 * i, duration: 0.28, ease: [0.16, 1, 0.3, 1] }}
                    >
                      <button
                        type="button"
                        className="w-full text-left flex items-center justify-between gap-3 text-[13px]"
                        onClick={() => useAppStore.getState().setPage("processes", { pid: p.pid, processQuery: p.name })}
                      >
                        <span className="truncate">
                          {p.name}
                          {p.count > 1 ? `  (${p.count})` : ""}
                        </span>
                        <span className="num text-ink-soft shrink-0">
                          {formatBytes(p.bytes)}
                          {p.gpu > 0 ? ` · ${p.gpu.toFixed(0)}%` : ""}
                        </span>
                      </button>
                      {gpu.memoryTotalBytes != null && (
                        <Meter value={(p.bytes / gpu.memoryTotalBytes) * 100} className="mt-1 h-1.5" />
                      )}
                    </motion.li>
                  ))}
                  {otherVram > 64 * 1024 * 1024 && (
                    <li className="flex justify-between text-[13px] text-ink-soft">
                      <span>Windows, driver, and unlisted apps</span>
                      <span className="num">{formatBytes(otherVram)}</span>
                    </li>
                  )}
                  {vramUsers.length === 0 && (
                    <li className="text-[13px] text-ink-soft">
                      No process reported VRAM to NVML. The GPU still holds memory for the desktop compositor.
                    </li>
                  )}
                </ul>
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </section>

      <section className="grid md:grid-cols-2 gap-3">
        <div id="perf-memory" className="card p-5 scroll-mt-4">
          <h2 className="font-semibold">
            <button
              type="button"
              className="bg-transparent border-0 p-0 text-inherit font-semibold underline decoration-white/15 underline-offset-2 hover:decoration-accent"
              onClick={() => useAppStore.getState().toggleResourcePanel("memory")}
            >
              Memory
            </button>
          </h2>
          <AreaChart
            series={[{ name: "Used", values: series.memory, color: "#3ddc97" }]}
            times={series.times}
            intervalMs={intervalMs}
          />
          <dl className="mt-3 grid grid-cols-2 gap-2 text-[13px]">
            <div>
              <dt className="text-ink-mute text-[11px]">Used</dt>
              <dd>
                <button
                  type="button"
                  className="text-left underline decoration-white/15 underline-offset-2 hover:decoration-accent"
                  onClick={() => useAppStore.getState().toggleResourcePanel("memory")}
                >
                  {formatBytes(memory.usedBytes)}
                </button>
              </dd>
            </div>
            <KV k="Available" v={formatBytes(memory.availableBytes)} />
            <KV k="Committed" v={memory.committedBytes != null ? formatBytes(memory.committedBytes) : "Unavailable on this system"} />
            <KV k="Cached" v={memory.cachedBytes != null ? formatBytes(memory.cachedBytes) : "Unavailable on this system"} />
            <KV k="Swap used" v={formatBytes(memory.swapUsedBytes)} />
            <KV k="Swap total" v={formatBytes(memory.swapTotalBytes)} />
          </dl>
        </div>
        <div id="perf-network" className="card p-5 scroll-mt-4">
          <h2 className="font-semibold">Network</h2>
          <AreaChart
            series={[
              { name: "Down", values: series.networkDown, color: "#7eb6ff" },
              { name: "Up", values: series.networkUp, color: "#f0b429" },
            ]}
            unit=" MB/s"
            max={Math.max(1, ...series.networkDown, ...series.networkUp)}
            times={series.times}
            intervalMs={intervalMs}
          />
          <p className="text-[13px] text-ink-soft mt-3">{network.explanation}</p>
        </div>
      </section>

      <section id="perf-disk" className="card p-5 scroll-mt-4">
        <h2 className="font-semibold">
          <button
            type="button"
            className="bg-transparent border-0 p-0 text-inherit font-semibold underline decoration-white/15 underline-offset-2 hover:decoration-accent"
            onClick={() => {
              const hasIo = processes.some((p) => p.diskBps > 0);
              if (hasIo) useAppStore.getState().toggleResourcePanel("disk");
              else useAppStore.getState().setPage("storage", { metric: "disk" });
            }}
          >
            Disk activity
          </button>
        </h2>
        <p className="text-[13px] text-ink-soft mt-1">
          This chart is transfer rate, not how full the volumes are. Capacity lives on Storage.
        </p>
        <AreaChart
          series={[{ name: series.fromDb ? "Activity MB/s" : "Busy", values: series.disk, color: "#f07167" }]}
          unit={series.fromDb ? "" : "%"}
          max={Math.max(4, ...series.disk)}
          times={series.times}
          intervalMs={intervalMs}
        />
        <div className="mt-4 grid md:grid-cols-2 gap-3">
          {disks.map((d) => (
            <div key={d.mount} className="bg-bg-elevated rounded-[10px] p-3">
              <div className="flex justify-between">
                <strong>{d.mount}</strong>
                <span className="text-ink-soft text-[13px]">{d.activity}</span>
              </div>
              <p className="text-[12px] text-ink-soft mt-2">
                Read {formatRate(d.readBps)} · Write {formatRate(d.writeBps)}
              </p>
              <p className="text-[12px] text-ink-mute">
                Active time {d.activeTimePercent != null ? `${d.activeTimePercent.toFixed(0)}%` : "Unavailable on this system"}{" "}
                · Queue {d.queueLength != null ? d.queueLength.toFixed(1) : "Unavailable on this system"} · Response{" "}
                {d.responseMs != null ? `${d.responseMs.toFixed(1)} ms` : "Unavailable on this system"}
              </p>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}

function KV({ k, v }: { k: string; v: string }) {
  return (
    <div>
      <dt className="text-ink-mute text-[11px]">{k}</dt>
      <dd>{v}</dd>
    </div>
  );
}

function groupVram(processes: ProcessRow[]) {
  const map = new Map<string, { key: string; name: string; pid: number; bytes: number; gpu: number; count: number }>();
  for (const p of processes) {
    const bytes = p.gpuMemoryBytes ?? 0;
    if (bytes <= 0) continue;
    const key = processGroupKey(p.name);
    const cur = map.get(key);
    if (!cur) {
      map.set(key, {
        key,
        name: p.displayName || p.name,
        pid: p.pid,
        bytes,
        gpu: p.gpu ?? 0,
        count: 1,
      });
    } else {
      cur.bytes += bytes;
      cur.gpu += p.gpu ?? 0;
      cur.count += 1;
    }
  }
  return [...map.values()].sort((a, b) => b.bytes - a.bytes).slice(0, 12);
}
