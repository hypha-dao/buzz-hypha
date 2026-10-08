import { OrgCardMotion, OrgEventCard } from "@/features/org/cards";
import type { MyWorkColumn, OrgCardModel } from "@/features/org/cards/types";

import { motionDelay } from "./OrgMotionFrame";
import { ORG_EMPTY_NOTHING_NEEDS_YOU } from "./OrgEmptyState";
import {
  overviewLineDelayMs,
  overviewRuleDelayMs,
} from "./overview/overviewMotion";

const COLUMNS: { id: MyWorkColumn; title: string; testId: string }[] = [
  {
    id: "needs_answer",
    title: "Needs your answer",
    testId: "org-my-work-column-needs-answer",
  },
  {
    id: "you_hold",
    title: "You hold",
    testId: "org-my-work-column-you-hold",
  },
  {
    id: "you_offered",
    title: "You offered",
    testId: "org-my-work-column-you-offered",
  },
  {
    id: "finished",
    title: "Finished",
    testId: "org-my-work-column-finished",
  },
];

type MyWorkBoardProps = {
  columns: Record<MyWorkColumn, OrgCardModel[]>;
};

export function MyWorkBoard({ columns }: MyWorkBoardProps) {
  const total = COLUMNS.reduce(
    (count, column) => count + columns[column.id].length,
    0,
  );

  if (total === 0) {
    return (
      <p
        className="flex flex-1 items-center justify-center px-6 py-16 text-center text-sm text-muted-foreground"
        data-testid="org-my-work-empty"
      >
        {ORG_EMPTY_NOTHING_NEEDS_YOU}
      </p>
    );
  }

  return (
    <div
      className="grid min-h-0 flex-1 grid-cols-1 items-start gap-6 overflow-auto p-4 md:grid-cols-2 xl:grid-cols-4"
      data-testid="org-my-work-board"
    >
      {COLUMNS.map((column) => {
        const cards = columns[column.id];
        return (
          <section
            aria-label={column.title}
            className="min-w-0"
            data-testid={column.testId}
            key={column.id}
          >
            <h2
              className="org-dir-line text-2xs font-medium uppercase tracking-wider text-muted-foreground"
              style={motionDelay(overviewLineDelayMs(0, 0))}
            >
              {column.title}
            </h2>
            <span
              aria-hidden="true"
              className="org-dir-rule mb-3 mt-2 block h-px w-10 bg-foreground/45"
              style={motionDelay(overviewRuleDelayMs(0))}
            />
            {cards.length === 0 ? (
              <p className="text-sm text-muted-foreground">Nothing here.</p>
            ) : column.id === "you_hold" ? (
              <HeldColumn cards={cards} />
            ) : (
              <CardList cards={cards} />
            )}
          </section>
        );
      })}
    </div>
  );
}

function HeldColumn({ cards }: { cards: OrgCardModel[] }) {
  const projects = cards.filter((card) => card.itemKind === "project");
  const tickets = cards.filter((card) => card.itemKind !== "project");
  return (
    <div className="flex flex-col gap-5">
      {projects.length > 0 ? (
        <div data-testid="org-my-work-held-projects">
          <h3 className="mb-2 text-2xs font-medium uppercase tracking-wider text-muted-foreground">
            Projects
          </h3>
          <CardList cards={projects} />
        </div>
      ) : null}
      {tickets.length > 0 ? (
        <div data-testid="org-my-work-held-tickets">
          <h3 className="mb-2 text-2xs font-medium uppercase tracking-wider text-muted-foreground">
            Tickets
          </h3>
          <CardList cards={tickets} startIndex={projects.length} />
        </div>
      ) : null}
    </div>
  );
}

function CardList({
  cards,
  startIndex = 0,
}: {
  cards: OrgCardModel[];
  startIndex?: number;
}) {
  return (
    <div className="flex flex-col gap-3">
      {cards.map((model, index) => (
        <OrgCardMotion enterIndex={startIndex + index} key={model.event.id}>
          <OrgEventCard model={model} />
        </OrgCardMotion>
      ))}
    </div>
  );
}
