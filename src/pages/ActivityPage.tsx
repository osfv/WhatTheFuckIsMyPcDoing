import { useEffect, useMemo, useState } from "react";
import { Rise } from "../components/ui/motion";
import { getEventHistory } from "../lib/api";
import { formatDateTime, whyLabel } from "../lib/format";
import { goToCategory, useAppStore } from "../store/app-store";
import type { ActivityEvent } from "../lib/types";

const NONE: ActivityEvent[] = [];

export function ActivityPage() {
  const live = useAppStore((s) => s.snapshot?.events) ?? NONE;
  const focusId = useAppStore((s) => s.focus.eventId);
  const [history, setHistory] = useState<ActivityEvent[]>([]);
  const [openId, setOpenId] = useState<string | null>(focusId ?? null);

  useEffect(() => {
    let alive = true;
    getEventHistory(24)
      .then((rows) => {
        if (alive) setHistory(rows);
      })
      .catch(() => {
        /* keep whatever we already have */
      });
    return () => {
      alive = false;
    };
  }, []);

  const events = useMemo(() => mergeEvents(live, history), [live, history]);

  return (
    <div className="flex flex-col gap-4 max-w-[820px]">
      <header>
        <h1 className="text-2xl font-semibold tracking-tight">Activity</h1>
        <p className="text-sm text-ink-soft mt-1">
          Meaningful events with a severity mark. Click one for the full explanation, or to jump to the related page.
        </p>
      </header>
      <Rise>
        <div className="relative pl-4">
          <div className="absolute left-[7px] top-2 bottom-2 w-px bg-line" aria-hidden />
          {events.map((event) => {
            const open = openId === event.id;
            return (
              <article key={event.id} className="relative mb-3">
                <span className={`sev-dot ${dot(event.severity)}`} aria-hidden />
                <button
                  className="card p-5 w-full text-left"
                  onClick={() => {
                    setOpenId(open ? null : event.id);
                  }}
                >
                  <div className="flex justify-between gap-3">
                    <h2 className="font-semibold">{event.title}</h2>
                    <span className="text-[12px] text-ink-mute">{formatDateTime(event.timestamp)}</span>
                  </div>
                  <p className="text-[13px] text-ink-soft mt-2 leading-relaxed">{event.body}</p>
                  {open && (
                    <div className="mt-3 flex flex-col gap-1.5">
                      {event.why && (
                        <p className="text-[13px]">
                          <span className="text-ink-mute">{whyLabel(event.why)}. </span>
                          {event.why}
                        </p>
                      )}
                      {event.recommendation && (
                        <p className="text-[13px] text-ink-soft">
                          <span className="text-ink-mute">Recommendation. </span>
                          {event.recommendation}
                        </p>
                      )}
                      <p className="text-[12px] text-ink-mute">
                        Impact {event.impact} · {event.category}
                      </p>
                    </div>
                  )}
                </button>
                {open && (
                  <button
                    className="btn mt-2"
                    onClick={() => goToCategory(event.category, event.process)}
                  >
                    Open related view
                  </button>
                )}
              </article>
            );
          })}
        </div>
      </Rise>
      {events.length === 0 && (
        <p className="text-ink-soft">Nothing interesting is happening right now.</p>
      )}
    </div>
  );
}

function mergeEvents(live: ActivityEvent[], history: ActivityEvent[]): ActivityEvent[] {
  const seen = new Set<string>();
  const out: ActivityEvent[] = [];
  for (const event of [...live, ...history]) {
    const fp = fingerprint(event);
    if (seen.has(fp) || seen.has(event.id)) continue;
    seen.add(fp);
    seen.add(event.id);
    out.push(event);
  }
  return out;
}

function fingerprint(event: ActivityEvent): string {
  return event.id.replace(/-\d{9,}$/, "");
}

function dot(severity: string) {
  if (severity === "critical" || severity === "warning") return "bad";
  if (severity === "notice") return "warn";
  return "ok";
}
