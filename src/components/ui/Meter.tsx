import { motion, useReducedMotion } from "motion/react";

export function Meter({ value, className }: { value: number; className?: string }) {
  const reduce = useReducedMotion();
  const scaleX = Math.max(0, Math.min(1, value / 100));
  return (
    <div className={`meter ${className ?? ""}`}>
      <motion.span
        initial={false}
        style={{ originX: 0, originY: 0.5 }}
        animate={{ scaleX }}
        transition={reduce ? { duration: 0 } : { duration: 0.5, ease: [0.16, 1, 0.3, 1] }}
      />
    </div>
  );
}
