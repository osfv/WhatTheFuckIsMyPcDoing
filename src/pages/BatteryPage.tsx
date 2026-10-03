import { useEffect, useState } from "react";
import { BatteryCharging, BatteryHigh, BatteryLow, BatteryWarning, Plug } from "@phosphor-icons/react";
import { Meter } from "../components/ui/Meter";
import { Rise } from "../components/ui/motion";
import { getPower } from "../lib/api";
import { formatDuration, formatRate } from "../lib/format";
import type { AppPower, BatteryStatus, PowerReport } from "../lib/types";
import { useAppStore } from "../store/app-store";

export function BatteryPage() {
  const [report, setReport] = useState<PowerReport | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const load = () =>
      getPower()
        .then((r) => {
          setReport(r);
          setError(null);
        })
        .catch((err: unknown) => setError(String(err)));
    load();
    const t = window.setInterval(load, 2000);
    return () => window.clearInterval(t);
  }, []);

  if (!report) {
    return (
      <div className="card p-6 max-w-lg">
        <h1 className="text-2xl font-semibold">Battery</h1>
        <p className="text-ink-soft mt-2">{error ?? "Reading battery and power use."}</p>
      </div>
    );
  }

  const { battery } = report;
  return (
    <div className="flex flex-col gap-4 max-w-[1100px]">
      <header>
        <h1 className="text-2xl font-semibold tracking-tight">Battery</h1>
        <p className="text-sm text-ink-soft mt-1">
          Charge, health, and which apps are costing you power.
          {battery.mocked ? " Showing mocked battery data (WTF_MOCK_BATTERY)." : ""}
        </p>
      </header>
      {error && <p className="text-sm text-ink-soft">{error}</p>}
      <Rise>
        <BatteryCard battery={battery} />
      </Rise>
      <Rise delay={0.06}>
        <PowerCard report={report} />
      </Rise>
    </div>
  );
}

function BatteryIcon({ battery }: { battery: BatteryStatus }) {
  if (!battery.present) return <Plug size={28} weight="duotone" />;
  if (battery.state === "charging") return <BatteryCharging size={28} weight="duotone" />;
  const pct = battery.percent ?? 100;
  if (pct <= 15) return <BatteryWarning size={28} weight="duotone" />;
  if (pct <= 40) return <BatteryLow size={28} weight="duotone" />;
  return <BatteryHigh size={28} weight="duotone" />;
}

function stateChip(battery: BatteryStatus): { label: string; tone: string } {
  switch (battery.state) {
    case "charging":
      return { label: "Charging", tone: "good" };
    case "discharging":
      return (battery.percent ?? 100) <= 15 ? { label: "Low", tone: "bad" } : { label: "On battery", tone: "warn" };
    case "full":
      return { label: "Full", tone: "good" };
    case "plugged":
      return { label: "Plugged in", tone: "info" };
    case "none":
      return { label: "No battery", tone: "info" };
    default:
      return { label: "Unknown", tone: "info" };
  }
}

function healthTone(label: string) {
  if (label === "Healthy") return "good";
  if (label === "Worn") return "warn";
  if (label === "Heavily worn") return "bad";
  return "info";
}

const wh = (v: number | null) => (v == null ? "Unavailable on this system" : `${v.toFixed(1)} Wh`);

