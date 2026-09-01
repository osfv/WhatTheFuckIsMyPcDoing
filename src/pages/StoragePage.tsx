import { Meter } from "../components/ui/Meter";
import { AreaChart } from "../components/ui/Sparkline";
import { formatBytes, formatPercent, formatRate } from "../lib/format";
import { useAppStore } from "../store/app-store";
import type { DiskMetrics, ProcessRow } from "../lib/types";

const NONE_DISKS: DiskMetrics[] = [];
const NONE_PROCS: ProcessRow[] = [];

export function StoragePage() {
  const disks = useAppStore((s) => s.snapshot?.disks) ?? NONE_DISKS;
  const processes = useAppStore((s) => s.snapshot?.processes) ?? NONE_PROCS;
  const history = useAppStore((s) => s.snapshot?.history?.disk);
  const intervalMs = useAppStore((s) => s.settings.sampleIntervalMs);
  const read = disks.reduce((s, d) => s + d.readBps, 0);
  const write = disks.reduce((s, d) => s + d.writeBps, 0);
  const topIo = [...processes].sort((a, b) => b.diskBps - a.diskBps).filter((p) => p.diskBps > 0).slice(0, 6);

  return (
    <div className="flex flex-col gap-4 max-w-[1100px]">
      <header>
        <h1 className="text-2xl font-semibold tracking-tight">Storage</h1>
        <p className="text-sm text-ink-soft mt-1">
          Storage Health is how full each volume is. Disk Activity is read and write. Those are not the same thing.
        </p>
      </header>

      <section>
        <h2 className="text-[15px] font-semibold mb-2">Storage Health</h2>
        <div className="grid gap-3">
          {disks.map((d) => (
            <article key={d.mount} className="card p-5">
              <div className="flex flex-wrap justify-between gap-3">
                <div>
                  <h3 className="text-lg font-semibold">{d.mount}</h3>
                  <p className="text-[13px] text-ink-soft mt-1">
                    {d.name || d.fs} · {d.kind} · {d.fs}
                    {d.removable ? " · removable" : ""}
                  </p>
                </div>
                <div className="text-right">
                  <div className="num text-2xl">{formatPercent(d.utilization * 100)}</div>
                  <div className="text-[12px] text-ink-mute">full</div>
                </div>
              </div>
              <Meter value={d.utilization * 100} className="mt-4 h-2" />
              <p className="text-[13px] text-ink-soft mt-3">
                {d.utilization >= 0.9
                  ? `${d.mount} is running out of space.`
                  : d.utilization >= 0.8
                    ? "Capacity is getting low."
                    : "Capacity is healthy."}{" "}
                {formatBytes(d.availableBytes)} free of {formatBytes(d.totalBytes)}.
              </p>
            </article>
          ))}
          {disks.length === 0 && <p className="text-ink-soft">No volumes reported yet.</p>}
        </div>
      </section>

      <section className="card p-5">
        <h2 className="text-[15px] font-semibold">Disk Activity</h2>
        <p className="text-[13px] text-ink-soft mt-1">
          Transfer rates, active time, queue, and response come from Windows disk counters when they are present.
        </p>
        <div className="grid md:grid-cols-2 gap-3 mt-4">
          <div>
            <div className="text-[11px] text-ink-mute">Read</div>
            <div className="num text-2xl">{formatRate(read)}</div>
          </div>
          <div>
            <div className="text-[11px] text-ink-mute">Write</div>
            <div className="num text-2xl">{formatRate(write)}</div>
          </div>
        </div>
        <div className="mt-4">
          <AreaChart
            series={[{ name: "Activity", values: history ?? [], color: "#f07167" }]}
            unit=""
            max={Math.max(4, ...(history ?? [1]))}
            intervalMs={intervalMs}
          />
        </div>
        <dl className="mt-4 grid grid-cols-2 md:grid-cols-4 gap-3 text-[13px]">
          {disks.slice(0, 1).map((d) => (
            <div key={d.mount} className="contents">
              <div>
                <dt className="text-ink-mute text-[11px]">Active time</dt>
                <dd>{d.activeTimePercent != null ? `${d.activeTimePercent.toFixed(0)}%` : "Unavailable on this system"}</dd>
              </div>
              <div>
                <dt className="text-ink-mute text-[11px]">Queue</dt>
                <dd>{d.queueLength != null ? d.queueLength.toFixed(1) : "Unavailable on this system"}</dd>
              </div>
              <div>
                <dt className="text-ink-mute text-[11px]">Response</dt>
                <dd>{d.responseMs != null ? `${d.responseMs.toFixed(1)} ms` : "Unavailable on this system"}</dd>
              </div>
              <div>
                <dt className="text-ink-mute text-[11px]">Now</dt>
                <dd>{d.activity}</dd>
              </div>
            </div>
          ))}
        </dl>
        <h3 className="text-[13px] font-semibold mt-4 mb-2">Processes hitting the disk</h3>
        <ul className="flex flex-col gap-1.5">
          {topIo.map((p) => (
            <li key={p.pid} className="flex justify-between text-[13px] gap-3">
              <span className="truncate">{p.displayName || p.name}</span>
              <span className="num text-ink-soft">{formatRate(p.diskBps)}</span>
            </li>
          ))}
          {topIo.length === 0 && <li className="text-[13px] text-ink-soft">Nothing interesting is happening right now.</li>}
        </ul>
      </section>
    </div>
  );
}
