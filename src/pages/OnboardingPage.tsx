import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { useEffect, useState } from "react";
import { completeOnboarding } from "../lib/api";
import type { AppSettings } from "../lib/types";
import { useAppStore } from "../store/app-store";

export function OnboardingPage() {
  const settings = useAppStore((s) => s.settings);
  const setSettings = useAppStore((s) => s.setSettings);
  const [step, setStep] = useState(0);
  const [draft, setDraft] = useState<AppSettings>(settings);
  const reduce = useReducedMotion();

  useEffect(() => {
    setDraft(settings);
  }, [settings]);

  const finish = async () => {
    const saved = await completeOnboarding(draft);
    setSettings(saved);
  };

  return (
    <div className="h-full grid place-items-center p-8">
      <motion.div
        className="card w-full max-w-[560px] p-8 overflow-hidden"
        initial={reduce ? false : { opacity: 0, y: 18, scale: 0.98 }}
        animate={{ opacity: 1, y: 0, scale: 1 }}
        transition={{ duration: 0.45, ease: [0.16, 1, 0.3, 1] }}
      >
        <p className="text-[12px] text-ink-mute">Step {step + 1} of 3</p>
        <AnimatePresence mode="wait">
          <motion.div
            key={step}
            initial={reduce ? false : { opacity: 0, x: 16 }}
            animate={{ opacity: 1, x: 0 }}
            exit={reduce ? undefined : { opacity: 0, x: -16 }}
            transition={{ duration: 0.28, ease: [0.16, 1, 0.3, 1] }}
          >
            {step === 0 && (
              <>
                <h1 className="text-3xl font-semibold tracking-tight mt-2 leading-[1.15] pb-1">
                  What the f*ck is your PC doing?
                </h1>
                <p className="text-ink-soft mt-4 leading-relaxed">
                  This watches CPU, GPU, memory, disk, network, and processes, then explains the load in English. It is a translator, not a Task Manager clone.
                </p>
                <ul className="mt-4 text-[14px] text-ink-soft flex flex-col gap-2">
                  <li>Live load and who is causing it</li>
                  <li>Hardware identity from Windows</li>
                  <li>A local history so you can ask what happened earlier</li>
                </ul>
                <button className="btn primary mt-6" onClick={() => setStep(1)}>
                  Continue
                </button>
              </>
            )}
            {step === 1 && (
              <>
                <h1 className="text-2xl font-semibold tracking-tight mt-2">Privacy</h1>
                <p className="text-ink-soft mt-4 leading-relaxed">
                  Data stays on this PC. No account, no cloud, no analytics. History lives in a local SQLite file and is pruned on a schedule you control.
                </p>
                <div className="flex gap-2 mt-6">
                  <button className="btn" onClick={() => setStep(0)}>Back</button>
                  <button className="btn primary" onClick={() => setStep(2)}>Continue</button>
                </div>
              </>
            )}
            {step === 2 && (
              <>
                <h1 className="text-2xl font-semibold tracking-tight mt-2">Alerts</h1>
                <p className="text-ink-soft mt-4 leading-relaxed">
                  Alerts wait for a condition to last. A one-second spike will not bother you. Optional Windows toasts are only for sustained warnings.
                </p>
                <div className="mt-4 flex flex-col gap-3 text-[14px]">
                  <label className="flex justify-between items-center gap-4">
                    Windows notifications
                    <input
                      type="checkbox"
                      checked={draft.notificationsEnabled}
                      onChange={(e) => setDraft({ ...draft, notificationsEnabled: e.target.checked })}
                    />
                  </label>
                  <label className="flex justify-between items-center gap-4">
                    Notify for
                    <select
                      className="field w-40"
                      value={draft.notifyMinSeverity}
                      onChange={(e) => setDraft({ ...draft, notifyMinSeverity: e.target.value })}
                    >
                      <option value="warning">Warnings and critical</option>
                      <option value="critical">Critical only</option>
                      <option value="off">Off</option>
                    </select>
                  </label>
                  <label className="flex justify-between items-center gap-4">
                    Hide to tray on close
                    <input
                      type="checkbox"
                      checked={draft.minimizeToTray}
                      onChange={(e) => setDraft({ ...draft, minimizeToTray: e.target.checked })}
                    />
                  </label>
                </div>
                <div className="flex gap-2 mt-6">
                  <button className="btn" onClick={() => setStep(1)}>Back</button>
                  <button className="btn primary" onClick={() => void finish()}>
                    Open the dashboard
                  </button>
                </div>
              </>
            )}
          </motion.div>
        </AnimatePresence>
      </motion.div>
    </div>
  );
}
