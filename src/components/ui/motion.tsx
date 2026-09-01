import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import type { ReactNode } from "react";

const ease = [0.16, 1, 0.3, 1] as const;

function clearRestingY(
  t: { y?: string | number; scale?: string | number },
  generated: string,
) {
  return Number(t.y) === 0 && (t.scale === 1 || t.scale === undefined) ? "none" : generated;
}

export function PageFade({ id, children }: { id: string; children: ReactNode }) {
  const reduce = useReducedMotion();
  return (
    <AnimatePresence mode="wait">
      <motion.div
        key={id}
        initial={reduce ? false : { opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        exit={reduce ? undefined : { opacity: 0, y: -8 }}
        transition={{ duration: reduce ? 0 : 0.32, ease }}
        transformTemplate={clearRestingY}
        className="h-full"
      >
        {children}
      </motion.div>
    </AnimatePresence>
  );
}

export function Rise({
  children,
  delay = 0,
  className,
}: {
  children: ReactNode;
  delay?: number;
  className?: string;
}) {
  const reduce = useReducedMotion();
  return (
    <motion.div
      className={className}
      initial={reduce ? false : { opacity: 0, y: 14 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: reduce ? 0 : 0.42, delay: reduce ? 0 : delay, ease }}
      transformTemplate={clearRestingY}
    >
      {children}
    </motion.div>
  );
}

export { ease };
