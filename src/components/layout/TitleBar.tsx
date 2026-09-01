import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, X } from "@phosphor-icons/react";
import { AnimatedNumber } from "../ui/AnimatedNumber";
import { useAppStore } from "../../store/app-store";

function windowApi() {
  try {
    return getCurrentWindow();
  } catch {
    return null;
  }
}

export function TitleBar() {
  const snapshot = useAppStore((s) => s.snapshot);
  const paused = useAppStore((s) => s.settings.pauseMonitoring);

  return (
    <header className="titlebar" data-tauri-drag-region>
      <div className="flex items-center gap-3 text-[12px] text-ink-mute">
        <span className="font-medium text-ink-soft">What The F*ck Is My PC Doing?</span>
        {snapshot && (
          <span className="num flex items-center gap-1 no-drag" data-tauri-drag-region="false">
            <button
              className="title-metric"
              data-tauri-drag-region="false"
              onClick={() => useAppStore.getState().toggleResourcePanel("cpu")}
            >
              CPU <AnimatedNumber value={snapshot.cpu.utilization} suffix="%" />
            </button>
            <span className="mx-1 opacity-40">/</span>
            <button
              className="title-metric"
              data-tauri-drag-region="false"
              onClick={() => useAppStore.getState().toggleResourcePanel("memory")}
            >
              RAM <AnimatedNumber value={snapshot.memory.utilization} suffix="%" />
            </button>
            {snapshot.gpu.utilization != null && (
              <>
                <span className="mx-1 opacity-40">/</span>
                <button
                  className="title-metric"
                  data-tauri-drag-region="false"
                  onClick={() => useAppStore.getState().toggleResourcePanel("gpu")}
                >
                  GPU <AnimatedNumber value={snapshot.gpu.utilization} suffix="%" />
                </button>
              </>
            )}
          </span>
        )}
        {paused && <span className="chip warn no-drag" data-tauri-drag-region="false">Paused</span>}
      </div>
      <div className="no-drag flex items-center" data-tauri-drag-region="false">
        <button
          className="win-btn"
          data-tauri-drag-region="false"
          aria-label="Minimize"
          onClick={() => void windowApi()?.minimize()}
        >
          <Minus size={14} />
        </button>
        <button
          className="win-btn"
          data-tauri-drag-region="false"
          aria-label="Maximize"
          onClick={() => void windowApi()?.toggleMaximize()}
        >
          <Square size={12} />
        </button>
        <button
          className="win-btn close"
          data-tauri-drag-region="false"
          aria-label="Close"
          onClick={() => void windowApi()?.close()}
        >
          <X size={14} />
        </button>
      </div>
    </header>
  );
}
