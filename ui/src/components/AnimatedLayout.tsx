import { motion, AnimatePresence } from "framer-motion";
import type { ReactNode } from "react";

interface AnimatedLayoutProps {
  children: ReactNode;
  layoutId?: string;
}

const panelVariants = {
  initial: { opacity: 0, scale: 0.95 },
  animate: { opacity: 1, scale: 1 },
  exit: { opacity: 0, scale: 0.95 },
};

const transition = {
  type: "spring" as const,
  stiffness: 300,
  damping: 30,
  mass: 0.8,
};

export function AnimatedPanel({ children, layoutId }: AnimatedLayoutProps) {
  return (
    <motion.div
      layoutId={layoutId}
      variants={panelVariants}
      initial="initial"
      animate="animate"
      exit="exit"
      transition={transition}
      className="h-full min-h-0"
    >
      {children}
    </motion.div>
  );
}

interface ViewTransitionProps {
  viewKey: string;
  children: ReactNode;
}

export function ViewTransition({ viewKey, children }: ViewTransitionProps) {
  return (
    <AnimatePresence mode="wait">
      <motion.div
        key={viewKey}
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        exit={{ opacity: 0, y: -8 }}
        transition={{ duration: 0.2, ease: "easeInOut" }}
        className="h-full"
      >
        {children}
      </motion.div>
    </AnimatePresence>
  );
}

interface SlideInProps {
  children: ReactNode;
  direction?: "left" | "right" | "up" | "down";
  delay?: number;
}

const directionOffsets = {
  left: { x: -20, y: 0 },
  right: { x: 20, y: 0 },
  up: { x: 0, y: -20 },
  down: { x: 0, y: 20 },
};

export function SlideIn({ children, direction = "left", delay = 0 }: SlideInProps) {
  const offset = directionOffsets[direction];
  return (
    <motion.div
      initial={{ opacity: 0, ...offset }}
      animate={{ opacity: 1, x: 0, y: 0 }}
      transition={{ duration: 0.25, delay, ease: "easeOut" }}
      className="h-full"
    >
      {children}
    </motion.div>
  );
}

interface PulseIndicatorProps {
  active: boolean;
  color?: string;
}

export function PulseIndicator({ active, color = "#4ADE80" }: PulseIndicatorProps) {
  return (
    <motion.span
      className="inline-block w-2 h-2 rounded-full"
      style={{ backgroundColor: color }}
      animate={
        active
          ? { scale: [1, 1.3, 1], opacity: [1, 0.7, 1] }
          : { scale: 1, opacity: 0.4 }
      }
      transition={
        active
          ? { repeat: Infinity, duration: 1.5, ease: "easeInOut" }
          : { duration: 0.3 }
      }
    />
  );
}