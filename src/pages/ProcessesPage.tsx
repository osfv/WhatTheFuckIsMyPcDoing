import { memo, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { MagnifyingGlass, TreeStructure, Rows, SquaresFour } from "@phosphor-icons/react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { formatBytes, formatDateTime, formatDuration, formatRate, processGroupKey, processLabel } from "../lib/format";
import type { ProcessRow } from "../lib/types";
import { getProcessDetails, killProcess, revealPath, searchWeb } from "../lib/api";
import { useAppStore } from "../store/app-store";

type SortKey = "name" | "cpu" | "memoryBytes" | "gpu" | "diskBps" | "pid";
type ViewRow = ProcessRow & {
  count?: number;
  depth?: number;
  rowKey: string;
  memberPids: number[];
  leaderMemory?: number;
};

const ROW_HEIGHT = 36;
const NONE_PROCS: ProcessRow[] = [];

export function ProcessesPage() {
  const processes = useAppStore((s) => s.snapshot?.processes);
  const snapshotTs = useAppStore((s) => s.snapshot?.timestamp);
  const gpuAvailable = useAppStore((s) => s.snapshot?.gpu.available) ?? false;
  const query = useAppStore((s) => s.query);
  const setQuery = useAppStore((s) => s.setQuery);
  const group = useAppStore((s) => s.processGroup);
  const setGroup = useAppStore((s) => s.setProcessGroup);
  const filter = useAppStore((s) => s.processFilter);
  const setFilter = useAppStore((s) => s.setProcessFilter);
  const selectedPid = useAppStore((s) => s.selectedPid);
  const setSelectedPid = useAppStore((s) => s.setSelectedPid);
  const inspector = useAppStore((s) => s.inspector);
  const setInspector = useAppStore((s) => s.setInspector);
  const [sort, setSort] = useState<SortKey>("cpu");
  const [dir, setDir] = useState<"desc" | "asc">("desc");
  const [confirmKill, setConfirmKill] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [holdOrder, setHoldOrder] = useState(false);
  const orderRef = useRef<string[]>([]);

  const list = processes ?? NONE_PROCS;

  const rows = useMemo(() => {
    const built = buildRows(list, query, filter, group, sort, dir);
    if (holdOrder && orderRef.current.length > 0) {
      const byKey = new Map(built.map((r) => [r.rowKey, r]));
      const next: ViewRow[] = [];
      for (const key of orderRef.current) {
        const row = byKey.get(key);
        if (row) {
          next.push(row);
          byKey.delete(key);
        }
      }
      for (const row of built) {
        if (byKey.has(row.rowKey)) next.push(row);
      }
      return next;
    }
    orderRef.current = built.map((r) => r.rowKey);
    return built;
  }, [list, query, filter, group, sort, dir, holdOrder]);

  const selectedRow = useMemo(
    () => (selectedPid == null ? undefined : findRow(rows, selectedPid)),
    [rows, selectedPid],
  );

  useEffect(() => {
    if (selectedPid == null) return;
    let alive = true;
    const pid = selectedPid;
    getProcessDetails(pid)
      .then((details) => {
        if (alive && useAppStore.getState().selectedPid === pid) {
          setInspector(details);
          setError(null);
        }
      })
      .catch((err: unknown) => {
        if (alive) setError(String(err));
      });
    return () => {
      alive = false;
    };
  }, [selectedPid, snapshotTs, setInspector]);

  useEffect(() => {
    setConfirmKill(false);
    if (selectedPid != null) setHoldOrder(true);
  }, [selectedPid]);

  const toggleSort = (key: SortKey) => {
    setHoldOrder(false);
    if (sort === key) setDir((d) => (d === "desc" ? "asc" : "desc"));
    else {
      setSort(key);
      setDir(key === "name" ? "asc" : "desc");
    }
  };

  const selectRow = useCallback(
    (row: ViewRow) => {
      setConfirmKill(false);
      setError(null);
      setSelectedPid(row.pid);
    },
    [setSelectedPid],
  );

  const processCount = list.length;

  return (
    <div className="flex h-full min-h-0 -m-1">
      <div className="flex-1 min-w-0 flex flex-col gap-3 p-1 min-h-0">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Processes</h1>
          <p className="text-sm text-ink-soft mt-1">
            {processCount} running. Grouped view sums Chrome-style process trees so the number means something.
          </p>
        </div>
        <div className="flex flex-wrap gap-2 items-center">
          <label className="relative flex-1 min-w-[220px]">
            <MagnifyingGlass className="absolute left-3 top-1/2 -translate-y-1/2 text-ink-mute" size={16} />
            <input
              className="field pl-9"
              placeholder="Search name, PID, or path"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              aria-label="Search processes"
            />
          </label>
          <Segmented
            value={filter}
            onChange={setFilter}
            options={[
              { id: "all", label: "All" },
              { id: "apps", label: "Apps" },
              { id: "system", label: "Windows" },
            ]}
          />
          <Segmented
            value={group}
            onChange={setGroup}
            options={[
              { id: "grouped", label: "Grouped", icon: SquaresFour },
              { id: "tree", label: "Tree", icon: TreeStructure },
              { id: "flat", label: "Flat", icon: Rows },
            ]}
          />
        </div>
        <ProcessTable
          rows={rows}
          selectedPid={selectedPid}
          gpuAvailable={gpuAvailable}
          onSelect={selectRow}
          onToggleSort={toggleSort}
          onPointerActiveChange={setHoldOrder}
        />
      </div>
      {selectedPid != null && (
        <aside className="inspector p-5 flex flex-col gap-4">
          <div>
            <div className="text-xs text-ink-mute">PID {selectedPid}</div>
            <h2 className="text-xl font-semibold tracking-tight">
              {inspector?.process.pid === selectedPid
                ? inspector.process.displayName || inspector.process.name
                : selectedRow?.displayName || selectedRow?.name || "Getting details"}
            </h2>
            <p className="text-[13px] text-ink-soft mt-2 leading-relaxed">
              {inspector?.process.pid === selectedPid
                ? inspector.hint ?? "No built-in profile for this process."
                : "Numbers below update as soon as details arrive."}
            </p>
            {inspector?.process.pid === selectedPid && inspector.groupCount > 1 && (
              <p className="text-[13px] mt-2">
                {inspector.expected ? "Normal. " : ""}
                {inspector.process.displayName || inspector.process.name} has {inspector.groupCount} related
                processes using {formatBytes(inspector.groupMemoryBytes)} and {inspector.groupCpu.toFixed(1)}% CPU
                together.
              </p>
            )}
          </div>
          <dl className="grid grid-cols-2 gap-x-3 gap-y-2 text-[13px]">
            <Stat
              k="CPU"
              v={`${(inspector?.process.pid === selectedPid ? inspector.process.cpu : selectedRow?.cpu ?? 0).toFixed(1)}%`}
            />
            <Stat
              k="RAM"
              v={formatBytes(
                inspector?.process.pid === selectedPid ? inspector.process.memoryBytes : selectedRow?.memoryBytes ?? 0,
              )}
            />
            <Stat
              k="GPU"
              v={gpuLabel(
                inspector?.process.pid === selectedPid ? inspector.process.gpu : selectedRow?.gpu,
                gpuAvailable,
              )}
            />
            <Stat
              k="Disk"
              v={formatRate(
                inspector?.process.pid === selectedPid ? inspector.process.diskBps : selectedRow?.diskBps ?? 0,
              )}
            />
            <Stat
              k="Lifetime"
              v={
                inspector?.process.pid === selectedPid
                  ? formatDuration(inspector.process.runTimeSec)
                  : selectedRow
                    ? formatDuration(selectedRow.runTimeSec)
                    : "Getting details"
              }
            />
            <Stat
              k="Started"
              v={
                inspector?.process.pid === selectedPid && inspector.process.startTime
                  ? formatDateTime(inspector.process.startTime)
                  : "Unavailable on this system"
              }
            />
            <Stat
              k="Sockets"
              v={inspector?.process.pid === selectedPid ? String(inspector.connections.length) : "Getting details"}
            />
          </dl>
          {inspector?.process.pid === selectedPid && (
            <>
              <div>
                <h3 className="text-xs text-ink-mute uppercase tracking-wide mb-1">Location</h3>
                <p className="selectable text-[12px] font-mono leading-relaxed break-all text-ink-soft">
                  {inspector.process.path ?? "Unavailable on this system"}
                </p>
              </div>
              <div>
                <h3 className="text-xs text-ink-mute uppercase tracking-wide mb-1">Publisher</h3>
                <p className="text-[13px]">{inspector.publisher ?? "Unknown publisher"}</p>
                <p className="text-[12px] text-ink-soft mt-1">{inspector.signatureStatus}</p>
              </div>
              {inspector.commandLine && (
                <div>
                  <h3 className="text-xs text-ink-mute uppercase tracking-wide mb-1">Command</h3>
                  <p className="selectable text-[11px] font-mono text-ink-soft break-all">{inspector.commandLine}</p>
                </div>
              )}
              {inspector.children.length > 0 && (
                <div>
                  <h3 className="text-xs text-ink-mute uppercase tracking-wide mb-1">Children</h3>
                  <ul className="text-[13px] text-ink-soft">
                    {inspector.children.slice(0, 12).map((c) => (
                      <li key={c.pid}>
                        <button
                          className="bg-transparent border-0 p-0 text-inherit"
                          onPointerDown={(e) => {
                            if (e.button !== 0) return;
                            setSelectedPid(c.pid);
                          }}
                        >
                          {c.displayName || c.name} · {c.pid}
                        </button>
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </>
          )}
          {error && <p className="text-[13px] text-bad">{error}</p>}
          <div className="mt-auto flex flex-col gap-2">
            {inspector?.process.pid === selectedPid && inspector.process.path && (
              <button className="btn" onClick={() => revealPath(inspector.process.path!).catch((e) => setError(String(e)))}>
                Open file location
              </button>
            )}
            {inspector?.process.pid === selectedPid && (
              <>
                <button
                  className="btn"
                  onClick={() =>
                    navigator.clipboard.writeText(
                      `${inspector.process.name} PID ${inspector.process.pid}\n${inspector.process.path ?? ""}`,
                    )
                  }
                >
                  Copy details
                </button>
                <button
                  className="btn"
                  onClick={() => searchWeb(`${inspector.process.name} windows process`).catch((e) => setError(String(e)))}
                >
                  Search online
                </button>
              </>
            )}
            {!confirmKill ? (
              <button className="btn danger" onClick={() => setConfirmKill(true)}>
                End process
              </button>
            ) : (
              <div className="flex gap-2">
                <button
                  className="btn danger flex-1"
                  onClick={async () => {
                    try {
                      await killProcess(selectedPid);
                      setConfirmKill(false);
                      setSelectedPid(null);
                    } catch (err) {
                      setError(String(err));
                    }
                  }}
                >
                  Confirm end
                </button>
                <button className="btn flex-1" onClick={() => setConfirmKill(false)}>
                  Cancel
                </button>
              </div>
            )}
          </div>
        </aside>
      )}
    </div>
  );
}

function ProcessTable({
  rows,
  selectedPid,
  gpuAvailable,
  onSelect,
  onToggleSort,
  onPointerActiveChange,
}: {
  rows: ViewRow[];
  selectedPid: number | null;
  gpuAvailable: boolean;
  onSelect: (row: ViewRow) => void;
  onToggleSort: (key: SortKey) => void;
  onPointerActiveChange: (active: boolean) => void;
}) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const clickedRef = useRef(false);
  const lastScrolledPid = useRef<number | null>(null);

  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 14,
    paddingStart: ROW_HEIGHT,
    scrollPaddingStart: ROW_HEIGHT,
    getItemKey: (index) => rows[index]?.rowKey ?? `i:${index}`,
  });

  const items = virtualizer.getVirtualItems();
  const paddingTop = items.length > 0 ? Math.max(0, items[0].start - ROW_HEIGHT) : 0;
  const paddingBottom =
    items.length > 0 ? Math.max(0, virtualizer.getTotalSize() - (items[items.length - 1]?.end ?? 0)) : 0;

  useLayoutEffect(() => {
    if (selectedPid == null) {
      lastScrolledPid.current = null;
      return;
    }
    if (clickedRef.current) {
      clickedRef.current = false;
      lastScrolledPid.current = selectedPid;
      return;
    }
    if (lastScrolledPid.current === selectedPid) return;
    const idx = rows.findIndex((r) => rowMatchesPid(r, selectedPid));
    if (idx < 0) return;
    lastScrolledPid.current = selectedPid;
    virtualizer.scrollToIndex(idx, { align: "center" });
  }, [selectedPid, rows, virtualizer]);

  const handleSelect = useCallback(
    (row: ViewRow) => {
      clickedRef.current = true;
      onSelect(row);
    },
    [onSelect],
  );

  return (
    <div
      ref={scrollRef}
      className="card process-scroll flex-1 min-h-0 overflow-auto"
      onPointerEnter={() => onPointerActiveChange(true)}
      onPointerLeave={() => {
        if (selectedPid == null) onPointerActiveChange(false);
      }}
    >
      <table className="table process-table">
        <thead>
          <tr>
            <Th onClick={() => onToggleSort("name")}>Name</Th>
            <Th onClick={() => onToggleSort("pid")}>PID</Th>
            <Th onClick={() => onToggleSort("cpu")}>CPU</Th>
            <Th onClick={() => onToggleSort("memoryBytes")}>RAM</Th>
            <Th onClick={() => onToggleSort("gpu")}>GPU</Th>
            <Th onClick={() => onToggleSort("diskBps")}>Disk</Th>
            <th>Network</th>
          </tr>
        </thead>
        <tbody>
          {paddingTop > 0 && (
            <tr className="virtual-pad" aria-hidden>
              <td colSpan={7} style={{ height: paddingTop }} />
            </tr>
          )}
          {items.map((item) => {
            const row = rows[item.index];
            if (!row) return null;
            return (
              <ProcessTableRow
                key={row.rowKey}
                row={row}
                selected={selectedPid != null && rowMatchesPid(row, selectedPid)}
                gpuAvailable={gpuAvailable}
                onSelect={handleSelect}
              />
            );
          })}
          {paddingBottom > 0 && (
            <tr className="virtual-pad" aria-hidden>
              <td colSpan={7} style={{ height: paddingBottom }} />
            </tr>
          )}
          {rows.length === 0 && (
            <tr>
              <td colSpan={7} className="text-ink-soft">
                Nothing interesting is happening right now.
              </td>
            </tr>
          )}
        </tbody>
      </table>
    </div>
  );
}

const ProcessTableRow = memo(function ProcessTableRow({
  row,
  selected,
  gpuAvailable,
  onSelect,
}: {
  row: ViewRow;
  selected: boolean;
  gpuAvailable: boolean;
  onSelect: (row: ViewRow) => void;
}) {
  return (
    <tr
      data-pid={row.pid}
      data-row-key={row.rowKey}
      className={selected ? "selected" : ""}
      onPointerDown={(e) => {
        if (e.button !== 0) return;
        onSelect(row);
      }}
    >
      <td className="max-w-[280px] truncate" style={{ paddingLeft: 12 + (row.depth ?? 0) * 14 }}>
        {row.displayName || row.name}
      </td>
      <td className="num text-ink-soft">{row.pid}</td>
      <td className="num">{Number.isFinite(row.cpu) ? row.cpu.toFixed(1) : "0.0"}%</td>
      <td className="num">{formatBytes(row.memoryBytes)}</td>
      <td className="num">{gpuLabel(row.gpu, gpuAvailable, true)}</td>
      <td className="num">{row.diskBps ? formatRate(row.diskBps) : "0 B/s"}</td>
      <td className="num text-ink-soft">{row.connectionCount || "0"}</td>
    </tr>
  );
}, processRowPropsEqual);

function processRowPropsEqual(
  prev: { row: ViewRow; selected: boolean; gpuAvailable: boolean; onSelect: (row: ViewRow) => void },
  next: { row: ViewRow; selected: boolean; gpuAvailable: boolean; onSelect: (row: ViewRow) => void },
) {
  return (
    prev.selected === next.selected &&
    prev.gpuAvailable === next.gpuAvailable &&
    prev.onSelect === next.onSelect &&
    prev.row.rowKey === next.row.rowKey &&
    prev.row.pid === next.row.pid &&
    prev.row.cpu === next.row.cpu &&
    prev.row.memoryBytes === next.row.memoryBytes &&
    prev.row.gpu === next.row.gpu &&
    prev.row.diskBps === next.row.diskBps &&
    prev.row.connectionCount === next.row.connectionCount &&
    prev.row.displayName === next.row.displayName &&
    prev.row.depth === next.row.depth
  );
}

function Th({ children, onClick }: { children: string; onClick: () => void }) {
  return (
    <th onClick={onClick}>
      <button type="button" className="bg-transparent border-0 text-inherit p-0">
        {children}
      </button>
    </th>
  );
}

function gpuLabel(gpu: number | null | undefined, available: boolean, compact = false): string {
  if (gpu != null && Number.isFinite(gpu)) return `${gpu.toFixed(1)}%`;
  if (available) return "0%";
  return compact ? "n/a" : "Unavailable on this system";
}

function Stat({ k, v }: { k: string; v: string }) {
  return (
    <div>
      <dt className="text-ink-mute text-[11px]">{k}</dt>
      <dd className="num">{v}</dd>
    </div>
  );
}

function Segmented<T extends string>({
  value,
  onChange,
  options,
}: {
  value: T;
  onChange: (v: T) => void;
  options: { id: T; label: string; icon?: typeof SquaresFour }[];
}) {
  return (
    <div className="flex bg-bg-elevated border border-line rounded-[8px] p-0.5">
      {options.map((o) => (
        <button
          key={o.id}
          type="button"
          className={`h-8 px-2.5 rounded-[6px] text-[12px] ${value === o.id ? "bg-surface-2 text-ink" : "text-ink-soft"}`}
          onClick={() => onChange(o.id)}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

function findRow(rows: ViewRow[], pid: number) {
  return rows.find((r) => rowMatchesPid(r, pid));
}

function rowMatchesPid(row: ViewRow, pid: number) {
  return row.pid === pid || row.memberPids.includes(pid);
}

function isSystem(p: ProcessRow) {
  const path = (p.path ?? "").toLowerCase();
  return path.includes("\\windows\\") || ["system", "registry", "smss.exe", "csrss.exe"].includes(p.name.toLowerCase());
}

function compare(a: ProcessRow, b: ProcessRow, key: SortKey, dir: "asc" | "desc") {
  const mul = dir === "asc" ? 1 : -1;
  if (key === "name") return processLabel(a).localeCompare(processLabel(b)) * mul;
  const av = (a[key] as number | null) ?? -1;
  const bv = (b[key] as number | null) ?? -1;
  return (av - bv) * mul;
}

function buildRows(
  processes: ProcessRow[],
  query: string,
  filter: "all" | "apps" | "system",
  group: "grouped" | "tree" | "flat",
  sort: SortKey,
  dir: "asc" | "desc",
): ViewRow[] {
  const list = processes.filter((p) => {
    const hay = `${p.displayName} ${p.name} ${p.pid} ${p.path ?? ""}`.toLowerCase();
    if (query && !hay.includes(query.toLowerCase())) return false;
    if (filter === "system") return isSystem(p);
    if (filter === "apps") return !isSystem(p);
    return true;
  });
  if (group === "grouped") return buildGrouped(list, sort, dir);
  if (group === "tree") return buildTree(list, sort, dir);
  return [...list]
    .map((p) => ({
      ...p,
      rowKey: `p:${p.pid}`,
      depth: 0,
      displayName: processLabel(p),
      memberPids: [p.pid],
    }))
    .sort((a, b) => compare(a, b, sort, dir));
}

function buildGrouped(list: ProcessRow[], sort: SortKey, dir: "asc" | "desc"): ViewRow[] {
  const map = new Map<string, ViewRow>();
  for (const p of list) {
    const key = processGroupKey(p.name);
    const rowKey = `g:${key}`;
    const cur = map.get(rowKey);
    if (!cur) {
      map.set(rowKey, { ...p, count: 1, depth: 0, rowKey, memberPids: [p.pid], leaderMemory: p.memoryBytes });
    } else {
      if (p.memoryBytes > (cur.leaderMemory ?? 0)) {
        cur.pid = p.pid;
        cur.leaderMemory = p.memoryBytes;
      }
      cur.cpu += p.cpu;
      cur.memoryBytes += p.memoryBytes;
      cur.diskBps += p.diskBps;
      cur.diskReadBps += p.diskReadBps;
      cur.diskWriteBps += p.diskWriteBps;
      cur.connectionCount += p.connectionCount;
      cur.count = (cur.count ?? 1) + 1;
      cur.memberPids.push(p.pid);
      if ((p.gpu ?? 0) > (cur.gpu ?? 0)) cur.gpu = p.gpu;
      if ((p.gpuMemoryBytes ?? 0) > (cur.gpuMemoryBytes ?? 0)) cur.gpuMemoryBytes = p.gpuMemoryBytes;
    }
  }
  return [...map.values()]
    .map((p) => ({
      ...p,
      displayName: processLabel(p, p.count ?? 1),
    }))
    .sort((a, b) => compare(a, b, sort, dir));
}

function buildTree(list: ProcessRow[], sort: SortKey, dir: "asc" | "desc"): ViewRow[] {
  const byPid = new Map(list.map((p) => [p.pid, p]));
  const children = new Map<number, ProcessRow[]>();
  const roots: ProcessRow[] = [];
  for (const p of list) {
    const parent = p.parentPid;
    if (parent != null && byPid.has(parent)) {
      const arr = children.get(parent) ?? [];
      arr.push(p);
      children.set(parent, arr);
    } else {
      roots.push(p);
    }
  }
  const out: ViewRow[] = [];
  const seen = new Set<number>();
  const walk = (p: ProcessRow, depth: number) => {
    if (seen.has(p.pid)) return;
    seen.add(p.pid);
    out.push({
      ...p,
      depth,
      rowKey: `p:${p.pid}`,
      displayName: processLabel(p),
      memberPids: [p.pid],
    });
    const kids = (children.get(p.pid) ?? []).sort((a, b) => compare(a, b, sort, dir));
    for (const k of kids) walk(k, depth + 1);
  };
  roots.sort((a, b) => compare(a, b, sort, dir)).forEach((r) => walk(r, 0));
  for (const p of list) {
    if (!seen.has(p.pid)) walk(p, 0);
  }
  return out;
}
