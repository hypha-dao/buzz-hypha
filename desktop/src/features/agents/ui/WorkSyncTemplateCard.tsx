import { GitBranch } from "lucide-react";

import { Badge } from "@/shared/ui/badge";

export const WORK_SYNC_TEMPLATE_ID = "template:work-sync";
export const WORK_SYNC_TEMPLATE_NAME = "Work sync";
export const WORK_SYNC_TEMPLATE_DESCRIPTION =
  "Pushes your work branches and posts progress notes on the tickets you hold.";
export const WORK_SYNC_TEMPLATE_UNAVAILABLE_COPY =
  "Not available yet. Work sync arrives with the work log, in a later release.";

/**
 * The one agent template the Hypha desktop offers (Design § Work sync): a
 * member's own `buzz-acp` agent that watches their checkouts and reports
 * progress on work they hold. It ships in wave 6; until then the card is
 * present but disabled so the door already says what it will offer.
 *
 * Accessibility: one button owns the actionable label. It is `aria-disabled`
 * rather than natively `disabled` so it stays in the tab order, and the
 * description plus the unavailability note are wired through
 * `aria-describedby` so keyboard and screen-reader users hear why it does
 * nothing. It has no click handler: there is nothing to launch yet.
 */
export function WorkSyncTemplateCard() {
  const descriptionId = `${WORK_SYNC_TEMPLATE_ID}-description`;
  return (
    <div
      className="relative aspect-[4/5] w-full min-w-0 overflow-hidden rounded-2xl border border-dashed border-border/70 bg-muted/30 text-left shadow-xs"
      data-testid="work-sync-template-card"
    >
      <button
        aria-describedby={descriptionId}
        aria-disabled="true"
        aria-label={`${WORK_SYNC_TEMPLATE_NAME} template`}
        className="absolute inset-0 z-10 cursor-not-allowed rounded-2xl focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
        data-testid="work-sync-template-button"
        type="button"
      />

      <div className="pointer-events-none relative z-20 flex h-full w-full min-w-0 flex-col items-center justify-center gap-5 px-4 pb-12 text-center">
        <div className="flex h-24 w-24 items-center justify-center rounded-[28%] border-[3px] border-background bg-muted text-muted-foreground/70">
          <GitBranch aria-hidden="true" className="h-9 w-9" />
        </div>
      </div>

      <div className="pointer-events-none absolute top-3 right-3 z-30">
        <Badge variant="outline">Coming soon</Badge>
      </div>

      <div
        className="pointer-events-none absolute right-3 bottom-3 left-3 z-30 flex min-w-0 flex-col gap-0.5 text-left text-sm leading-5"
        id={descriptionId}
      >
        <span className="min-w-0 truncate font-semibold text-muted-foreground tracking-normal">
          {WORK_SYNC_TEMPLATE_NAME}
        </span>
        <span className="line-clamp-2 min-w-0 text-xs font-normal text-muted-foreground/80">
          {WORK_SYNC_TEMPLATE_DESCRIPTION}
        </span>
        <span className="sr-only">{WORK_SYNC_TEMPLATE_UNAVAILABLE_COPY}</span>
      </div>
    </div>
  );
}
