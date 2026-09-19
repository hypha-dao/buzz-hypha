import { Card } from "@/shared/ui/card";

import type { TallyNote } from "./parseOverview";

type TallyCardProps = {
  tally: TallyNote | null;
};

const MOVE_LABEL: Record<string, string> = {
  "1": "Direction → projects",
  "2": "Project → tickets",
  "3": "Completion → next",
  "4": "Health",
};

function countMap(counts: Record<string, number>): string {
  const parts = Object.entries(counts)
    .filter(([, value]) => value > 0)
    .map(([key, value]) => `${key.replaceAll("_", " ")} ${value}`);
  return parts.length > 0 ? parts.join(", ") : "none";
}

export function TallyCard({ tally }: TallyCardProps) {
  return (
    <Card className="p-4" data-testid="org-tally-card">
      <h2 className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
        Tally
      </h2>
      {tally ? (
        <div className="mt-2 space-y-2 text-sm">
          <p data-testid="org-tally-week">
            Week {tally.week} · last {tally.windowWeeks} weeks
          </p>
          {Object.entries(tally.moves).map(([move, row]) => (
            <div data-testid={`org-tally-move-${move}`} key={move}>
              <p className="font-medium">
                {MOVE_LABEL[move] ?? `Move ${move}`}
              </p>
              <p className="text-muted-foreground">
                opened {row.opened}, agreed {row.accepted}, amended{" "}
                {row.amended}, declined {countMap(row.declined)}, dropped{" "}
                {countMap(row.dropped)}
              </p>
            </div>
          ))}
          {tally.health ? (
            <p data-testid="org-tally-health">
              Health: {tally.health.agreed} of {tally.health.rated} agreed ·{" "}
              {tally.health.reads} reads
            </p>
          ) : null}
          <p className="text-muted-foreground">
            Open drafts older than 5 days: {tally.openOlderThan5d}. Receipt
            refusals: {tally.receiptRejected}.
          </p>
        </div>
      ) : (
        <p className="mt-2 text-sm text-muted-foreground">Not set yet.</p>
      )}
    </Card>
  );
}
