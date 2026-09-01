import { useEffect, useRef, useState } from "react";

const FALLBACK_HEX = "#ff6b57";

interface SparklineProps {
  values: number[];
  color?: string;
  fill?: boolean;
  height?: number;
  max?: number;
}

export function Sparkline({ values, color = "var(--accent)", fill = true, height = 36, max }: SparklineProps) {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const draw = () => {
      try {
        const dpr = window.devicePixelRatio || 1;
        const width = canvas.clientWidth;
        canvas.width = Math.max(1, Math.floor(width * dpr));
        canvas.height = Math.max(1, Math.floor(height * dpr));
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
        ctx.clearRect(0, 0, width, height);
        if (!values || values.length < 2) return;

        const stroke = resolveCanvasColor(color);
        const hi = max ?? Math.max(1, ...values);
        ctx.beginPath();
        values.forEach((v, i) => {
          const x = (i / (values.length - 1)) * (width - 2) + 1;
          const y = height - 2 - (Math.min(v, hi) / hi) * (height - 4);
          if (i === 0) ctx.moveTo(x, y);
          else ctx.lineTo(x, y);
        });
        ctx.strokeStyle = stroke;
        ctx.lineWidth = 1.4;
        ctx.stroke();
        if (fill) {
          ctx.lineTo(width - 1, height);
          ctx.lineTo(1, height);
          ctx.closePath();
          const g = ctx.createLinearGradient(0, 0, 0, height);
          g.addColorStop(0, withAlpha(stroke, 0.28));
          g.addColorStop(1, withAlpha(stroke, 0));
          ctx.fillStyle = g;
          ctx.fill();
        }
      } catch {
        // Canvas color parsing throws; never let it take down React.
      }
    };

    draw();
    const ro = new ResizeObserver(draw);
    ro.observe(canvas);
    return () => ro.disconnect();
  }, [values, color, fill, height, max]);

  return <canvas ref={ref} style={{ width: "100%", height }} aria-hidden />;
}

function resolveCanvasColor(color: string | undefined): string {
  const raw = (color ?? "").trim() || FALLBACK_HEX;
  if (isHex(raw)) return normalizeHex(raw);
  if (!raw.includes("var(") && isConcreteColor(raw)) return raw;

  const fromCss = resolveCssColor(raw);
  if (fromCss) {
    if (isHex(fromCss)) return normalizeHex(fromCss);
    if (isConcreteColor(fromCss)) return fromCss;
  }

  return FALLBACK_HEX;
}

