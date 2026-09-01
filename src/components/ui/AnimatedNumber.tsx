import { animate, useReducedMotion } from "motion/react";
import { useEffect, useRef, useState } from "react";

export function AnimatedNumber({
  value,
  suffix = "",
  digits = 0,
  className,
}: {
  value: number;
  suffix?: string;
  digits?: number;
  className?: string;
}) {
  const reduce = useReducedMotion();
  const safe = Number.isFinite(value) ? value : 0;
  const [text, setText] = useState(() => format(safe, digits, suffix));
  const current = useRef(safe);

  useEffect(() => {
    if (reduce) {
      current.current = safe;
      setText(format(safe, digits, suffix));
      return;
    }
    const ctrl = animate(current.current, safe, {
      duration: 0.45,
      ease: [0.16, 1, 0.3, 1],
      onUpdate: (latest) => {
        current.current = latest;
        setText(format(latest, digits, suffix));
      },
    });
    return () => ctrl.stop();
  }, [safe, reduce, digits, suffix]);

  return <span className={className}>{text}</span>;
}

function format(value: number, digits: number, suffix: string) {
  return `${value.toFixed(digits)}${suffix}`;
}
