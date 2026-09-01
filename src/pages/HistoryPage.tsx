import { useEffect, useMemo, useState } from "react";
import { AreaChart } from "../components/ui/Sparkline";
import { getEventHistory, getHistory } from "../lib/api";
import { formatDateTime } from "../lib/format";
import type { ActivityEvent, HistoryPoint } from "../lib/types";
import { goToCategory } from "../store/app-store";

const ranges = [
  { id: 1, label: "1 hour" },
  { id: 6, label: "6 hours" },
  { id: 24, label: "24 hours" },
  { id: 168, label: "7 days" },
];

export function HistoryPage() {
  const [hours, setHours] = useState(24);
  const [points, setPoints] = useState<HistoryPoint[]>([]);
  const [events, setEvents] = useState<ActivityEvent[]>([]);

  useEffect(() => {
    getHistory(hours).then(setPoints).catch(() => setPoints([]));
    getEventHistory(hours).then(setEvents).catch(() => setEvents([]));
  }, [hours]);

  const cpu = points.map((p) => p.cpu);
  const gpu = points.map((p) => p.gpu);
  const memory = points.map((p) => p.memory);
  const notes = useMemo(() => explainSlow(points, events), [points, events]);

  return (
    <div className="flex flex-col gap-4 max-w-[1400px]">
      <header className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">History</h1>
          <p className="text-sm text-ink-soft mt-1">
            Stored samples only. If the app was closed, that stretch is missing on purpose.
          </p>
        </div>
        <div className="flex gap-1">
          {ranges.map((r) => (
            <button key={r.id} className={`btn ${hours === r.id ? "primary" : ""}`} onClick={() => setHours(r.id)}>
              {r.label}
            </button>
          ))}
        </div>
      </header>
      <section className="card p-5">
        <h2 className="font-semibold mb-3">Resource history</h2>
        {points.length < 2 ? (
          <p className="text-ink-soft text-sm">Not enough samples yet. Leave the app running for a bit.</p>
        ) : (
          <AreaChart
            series={[
              { name: "CPU", values: cpu, color: "#ff6b57" },
              { name: "GPU", values: gpu, color: "#7eb6ff" },
              { name: "RAM", values: memory, color: "#3ddc97" },
            ]}
            times={points.map((p) => p.timestamp)}
          />
        )}
      </section>
      <section className="card p-5">
        <h2 className="font-semibold mb-3">Slow periods</h2>
        <ul className="flex flex-col gap-2">
          {notes.map((n, i) => (
            <li key={`${i}-${n.slice(0, 48)}`} className="text-[13px] text-ink-soft leading-relaxed">
              {n}
            </li>
          ))}
        </ul>
      </section>
      <section className="card p-5">
        <h2 className="font-semibold mb-3">What happened</h2>
        {events.length === 0 ? (
          <p className="text-ink-soft text-sm">No stored events in this window.</p>
        ) : (
          <ul className="flex flex-col gap-3">
            {events.slice(0, 80).map((e) => (
              <li key={e.id} className="border-b border-line pb-3">
                <button
                  className="w-full text-left bg-transparent border-0 p-0"
                  onClick={() => goToCategory(e.category, e.process)}
                >
                  <div className="flex justify-between gap-3">
                    <strong className="text-[14px]">{e.title}</strong>
                    <span className="text-[12px] text-ink-mute">{formatDateTime(e.timestamp)}</span>
                  </div>
                  <p className="text-[13px] text-ink-soft mt-1">{e.body}</p>
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}

function explainSlow(points: HistoryPoint[], events: ActivityEvent[]): string[] {
  if (points.length < 4) {
    return ["Not enough stored samples to explain a slow stretch yet."];
  }
  const notes: string[] = [];
  let start: HistoryPoint | null = null;
  let peakCpu = 0;
  let peakRam = 0;
  const flush = (end: HistoryPoint) => {
    if (!start) return;
    const span = end.timestamp - start.timestamp;
    if (span < 90) {
      start = null;
      peakCpu = 0;
      peakRam = 0;
      return;
    }
    const related = events.filter((e) => e.timestamp >= start!.timestamp && e.timestamp <= end.timestamp);
    const who = related.find((e) => e.process)?.process;
    const when = formatDateTime(start.timestamp);
    const mins = Math.max(1, Math.round(span / 60));
    let line = `${when}: load stayed high for about ${mins} minute${mins === 1 ? "" : "s"} (CPU peak ${peakCpu.toFixed(0)}%, RAM peak ${peakRam.toFixed(0)}%).`;
    if (who) line += ` ${who} shows up in the events for that window.`;
    else if (related[0]) line += ` Closest note: ${related[0].title}.`;
    else line += " No stored event names the process, so the cause stays unknown.";
    notes.push(line);
    start = null;
    peakCpu = 0;
    peakRam = 0;
  };

  for (const p of points) {
    const busy = p.cpu >= 80 || p.memory >= 90;
    if (busy) {
      if (!start) start = p;
      peakCpu = Math.max(peakCpu, p.cpu);
      peakRam = Math.max(peakRam, p.memory);
    } else if (start) {
      flush(p);
    }
  }
  if (start) flush(points[points.length - 1]);

  if (notes.length === 0) {
    return ["No slow stretches in this window. CPU stayed under 80% and RAM under 90% in the stored samples."];
  }
  return notes.slice(0, 8);
}
