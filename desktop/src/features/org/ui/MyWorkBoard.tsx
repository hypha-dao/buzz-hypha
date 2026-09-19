import { OrgEventCard } from "@/features/org/cards";
import type { MyWorkColumn, OrgCardModel } from "@/features/org/cards/types";

import { ORG_EMPTY_NOTHING_NEEDS_YOU } from "./OrgEmptyState";

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
];

type MyWorkBoardProps = {
  columns: Record<MyWorkColumn, OrgCardModel[]>;
};

export function MyWorkBoard({ columns }: MyWorkBoardProps) {
  const total =
    columns.needs_answer.length +
    columns.you_hold.length +
    columns.you_offered.length;

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
      className="grid min-h-0 flex-1 grid-cols-1 gap-6 overflow-auto p-4 md:grid-cols-3"
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
            <h2 className="mb-3 text-2xs font-medium uppercase tracking-wider text-muted-foreground">
              {column.title}
            </h2>
            <div className="flex flex-col gap-3">
              {cards.length === 0 ? (
                <p className="text-sm text-muted-foreground">Nothing here.</p>
              ) : (
                cards.map((model) => (
                  <OrgEventCard key={model.event.id} model={model} />
                ))
              )}
            </div>
          </section>
        );
      })}
    </div>
  );
}
