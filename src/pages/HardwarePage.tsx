import { useEffect, useState } from "react";
import { motion } from "motion/react";
import { Rise } from "../components/ui/motion";
import { detectVendor, VendorMark } from "../components/ui/VendorMark";
import { getHardware } from "../lib/api";
import { formatBytes, formatDuration, formatTemp } from "../lib/format";
import type { HardwareInfo, HwDevice, Snapshot } from "../lib/types";
import { useAppStore } from "../store/app-store";

export function HardwarePage() {
  const snapshot = useAppStore((s) => s.snapshot);
  const unit = useAppStore((s) => s.settings.temperatureUnit);
  const [hw, setHw] = useState<HardwareInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const load = () =>
      getHardware()
        .then((info) => {
          setHw(info);
          setError(null);
        })
        .catch((err: unknown) => setError(String(err)));
    load();
    const t = window.setInterval(load, 20000);
    return () => window.clearInterval(t);
  }, []);

  const inventory = hw
    ? withSnapshotFallback(hw, snapshot, unit)
    : snapshot
      ? withSnapshotFallback(blankHardware(), snapshot, unit)
      : null;

  if (!inventory) {
    return (
      <div className="card p-6 max-w-lg">
        <h1 className="text-2xl font-semibold">Hardware</h1>
        <p className="text-ink-soft mt-2">
          {error ? error : "Reading hardware inventory. This stays on the machine."}
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-4 max-w-[1100px]">
      <header>
        <h1 className="text-2xl font-semibold tracking-tight">Hardware</h1>
        <p className="text-sm text-ink-soft mt-1">
          {inventory.hostname} · {inventory.osName} {inventory.osVersion} · up {formatDuration(inventory.uptimeSec)}
        </p>
      </header>
      {error && <p className="text-sm text-ink-soft">{error}</p>}
      <div className="grid md:grid-cols-2 gap-3">
        {inventory.cpu ? (
          <Rise delay={0.02}>
            <DeviceCard title="CPU" device={inventory.cpu} live={snapshot?.cpu.brand} />
          </Rise>
        ) : (
          <Missing title="CPU" />
        )}
        {inventory.gpus.length > 0 ? (
          inventory.gpus.map((g, i) => (
            <Rise key={`${g.name}-${i}`} delay={0.06 + i * 0.04}>
              <DeviceCard title="GPU" device={g} live={snapshot?.gpu.name} />
            </Rise>
          ))
        ) : (
          <Missing title="GPU" />
        )}
        {inventory.motherboard && <DeviceCard title="Motherboard" device={inventory.motherboard} />}
        {inventory.memoryModules.length > 0 ? (
          <section className="card p-4">
            <h2 className="font-semibold mb-2">Memory</h2>
            <ul className="flex flex-col gap-2">
              {inventory.memoryModules.map((m, i) => {
                const vendor = detectVendor(m.manufacturer, m.name, m.detail);
                return (
                  <li key={i} className="text-[13px] flex items-start gap-3">
                    {vendor ? <VendorMark vendor={vendor} size={24} /> : null}
                    <div className="min-w-0 flex-1">
                      <div>{m.manufacturer ? `${m.manufacturer} · ${m.name}` : m.name}</div>
                      <div className="text-ink-soft">{m.detail}</div>
                    </div>
                  </li>
                );
              })}
            </ul>
          </section>
        ) : (
          <Missing title="Memory" />
        )}
        {inventory.storage.length > 0 ? (
          inventory.storage.map((s, i) => <DeviceCard key={`${s.name}-${i}`} title="Storage" device={s} />)
        ) : (
          <Missing title="Storage" />
        )}
        {inventory.networkAdapters.map((n, i) => (
          <DeviceCard key={`${n.name}-${i}`} title="Network" device={n} />
        ))}
        {inventory.monitors.map((m, i) => (
          <DeviceCard key={`${m.name}-${i}`} title="Display" device={m} />
        ))}
      </div>
    </div>
  );
}

function blankHardware(): HardwareInfo {
  return {
    cpu: null,
    gpus: [],
    motherboard: null,
    memoryModules: [],
    storage: [],
    networkAdapters: [],
    monitors: [],
    osName: "Windows",
    osVersion: "",
    hostname: "This PC",
    uptimeSec: 0,
  };
}