function BatteryCard({ battery }: { battery: BatteryStatus }) {
  const chip = stateChip(battery);
  if (!battery.present) {
    return (
      <section className="card p-6" data-testid="battery-card">
        <div className="flex items-start gap-4">
          <BatteryIcon battery={battery} />
          <div className="flex-1">
            <div className="flex flex-wrap items-center gap-2">
              <h2 className="text-xl font-semibold">{battery.headline}</h2>
              <span className={`chip ${chip.tone}`}>{chip.label}</span>
            </div>
            <p className="text-[13px] text-ink-soft mt-1">{battery.summary}</p>
          </div>
        </div>
      </section>
    );
  }
  const time =
    battery.state === "discharging" && battery.timeRemainingSec != null
      ? { label: "Time left", value: formatDuration(battery.timeRemainingSec) }
      : battery.state === "charging" && battery.timeToFullSec != null
        ? { label: "Until full", value: formatDuration(battery.timeToFullSec) }
        : { label: "Time left", value: battery.state === "discharging" ? "Estimating" : "On wall power" };
  return (
    <section className="card p-6" data-testid="battery-card">
      <div className="flex flex-wrap justify-between gap-4">
        <div className="flex items-start gap-4">
          <BatteryIcon battery={battery} />
          <div>
            <div className="flex flex-wrap items-center gap-2">
              <h2 className="text-xl font-semibold">{battery.headline}</h2>
              <span className={`chip ${chip.tone}`}>{chip.label}</span>
            </div>
            <p className="text-[13px] text-ink-soft mt-1 max-w-[620px]">{battery.summary}</p>
          </div>
        </div>
        <div className="text-right">
          <div className="num text-3xl">{battery.percent != null ? `${battery.percent.toFixed(0)}%` : "?"}</div>
          <div className="text-[12px] text-ink-mute">charge</div>
        </div>
      </div>
      <Meter value={battery.percent ?? 0} className="mt-4 h-2" />
      <dl className="mt-4 grid grid-cols-2 md:grid-cols-4 gap-3 text-[13px]">
        <div>
          <dt className="text-ink-mute text-[11px]">{battery.state === "charging" ? "Charge rate" : "Drain rate"}</dt>
          <dd className="num">{battery.rateW != null ? `${battery.rateW.toFixed(1)} W` : "Unavailable on this system"}</dd>
        </div>
        <div>
          <dt className="text-ink-mute text-[11px]">{time.label}</dt>
          <dd className="num">{time.value}</dd>
        </div>
        <div>
          <dt className="text-ink-mute text-[11px]">Health</dt>
          <dd className="flex items-center gap-2">
            <span className="num">{battery.healthPercent != null ? `${battery.healthPercent.toFixed(0)}%` : "Unknown"}</span>
            <span className={`chip ${healthTone(battery.healthLabel)}`}>{battery.healthLabel}</span>
          </dd>
        </div>
        <div>
          <dt className="text-ink-mute text-[11px]">Cycles</dt>
          <dd className="num">{battery.cycleCount ?? "Unavailable on this system"}</dd>
        </div>
        <div>
          <dt className="text-ink-mute text-[11px]">Holds now</dt>
          <dd className="num">{wh(battery.fullWh)}</dd>
        </div>
        <div>
          <dt className="text-ink-mute text-[11px]">Held when new</dt>
          <dd className="num">{wh(battery.designWh)}</dd>
        </div>
        <div>
          <dt className="text-ink-mute text-[11px]">Remaining</dt>
          <dd className="num">{wh(battery.remainingWh)}</dd>
        </div>
        <div>
          <dt className="text-ink-mute text-[11px]">Pack</dt>
          <dd className="truncate">
            {[battery.manufacturer, battery.name].filter(Boolean).join(" · ") || "Unavailable on this system"}
          </dd>
        </div>
      </dl>
      {battery.healthPercent != null && (
        <div className="mt-4">
          <div className="text-[11px] text-ink-mute mb-1">Capacity vs. design</div>
          <Meter value={battery.healthPercent} className="h-1.5" />
        </div>
      )}
    </section>
  );
}

function PowerCard({ report }: { report: PowerReport }) {
  const setPage = useAppStore((s) => s.setPage);
  const max = Math.max(0.01, ...report.apps.map((a) => a.watts));
  return (
    <section className="card p-6" data-testid="power-card">
      <h2 className="text-xl font-semibold">{report.headline}</h2>
      <p className="text-[13px] text-ink-soft mt-1">{report.summary}</p>
      <div className="mt-4 grid grid-cols-2 md:grid-cols-3 gap-3 text-[13px]">
        <div>
          <div className="text-[11px] text-ink-mute">Apps, estimated</div>
          <div className="num text-2xl">{report.estimatedAppW.toFixed(1)} W</div>
        </div>
        {report.systemDrawW != null && (
          <div>
            <div className="text-[11px] text-ink-mute">Whole PC, measured</div>
            <div className="num text-2xl">{report.systemDrawW.toFixed(1)} W</div>
          </div>
        )}
        {report.otherW != null && (
          <div>
            <div className="text-[11px] text-ink-mute">Screen and the rest</div>
            <div className="num text-2xl">{report.otherW.toFixed(1)} W</div>
          </div>
        )}
      </div>
      <ul className="mt-5 flex flex-col gap-2.5">
        {report.apps.map((app) => (
          <AppRow
            key={app.key}
            app={app}
            max={max}
            onOpen={() => setPage("processes", { pid: app.pid, processQuery: app.name.replace(/ \(\d+\)$/, "") })}
          />
        ))}
        {report.apps.length === 0 && <li className="text-[13px] text-ink-soft">No app is using noticeable power.</li>}
      </ul>
      <p className="text-[12px] text-ink-mute mt-5">{report.sourceLabel}</p>
    </section>
  );
}

function AppRow({ app, max, onOpen }: { app: AppPower; max: number; onOpen: () => void }) {
  const parts = [
    app.cpu > 0.05 ? `CPU ${app.cpu.toFixed(1)}%` : null,
    app.gpu > 0.05 ? `GPU ${app.gpu.toFixed(0)}%` : null,
    app.diskBps > 0 ? `Disk ${formatRate(app.diskBps)}` : null,
  ].filter(Boolean);
  return (
    <li>
      <button className="w-full text-left" onClick={onOpen} title="Show in Processes">
        <div className="flex justify-between gap-3 text-[13px]">
          <span className="truncate font-medium">{app.name}</span>
          <span className="num text-ink-soft shrink-0">
            ~{app.watts.toFixed(1)} W · {(app.share * 100).toFixed(0)}%
            {app.batteryPercentPerHour != null && app.batteryPercentPerHour >= 0.5
              ? ` · ${app.batteryPercentPerHour.toFixed(0)}%/h`
              : ""}
          </span>
        </div>
        <Meter value={(app.watts / max) * 100} className="mt-1.5 h-1.5" />
        <div className="text-[11px] text-ink-mute mt-1">
          Mostly {app.mainComponent}
          {parts.length > 0 ? ` · ${parts.join(" · ")}` : ""}
        </div>
      </button>
    </li>
  );
}
