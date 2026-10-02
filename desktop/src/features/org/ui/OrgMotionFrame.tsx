import type { CSSProperties, ReactNode } from "react";

import { cn } from "@/shared/lib/cn";

import { overviewCardDelayMs } from "./overview/overviewMotion";

import "./overview/overviewMotion.css";

type OrgMotionFrameProps = {
  children: ReactNode;
  className?: string;
  enterIndex: number;
};

/** Same settle-and-lift entrance as an Overview card. */
export function OrgMotionFrame({
  children,
  className,
  enterIndex,
}: OrgMotionFrameProps) {
  return (
    <div className="org-overview-lift min-w-0">
      <div
        className={cn("org-overview-settle", className)}
        style={motionDelay(overviewCardDelayMs(enterIndex))}
      >
        {children}
      </div>
    </div>
  );
}

export function motionDelay(ms: number): CSSProperties {
  return { animationDelay: `${ms}ms` };
}
