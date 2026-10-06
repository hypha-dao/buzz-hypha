import { cn } from "@/shared/lib/cn";
import { Button } from "@/shared/ui/button";

import type { DirectionSlug } from "../../commands";

import { DirectionMark } from "./DirectionMark";
import { OverviewCardShell } from "./OverviewCardShell";
import { DIRECTION_LABEL, NOT_SET_YET } from "./overviewCopy";
import {
  overviewLineDelayMs,
  overviewMarkDelayMs,
  overviewRuleDelayMs,
  overviewStampDelayMs,
} from "./overviewMotion";
import type { DirectionSlot } from "./parseOverview";

type DirectionCardsProps = {
  className?: string;
  slots: readonly DirectionSlot[];
  onOpenDirection: (slug: DirectionSlug) => void;
};

export function DirectionCards({
  className,
  slots,
  onOpenDirection,
}: DirectionCardsProps) {
  return (
    <section
      aria-labelledby="org-direction-heading"
      className={cn("flex flex-col gap-4", className)}
    >
      <h2
        className="text-xs font-medium uppercase tracking-wider text-muted-foreground"
        id="org-direction-heading"
      >
        Direction
      </h2>
      <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
        {slots.map((slot, index) => (
          <DirectionCard
            enterIndex={index}
            key={slot.slug}
            onOpenDirection={onOpenDirection}
            slot={slot}
          />
        ))}
      </div>
    </section>
  );
}

function DirectionCard({
  enterIndex,
  onOpenDirection,
  slot,
}: {
  enterIndex: number;
  onOpenDirection: (slug: DirectionSlug) => void;
  slot: DirectionSlot;
}) {
  const label = DIRECTION_LABEL[slot.slug];
  const head = slot.head;
  const lineCount =
    head === null ? 1 : head.lines.length > 0 ? head.lines.length : 1;

  return (
    <OverviewCardShell
      enterIndex={enterIndex}
      testId={`org-direction-card-${slot.slug}`}
      wide={slot.slug === "situation"}
    >
      <div>
        <div className="flex items-start gap-2.5">
          <DirectionMark
            kind={slot.slug}
            style={{ animationDelay: `${overviewMarkDelayMs(enterIndex)}ms` }}
          />
          <h3 className="text-base font-semibold leading-snug">
            {label.title}
            <span className="font-normal text-muted-foreground">
              {" "}
              — {label.question}
            </span>
          </h3>
        </div>
        <span
          aria-hidden="true"
          className="org-dir-rule mt-3 block h-px w-10 bg-foreground/45"
          style={{ animationDelay: `${overviewRuleDelayMs(enterIndex)}ms` }}
        />
        {head === null ? (
          <p
            className="org-dir-line mt-4 text-base leading-relaxed text-muted-foreground"
            data-testid={`org-direction-empty-${slot.slug}`}
            style={{
              animationDelay: `${overviewLineDelayMs(enterIndex, 0)}ms`,
            }}
          >
            {NOT_SET_YET}
          </p>
        ) : head.lines.length > 0 ? (
          <ol className="mt-4 list-decimal space-y-2 pl-5">
            {head.lines.map((line, lineIndex) => (
              <li
                className="org-dir-line text-base leading-relaxed"
                key={line.id}
                style={{
                  animationDelay: `${overviewLineDelayMs(enterIndex, lineIndex)}ms`,
                }}
              >
                {line.text}
                {slot.slug === "objectives" && line.doneWhen ? (
                  <p
                    className="text-sm text-muted-foreground"
                    data-testid={`org-direction-done-when-${line.id}`}
                  >
                    Done when: {line.doneWhen}
                  </p>
                ) : null}
                {slot.slug === "strategy" && line.lineType ? (
                  <p
                    className="text-sm capitalize text-muted-foreground"
                    data-testid={`org-direction-type-${line.id}`}
                  >
                    {line.lineType}
                  </p>
                ) : null}
              </li>
            ))}
          </ol>
        ) : (
          <p
            className="org-dir-line mt-4 text-base leading-relaxed"
            style={{
              animationDelay: `${overviewLineDelayMs(enterIndex, 0)}ms`,
            }}
          >
            {head.body}
          </p>
        )}
      </div>
      <div
        className="org-dir-stamp mt-auto pt-5"
        style={{
          animationDelay: `${overviewStampDelayMs(enterIndex, lineCount)}ms`,
        }}
      >
        <Button
          data-testid={`org-direction-open-${slot.slug}`}
          onClick={() => onOpenDirection(slot.slug)}
          type="button"
          variant="outline"
        >
          Read more
        </Button>
      </div>
    </OverviewCardShell>
  );
}
