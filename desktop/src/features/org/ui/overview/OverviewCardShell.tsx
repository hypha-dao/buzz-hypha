import type { ReactNode } from "react";

import { cn } from "@/shared/lib/cn";
import { Card } from "@/shared/ui/card";

import { overviewCardDelayMs } from "./overviewMotion";

import "./overviewMotion.css";

type OverviewCardShellProps = {
  children: ReactNode;
  className?: string;
  enterIndex: number;
  testId?: string;
  /** Spans both columns of a two-column grid. */
  wide?: boolean;
};

/** One overview card: settles on enter, lifts on hover and keyboard focus. */
export function OverviewCardShell({
  children,
  className,
  enterIndex,
  testId,
  wide = false,
}: OverviewCardShellProps) {
  return (
    <div
      className={cn(
        "org-overview-lift h-full min-w-0",
        wide && "md:col-span-2",
      )}
    >
      <Card
        className={cn(
          "org-overview-settle flex h-full flex-col rounded-2xl p-6",
          className,
        )}
        data-testid={testId}
        style={{ animationDelay: `${overviewCardDelayMs(enterIndex)}ms` }}
      >
        {children}
      </Card>
    </div>
  );
}
