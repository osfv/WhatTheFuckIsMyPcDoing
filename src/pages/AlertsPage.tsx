import { useEffect, useState } from "react";
import { Rise } from "../components/ui/motion";
import { acknowledgeAlert, acknowledgeAllAlerts, getAlerts } from "../lib/api";
import { formatDateTime } from "../lib/format";
import type { Alert } from "../lib/types";
import { goToCategory, useAppStore } from "../store/app-store";

const NONE: Alert[] = [];

export function AlertsPage() {
  const live = useAppStore((s) => s.snapshot?.alerts);
  const acknowledgeLocal = useAppStore((s) => s.acknowledgeLocal);
  const acknowledgeAllLocal = useAppStore((s) => s.acknowledgeAllLocal);
  const [fetched, setFetched] = useState<Alert[] | null>(null);
  const [filter, setFilter] = useState<"open" | "all">("open");

  useEffect(() => {
    getAlerts().then(setFetched).catch(() => undefined);
  }, []);

  const alerts = live ?? fetched ?? NONE;
  const shown = filter === "open" ? alerts.filter((a) => !a.acknowledged) : alerts;
  const openCount = alerts.filter((a) => !a.acknowledged).length;

  return (
    <div className="flex flex-col gap-4 max-w-[820px]">
      <header className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Alerts</h1>
          <p className="text-sm text-ink-soft mt-1">
            These wait for a condition to last. A one-second spike will not show up here.
          </p>
        </div>
        <div className="flex gap-2">
          <button className={`btn ${filter === "open" ? "primary" : ""}`} onClick={() => setFilter("open")}>
            Open {openCount ? `(${openCount})` : ""}
          </button>
          <button className={`btn ${filter === "all" ? "primary" : ""}`} onClick={() => setFilter("all")}>
            All
          </button>
          {openCount > 0 && (
            <button
              className="btn"
              onClick={async () => {
                await acknowledgeAllAlerts();
                acknowledgeAllLocal();
                setFetched((cur) => (cur ?? alerts).map((a) => ({ ...a, acknowledged: true })));
              }}
            >
              Dismiss all
            </button>
          )}
        </div>
      </header>
      {shown.map((alert, i) => (
        <Rise key={alert.id} delay={Math.min(i, 6) * 0.04}>
          <article
            className="card p-5 flex flex-col gap-2 cursor-pointer"
            onClick={() => goToCategory(alert.category)}
          >
            <div className="flex justify-between gap-3">
              <h2 className="font-semibold">{alert.title}</h2>
              <span className={`chip ${alert.severity === "critical" || alert.severity === "warning" ? "warn" : "info"}`}>
                {alert.severity}
              </span>
            </div>
            <p className="text-[13px] text-ink-soft">{alert.body}</p>
            <div className="flex justify-between items-center">
              <span className="text-[12px] text-ink-mute">{formatDateTime(alert.timestamp)}</span>
              {!alert.acknowledged && (
                <button
                  className="btn"
                  onClick={async (e) => {
                    e.stopPropagation();
                    await acknowledgeAlert(alert.id);
                    acknowledgeLocal(alert.id);
                    setFetched((cur) =>
                      (cur ?? alerts).map((a) => (a.id === alert.id ? { ...a, acknowledged: true } : a)),
                    );
                  }}
                >
                  Dismiss
                </button>
              )}
              {alert.acknowledged && <span className="text-[12px] text-ink-mute">Dismissed</span>}
            </div>
          </article>
        </Rise>
      ))}
      {shown.length === 0 && (
        <p className="text-ink-soft">
          {filter === "open" ? "No open alerts. That is the desired state." : "No alerts stored yet."}
        </p>
      )}
    </div>
  );
}