function withSnapshotFallback(hw: HardwareInfo, snapshot: Snapshot | null, unit: string): HardwareInfo {
  const next = { ...hw };
  if (!next.cpu && snapshot?.cpu) {
    next.cpu = {
      name: snapshot.cpu.brand || "CPU",
      manufacturer: null,
      detail: `${snapshot.cpu.coreCount} cores / ${snapshot.cpu.logicalCount} threads`,
      extras: snapshot.cpu.frequencyMhz
        ? [{ label: "Clock", value: `${snapshot.cpu.frequencyMhz} MHz` }]
        : [],
    };
  }
  if (next.cpu && snapshot?.cpu.temperatureC != null) {
    next.cpu = {
      ...next.cpu,
      extras: [
        { label: "Temperature", value: formatTemp(snapshot.cpu.temperatureC, unit) },
        ...next.cpu.extras.filter((e) => e.label !== "Temperature"),
      ],
    };
  }
  if (next.gpus.length === 0 && snapshot?.gpu && snapshot.gpu.name && snapshot.gpu.name !== "Unavailable on this system") {
    const fromAdapters =
      snapshot.gpu.adapters?.map((a) => ({
        name: a.name,
        manufacturer: a.vendor || null,
        detail: a.memoryTotalBytes
          ? `${formatBytes(a.memoryTotalBytes)} VRAM`
          : "VRAM unavailable on this system",
        extras: [],
      })) ?? [];
    next.gpus =
      fromAdapters.length > 0
        ? fromAdapters
        : [
            {
              name: snapshot.gpu.name,
              manufacturer: snapshot.gpu.vendor || null,
              detail: snapshot.gpu.unavailableReason ?? snapshot.gpu.activity,
              extras: [],
            },
          ];
  }
  if (snapshot?.gpu && next.gpus.length > 0) {
    next.gpus = next.gpus.map((g) => {
      const adapter = snapshot.gpu.adapters?.find((a) => namesOverlap(g.name, a.name));
      const primary = namesOverlap(g.name, snapshot.gpu.name);
      const total = adapter?.memoryTotalBytes ?? (primary ? snapshot.gpu.memoryTotalBytes : null);
      const tempC = adapter?.temperatureC ?? (primary ? snapshot.gpu.temperatureC : null);
      const extras = [...g.extras];
      if (tempC != null) {
        const withoutTemp = extras.filter((e) => e.label !== "Temperature");
        extras.length = 0;
        extras.push({ label: "Temperature", value: formatTemp(tempC, unit) }, ...withoutTemp);
      }
      if (total) {
        const withoutVram = extras.filter((e) => e.label !== "Reported VRAM" && e.label !== "VRAM");
        extras.length = 0;
        extras.push({ label: "VRAM", value: formatBytes(total) }, ...withoutVram);
      }
      return { ...g, extras };
    });
  }
  if (next.memoryModules.length === 0 && snapshot?.memory) {
    next.memoryModules = [
      {
        name: `${formatBytes(snapshot.memory.totalBytes)} installed`,
        manufacturer: null,
        detail: `${formatBytes(snapshot.memory.usedBytes)} in use right now`,
        extras: [],
      },
    ];
  }
  if (next.storage.length === 0 && snapshot?.disks?.length) {
    next.storage = snapshot.disks.map((d) => ({
      name: d.name || d.mount,
      manufacturer: null,
      detail: `${formatBytes(d.totalBytes)} · ${d.mount}`,
      extras: [],
    }));
  }
  return next;
}

function Missing({ title }: { title: string }) {
  return (
    <section className="card p-4">
      <div className="text-[11px] text-ink-mute">{title}</div>
      <h2 className="font-semibold mt-1">Unavailable on this system</h2>
      <p className="text-[13px] text-ink-soft mt-1">Windows did not report this device to the inventory.</p>
    </section>
  );
}

function namesOverlap(a: string, b: string) {
  const left = a.trim().toLowerCase();
  const right = b.trim().toLowerCase();
  if (!left || !right) return false;
  return left === right || left.includes(right) || right.includes(left);
}

function DeviceCard({ title, device, live }: { title: string; device: HwDevice; live?: string }) {
  const vendor = detectVendor(device.manufacturer, device.name, device.detail, live);
  const showLive = live && live.trim().toLowerCase() !== device.name.trim().toLowerCase();
  const manufacturer = device.manufacturer?.trim() ?? "";
  const detail = device.detail?.trim() ?? "";
  const nameL = device.name.trim().toLowerCase();
  const manufacturerL = manufacturer.toLowerCase();
  const detailL = detail.toLowerCase();
  const manufacturerIsName = Boolean(manufacturer) && manufacturerL === nameL;
  const detailIsManufacturer = Boolean(manufacturer && detail) && detailL === manufacturerL;
  const manufacturerInDetail =
    Boolean(manufacturer && detail) && !detailIsManufacturer && detailL.includes(manufacturerL);
  const manufacturerInName =
    Boolean(manufacturer) && manufacturerL.length >= 2 && nameL.includes(manufacturerL);
  const showManufacturer =
    Boolean(manufacturer) && !manufacturerIsName && !manufacturerInDetail && !manufacturerInName;
  const showDetail = Boolean(detail) && !detailIsManufacturer;
  return (
    <motion.section className="card p-4" whileHover={{ y: -2 }} transition={{ type: "spring", stiffness: 420, damping: 28 }}>
      <div className="flex items-start gap-3">
        {vendor ? <VendorMark vendor={vendor} /> : null}
        <div className="min-w-0 flex-1">
          <div className="text-[11px] text-ink-mute">{title}</div>
          <h2 className="font-semibold mt-1 leading-tight">{device.name}</h2>
          {showManufacturer && <p className="text-[13px] text-ink-soft">{manufacturer}</p>}
        </div>
      </div>
      {showDetail && <p className="text-[13px] text-ink-soft mt-2">{detail}</p>}
      {showLive && <p className="text-[12px] text-ink-mute mt-2">Also reported as {live}</p>}
      {device.extras.length > 0 && (
        <dl className="mt-3 grid grid-cols-2 gap-2 text-[13px]">
          {device.extras.map((f) => (
            <div key={f.label}>
              <dt className="text-[11px] text-ink-mute">{f.label}</dt>
              <dd>{f.value}</dd>
            </div>
          ))}
        </dl>
      )}
    </motion.section>
  );
}
