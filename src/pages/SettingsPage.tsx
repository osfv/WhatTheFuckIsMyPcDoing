import { useEffect } from "react";
import { pauseMonitoring, saveSettings } from "../lib/api";
import type { AppSettings } from "../lib/types";
import { useAppStore } from "../store/app-store";

const accents = ["#FF6B57", "#5EEAD4", "#7EB6FF", "#F0B429", "#C4B5FD"];

export function SettingsPage() {
  const settings = useAppStore((s) => s.settings);
  const setSettings = useAppStore((s) => s.setSettings);
  const section = useAppStore((s) => s.focus.settingsSection);

  const patch = async (partial: Partial<AppSettings>) => {
    const next = { ...settings, ...partial };
    setSettings(next);
    await saveSettings(next);
  };

  useEffect(() => {
    if (!section) return;
    document.getElementById(`settings-${section}`)?.scrollIntoView({ block: "start" });
  }, [section]);

  return (
    <div className="flex flex-col gap-4 max-w-[720px]">
      <header>
        <h1 className="text-2xl font-semibold tracking-tight">Settings</h1>
        <p className="text-sm text-ink-soft mt-1">Everything here stays on this PC. There is no account.</p>
      </header>

      <section id="settings-general" className="card p-5 flex flex-col gap-3 scroll-mt-4">
        <h2 className="font-semibold">General</h2>
        <Toggle
          label="Start with Windows"
          checked={settings.startOnLogin}
          onChange={(v) => patch({ startOnLogin: v })}
        />
        <Toggle
          label="Close button hides to tray"
          checked={settings.minimizeToTray}
          onChange={(v) => patch({ minimizeToTray: v })}
        />
        <Toggle
          label="Pause monitoring"
          checked={settings.pauseMonitoring}
          onChange={(v) => {
            void pauseMonitoring(v).then(setSettings);
          }}
        />
      </section>

      <section id="settings-monitoring" className="card p-5 flex flex-col gap-3 scroll-mt-4">
        <h2 className="font-semibold">Monitoring</h2>
        <p className="text-[13px] text-ink-soft">Turn off sensors you do not care about. Disabled sensors show as unavailable.</p>
        <Toggle label="CPU" checked={settings.monitorCpu} onChange={(v) => patch({ monitorCpu: v })} />
        <Toggle label="GPU" checked={settings.monitorGpu} onChange={(v) => patch({ monitorGpu: v })} />
        <Toggle label="Memory" checked={settings.monitorMemory} onChange={(v) => patch({ monitorMemory: v })} />
        <Toggle label="Disk" checked={settings.monitorDisk} onChange={(v) => patch({ monitorDisk: v })} />
        <Toggle label="Network" checked={settings.monitorNetwork} onChange={(v) => patch({ monitorNetwork: v })} />
        <Toggle label="Processes" checked={settings.monitorProcesses} onChange={(v) => patch({ monitorProcesses: v })} />
        <label className="flex justify-between gap-4 text-[13px] items-center">
          Sample interval
          <select
            className="field w-40"
            value={settings.sampleIntervalMs}
            onChange={(e) => patch({ sampleIntervalMs: Number(e.target.value) })}
          >
            <option value={1000}>1 second</option>
            <option value={2000}>2 seconds</option>
            <option value={5000}>5 seconds</option>
          </select>
        </label>
      </section>

      <section id="settings-alerts" className="card p-5 flex flex-col gap-3 scroll-mt-4">
        <h2 className="font-semibold">Alerts</h2>
        <p className="text-[13px] text-ink-soft">
          Alerts wait for a threshold to last. Windows toasts fire only for sustained events at the severity you pick.
        </p>
        <Toggle
          label="Windows notifications"
          checked={settings.notificationsEnabled}
          onChange={(v) => patch({ notificationsEnabled: v })}
        />
        <label className="flex justify-between gap-4 text-[13px] items-center">
          Notify for
          <select
            className="field w-48"
            value={settings.notifyMinSeverity ?? "warning"}
            onChange={(e) => patch({ notifyMinSeverity: e.target.value })}
          >
            <option value="warning">Warnings and critical</option>
            <option value="critical">Critical only</option>
            <option value="off">Off</option>
          </select>
        </label>
        <NumberField
          label="CPU alert percent"
          value={settings.cpuAlertPercent}
          onChange={(v) => patch({ cpuAlertPercent: v })}
        />
        <NumberField
          label="CPU alert duration (seconds)"
          value={settings.cpuAlertSeconds}
          integer
          onChange={(v) => patch({ cpuAlertSeconds: v })}
        />
        <NumberField
          label="GPU temperature alert (°C)"
          value={settings.gpuTempAlertC}
          onChange={(v) => patch({ gpuTempAlertC: v })}
        />
        <NumberField
          label="RAM alert percent"
          value={settings.ramAlertPercent}
          onChange={(v) => patch({ ramAlertPercent: v })}
        />
        <NumberField
          label="RAM alert duration (seconds)"
          value={settings.ramAlertSeconds ?? 120}
          integer
          onChange={(v) => patch({ ramAlertSeconds: v })}
        />
        <NumberField
          label="Disk full percent"
          value={settings.diskFullPercent}
          onChange={(v) => patch({ diskFullPercent: v })}
        />
      </section>

      <section id="settings-history" className="card p-5 flex flex-col gap-3 scroll-mt-4">
        <h2 className="font-semibold">History</h2>
        <p className="text-[13px] text-ink-soft">
          Samples are stored locally about every 15 seconds, then pruned. The app does not write a raw dump every tick.
        </p>
        <label className="flex justify-between gap-4 text-[13px] items-center">
          Keep history
          <select
            className="field w-40"
            value={settings.historyRetentionDays}
            onChange={(e) => patch({ historyRetentionDays: Number(e.target.value) })}
          >
            <option value={1}>1 day</option>
            <option value={7}>7 days</option>
            <option value={30}>30 days</option>
          </select>
        </label>
      </section>

      <section id="settings-appearance" className="card p-5 flex flex-col gap-3 scroll-mt-4">
        <h2 className="font-semibold">Appearance</h2>
        <div className="flex gap-2">
          {accents.map((c) => (
            <button
              key={c}
              aria-label={`Accent ${c}`}
              className="w-8 h-8 rounded-full border border-line-strong"
              style={{ background: c, outline: settings.accentColor === c ? "2px solid white" : undefined }}
              onClick={() => patch({ accentColor: c })}
            />
          ))}
        </div>
        <label className="flex justify-between gap-4 text-[13px] items-center">
          Temperature
          <select
            className="field w-40"
            value={settings.temperatureUnit}
            onChange={(e) => patch({ temperatureUnit: e.target.value })}
          >
            <option value="celsius">Celsius</option>
            <option value="fahrenheit">Fahrenheit</option>
          </select>
        </label>
        <Toggle
          label="Reduce motion"
          checked={settings.reducedMotion}
          onChange={(v) => patch({ reducedMotion: v })}
        />
      </section>

      <section id="settings-discord" className="card p-5 flex flex-col gap-3 scroll-mt-4">
        <h2 className="font-semibold">Discord</h2>
        <Toggle
          label="Show on Discord"
          checked={settings.discordPresence}
          onChange={(v) => patch({ discordPresence: v })}
        />
        <p className="text-[13px] text-ink-soft leading-relaxed">
          Turn this on. Discord detecting the exe as a game is a question mark and a timer with no CPU line.
          With this on, your profile should show CPU and RAM under the name. The GitHub button is only visible
          to other people, not on your own profile. Discord has to be running.
        </p>
        <p className="text-[13px] text-ink-soft leading-relaxed">
          If the empty Playing tile stays, remove this app from Discord Settings, Activity Privacy, Registered Games.
          For a real icon, upload a 1024 image under the Discord app Rich Presence art assets.
        </p>
      </section>

      <section id="settings-privacy" className="card p-5 scroll-mt-4">
        <h2 className="font-semibold">Privacy</h2>
        <p className="text-[13px] text-ink-soft mt-2 leading-relaxed">
          Data stays on this PC. The app records CPU, GPU, memory, disk, network, process names, and hardware identifiers. History lives in a local SQLite file. Nothing is uploaded. There is no analytics channel because there is no analytics. Discord presence is off by default. If you turn it on, live CPU and RAM are sent to the Discord app on this PC.
        </p>
      </section>

      <section id="settings-advanced" className="card p-5 flex flex-col gap-3 scroll-mt-4">
        <h2 className="font-semibold">Advanced</h2>
        <p className="text-[13px] text-ink-soft leading-relaxed">
          Process CPU is divided by logical processors so 100% means one full machine, not one core. Chrome-style apps are grouped. Ending a process is a last resort and asks for confirm.
        </p>
      </section>
    </div>
  );
}

function Toggle({ label, checked, onChange }: { label: string; checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <label className="flex items-center justify-between gap-4 text-[13px]">
      {label}
      <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
    </label>
  );
}

function NumberField({
  label,
  value,
  onChange,
  integer,
}: {
  label: string;
  value: number;
  onChange: (v: number) => void;
  integer?: boolean;
}) {
  return (
    <label className="flex items-center justify-between gap-4 text-[13px]">
      {label}
      <input
        className="field w-28"
        type="number"
        step={integer ? 1 : undefined}
        value={Number.isFinite(value) ? value : 0}
        onChange={(e) => {
          const n = Number(e.target.value);
          if (!Number.isFinite(n)) return;
          onChange(integer ? Math.max(0, Math.round(n)) : n);
        }}
      />
    </label>
  );
}
