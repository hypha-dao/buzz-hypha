import { cn } from "@/shared/lib/cn";

import {
  stateChipLabel,
  stateChipTone,
  type WorkItem,
} from "../../work/model";

type StateChipProps = {
  item: Pick<WorkItem, "state" | "type">;
  who?: string | null;
};

export function StateChip({ item, who }: StateChipProps) {
  const tone = stateChipTone(item.state);
  const label = stateChipLabel(item);
  const whoLabel =
    who && (item.state === "accepted" || item.state === "in_review")
      ? who
      : who && item.state === "offered"
        ? who
        : null;

  return (
    <span
      className={cn(
        "inline-flex shrink-0 items-center gap-1 rounded-full px-2.5 py-0.5 text-2xs font-medium",
        tone === "done" && "bg-muted text-muted-foreground",
        tone === "doing" && "bg-primary/10 text-primary",
        tone === "waiting" && "border border-border text-muted-foreground",
        tone === "open" && "bg-foreground text-background",
      )}
      data-state={item.state}
      data-testid="org-state-chip"
    >
      {label}
      {whoLabel ? (
        <span className="font-normal opacity-80" data-testid="org-state-chip-who">
          {whoLabel}
        </span>
      ) : null}
    </span>
  );
}