function resolveCssColor(value: string): string {
  const varMatch = value.match(/var\(\s*(--[\w-]+)/);
  if (varMatch) {
    const fromRoot = getComputedStyle(document.documentElement).getPropertyValue(varMatch[1]).trim();
    if (fromRoot && !fromRoot.includes("var(")) return fromRoot;
  }

  try {
    const probe = document.createElement("span");
    probe.style.color = value;
    document.documentElement.appendChild(probe);
    const computed = getComputedStyle(probe).color;
    probe.remove();
    if (computed && computed !== "rgba(0, 0, 0, 0)" && computed !== "transparent") {
      return computed;
    }
  } catch {
    // ignore
  }
  return "";
}

function isHex(color: string): boolean {
  return /^#([0-9a-f]{3}|[0-9a-f]{4}|[0-9a-f]{6}|[0-9a-f]{8})$/i.test(color);
}

function normalizeHex(hex: string): string {
  const n = hex.slice(1);
  if (n.length === 3 || n.length === 4) {
    return `#${[...n.slice(0, 3)].map((c) => c + c).join("")}`;
  }
  return `#${n.slice(0, 6)}`;
}

function isConcreteColor(color: string): boolean {
  return /^(rgb|rgba|hsl|hsla|oklch|lab|lch|hwb)\(/i.test(color) && !color.includes("var(");
}

function withAlpha(color: string, alpha: number): string {
  if (isHex(color)) return hexAlpha(color, alpha);
  const comma = color.match(/^rgba?\(\s*([\d.]+)\s*,\s*([\d.]+)\s*,\s*([\d.]+)/i);
  if (comma) return `rgba(${comma[1]}, ${comma[2]}, ${comma[3]}, ${alpha})`;
  const space = color.match(/^rgba?\(\s*([\d.]+)\s+([\d.]+)\s+([\d.]+)/i);
  if (space) return `rgba(${space[1]}, ${space[2]}, ${space[3]}, ${alpha})`;
  return hexAlpha(FALLBACK_HEX, alpha);
}

function hexAlpha(hex: string, a: number): string {
  const n = hex.replace("#", "");
  const r = Number.parseInt(n.slice(0, 2), 16);
  const g = Number.parseInt(n.slice(2, 4), 16);
  const b = Number.parseInt(n.slice(4, 6), 16);
  if ([r, g, b].some((c) => Number.isNaN(c))) return `rgba(255, 107, 87, ${a})`;
  return `rgba(${r}, ${g}, ${b}, ${a})`;
}

interface AreaChartProps {
  series: { name: string; values: number[]; color: string }[];
  height?: number;
  max?: number;
  unit?: string;
  times?: number[];
  intervalMs?: number;
}

export function AreaChart({ series, height = 220, max, unit = "%", times, intervalMs = 1000 }: AreaChartProps) {
  const ref = useRef<HTMLCanvasElement>(null);
  const wrap = useRef<HTMLDivElement>(null);
  const [tip, setTip] = useState<{ x: number; y: number; text: string } | null>(null);

  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const draw = () => {
    try {
      const dpr = window.devicePixelRatio || 1;
      const width = canvas.clientWidth;
      canvas.width = Math.max(1, Math.floor(width * dpr));
      canvas.height = Math.max(1, Math.floor(height * dpr));
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.clearRect(0, 0, width, height);
      const pad = { l: 36, r: 10, t: 12, b: 22 };
      const hi = max ?? Math.max(1, ...(series ?? []).flatMap((s) => s.values ?? []), 1);
      ctx.strokeStyle = "rgb(255 255 255 / 0.06)";
      ctx.fillStyle = "rgb(157 163 174 / 0.8)";
      ctx.font = "11px Geist Mono Variable, ui-monospace";
      for (let i = 0; i <= 4; i++) {
        const y = pad.t + ((height - pad.t - pad.b) * i) / 4;
        ctx.beginPath();
        ctx.moveTo(pad.l, y);
        ctx.lineTo(width - pad.r, y);
        ctx.stroke();
        const label = `${Math.round(hi * (1 - i / 4))}${unit}`;
        ctx.fillText(label, 4, y + 4);
      }
      for (const s of series ?? []) {
        if (!s.values || s.values.length < 2) continue;
        const stroke = resolveCanvasColor(s.color);
        ctx.beginPath();
        s.values.forEach((v, i) => {
          const x = pad.l + (i / (s.values.length - 1)) * (width - pad.l - pad.r);
          const y = pad.t + (1 - Math.min(v, hi) / hi) * (height - pad.t - pad.b);
          if (i === 0) ctx.moveTo(x, y);
          else ctx.lineTo(x, y);
        });
        ctx.strokeStyle = stroke;
        ctx.lineWidth = 1.6;
        ctx.stroke();
      }
    } catch {
      // Canvas color parsing throws; never let it take down React.
    }
  };

    draw();
    const ro = new ResizeObserver(draw);
    ro.observe(canvas);
    return () => ro.disconnect();
  }, [series, height, max, unit]);

  const onMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = ref.current;
    const longest = Math.max(0, ...(series ?? []).map((s) => s.values.length));
    if (!canvas || longest < 2) return;
    const rect = canvas.getBoundingClientRect();
    const padL = 36;
    const padR = 10;
    const x = e.clientX - rect.left;
    const inner = rect.width - padL - padR;
    const t = Math.max(0, Math.min(1, (x - padL) / Math.max(1, inner)));
    const idx = Math.round(t * (longest - 1));
    const clock = (() => {
      if (times?.[idx]) {
        const ms = times[idx] > 1e12 ? times[idx] : times[idx] * 1000;
        return new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
      }
      const ago = (longest - 1 - idx) * intervalMs;
      const s = Math.round(ago / 1000);
      if (s < 60) return `${s}s ago`;
      if (s < 3600) return `${Math.round(s / 60)}m ago`;
      return `${Math.round(s / 3600)}h ago`;
    })();
    const parts = (series ?? [])
      .map((s) => {
        const v = s.values[idx];
        if (v == null || Number.isNaN(v)) return null;
        const shown = unit === "%" ? `${v.toFixed(0)}${unit}` : `${v.toFixed(v >= 10 ? 0 : 2)}${unit}`;
        return `${s.name} ${shown}`;
      })
      .filter(Boolean);
    setTip({ x, y: e.clientY - rect.top, text: `${clock} · ${parts.join(" · ")}` });
  };

  return (
    <div ref={wrap} className="relative">
      <canvas
        ref={ref}
        style={{ width: "100%", height }}
        role="img"
        aria-label="Time series chart"
        onMouseMove={onMove}
        onMouseLeave={() => setTip(null)}
      />
      {tip && (
        <div
          className="pointer-events-none absolute z-10 rounded-md border border-line bg-surface-2 px-2 py-1 text-[11px] text-ink-soft num"
          style={{ left: Math.min(tip.x + 10, (wrap.current?.clientWidth ?? 400) - 180), top: Math.max(0, tip.y - 28) }}
        >
          {tip.text}
        </div>
      )}
      <div className="flex gap-4 mt-2">
        {series.map((s) => (
          <span key={s.name} className="text-xs text-ink-soft flex items-center gap-2">
            <i className="inline-block w-2 h-2 rounded-full" style={{ background: s.color }} />
            {s.name}
          </span>
        ))}
      </div>
    </div>
  );
}
