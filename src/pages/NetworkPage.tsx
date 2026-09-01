import { useEffect, useState } from "react";
import { getConnections } from "../lib/api";
import { formatRate } from "../lib/format";
import type { NetConnection } from "../lib/types";
import { useAppStore } from "../store/app-store";

export function NetworkPage() {
  const snapshot = useAppStore((s) => s.snapshot);
  const [conns, setConns] = useState<NetConnection[]>([]);
  const [q, setQ] = useState("");

  useEffect(() => {
    const t = window.setInterval(() => {
      getConnections().then(setConns).catch(() => undefined);
    }, 2000);
    getConnections().then(setConns).catch(() => undefined);
    return () => window.clearInterval(t);
  }, []);

  if (!snapshot) return <p className="text-ink-soft">Waiting for the first sample.</p>;
  const setPage = useAppStore.getState().setPage;
  const net = snapshot.network;
  const talkers = [...snapshot.processes]
    .filter((p) => p.connectionCount > 0)
    .sort((a, b) => b.connectionCount - a.connectionCount)
    .slice(0, 8);
  const filtered = conns.filter((c) =>
    `${c.processName} ${c.remote} ${c.local} ${c.pid}`.toLowerCase().includes(q.toLowerCase()),
  );

  return (
    <div className="flex flex-col gap-4 max-w-[1400px]">
      <header>
        <h1 className="text-2xl font-semibold tracking-tight">Network</h1>
        <p className="text-sm text-ink-soft mt-1">{net.explanation}</p>
      </header>
      <section className="grid md:grid-cols-3 gap-3">
        <div className="card p-4">
          <div className="text-xs text-ink-mute">Download</div>
          <div className="num text-2xl mt-1">{formatRate(net.downloadBps)}</div>
        </div>
        <div className="card p-4">
          <div className="text-xs text-ink-mute">Upload</div>
          <div className="num text-2xl mt-1">{formatRate(net.uploadBps)}</div>
        </div>
        <div className="card p-4">
          <div className="text-xs text-ink-mute">Sockets</div>
          <div className="num text-2xl mt-1">{net.connectionCount}</div>
        </div>
      </section>
      <section className="card p-4">
        <h2 className="font-semibold mb-3">Interfaces</h2>
        <ul className="flex flex-col gap-2">
          {net.interfaces.slice(0, 8).map((iface, i) => (
            <li key={`${iface.name}-${i}`} className="flex justify-between text-[13px] gap-4">
              <span className="truncate">{iface.name}</span>
              <span className="num text-ink-soft">
                {formatRate(iface.downloadBps)} down · {formatRate(iface.uploadBps)} up
              </span>
            </li>
          ))}
          {net.interfaces.length === 0 && (
            <li className="text-[13px] text-ink-soft">Unavailable on this system</li>
          )}
        </ul>
      </section>
      <section className="card p-4">
        <h2 className="font-semibold mb-2">Processes with sockets</h2>
        <p className="text-[12px] text-ink-mute mb-3">
          Per-process bandwidth is unavailable on this system. Connection counts are what Windows exposes here. A lot of sockets is not malware by itself.
        </p>
        <ul className="flex flex-col gap-1.5">
          {talkers.map((p) => (
            <li key={p.pid}>
              <button
                className="flex w-full justify-between text-[13px] bg-transparent border-0 p-0 text-left"
                onClick={() => setPage("processes", { pid: p.pid, processQuery: p.displayName || p.name })}
              >
                <span className="truncate">{p.displayName || p.name}</span>
                <span className="num text-ink-soft">{p.connectionCount} sockets</span>
              </button>
            </li>
          ))}
          {talkers.length === 0 && (
            <li className="text-[13px] text-ink-soft">Nothing interesting is happening right now.</li>
          )}
        </ul>
      </section>
      <section className="card p-4 flex flex-col min-h-[320px]">
        <div className="flex items-center justify-between gap-3 mb-3">
          <h2 className="font-semibold">Active connections</h2>
          <input className="field max-w-xs" placeholder="Filter" value={q} onChange={(e) => setQ(e.target.value)} />
        </div>
        <div className="overflow-auto flex-1">
          <table className="table">
            <thead>
              <tr>
                <th>Process</th>
                <th>PID</th>
                <th>Proto</th>
                <th>Remote</th>
                <th>State</th>
              </tr>
            </thead>
            <tbody>
              {filtered.slice(0, 250).map((c, i) => (
                <tr
                  key={`${c.pid}-${c.local}-${c.remote}-${i}`}
                  onClick={() => setPage("processes", { pid: c.pid, processQuery: c.processName })}
                >
                  <td>{c.processName}</td>
                  <td className="num">{c.pid}</td>
                  <td>{c.protocol}</td>
                  <td className="font-mono text-[12px]">{c.remote}</td>
                  <td className="text-ink-soft">{c.state}</td>
                </tr>
              ))}
              {filtered.length === 0 && (
                <tr>
                  <td colSpan={5} className="text-ink-soft">
                    Nothing interesting is happening right now.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </section>
    </div>
  );
}
