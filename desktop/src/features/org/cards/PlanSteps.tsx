import { asString, parseJsonObject } from "./tags";

type PlanStep = {
  piece: string;
  held: boolean;
};

type ChangeView = {
  from: string;
  to: string;
  doneWhen: string[];
  steps: PlanStep[];
};

export function changeFromContent(content: string): ChangeView | null {
  const parsed = parseJsonObject(content);
  if (!parsed) return null;
  const change =
    parsed.change && typeof parsed.change === "object"
      ? (parsed.change as Record<string, unknown>)
      : null;
  const from = change ? (asString(change.from) ?? "") : "";
  const to = change ? (asString(change.to) ?? "") : "";
  const doneWhen =
    change && Array.isArray(change.done_when)
      ? change.done_when.filter(
          (line): line is string => typeof line === "string",
        )
      : [];
  const steps = Array.isArray(parsed.plan)
    ? parsed.plan.flatMap((step) => {
        if (!step || typeof step !== "object") return [];
        const row = step as Record<string, unknown>;
        const piece = asString(row.piece);
        if (!piece) return [];
        return [
          { piece, held: typeof row.held === "string" && row.held.length > 0 },
        ];
      })
    : [];
  if (!from && !to && doneWhen.length === 0 && steps.length === 0) return null;
  return { from, to, doneWhen, steps };
}

/** From, to, done when, and the step list. Held steps are text, not actions. */
export function PlanSteps({ content }: { content: string }) {
  const change = changeFromContent(content);
  if (!change) return null;
  return (
    <div className="mt-3" data-testid="org-plan">
      {change.from ? (
        <p className="text-sm text-muted-foreground">
          <span className="text-foreground">From.</span> {change.from}
        </p>
      ) : null}
      {change.to ? (
        <p className="mt-1 text-sm text-muted-foreground">
          <span className="text-foreground">To.</span> {change.to}
        </p>
      ) : null}
      {change.doneWhen.length > 0 ? (
        <ul className="mt-2 list-disc pl-4 text-sm">
          {change.doneWhen.map((line) => (
            <li data-testid="org-plan-done-when" key={line}>
              {line}
            </li>
          ))}
        </ul>
      ) : null}
      {change.steps.length > 0 ? (
        <ol className="mt-2 list-decimal pl-4 text-sm">
          {change.steps.map((step) => (
            <li data-testid={`org-plan-step-${step.piece}`} key={step.piece}>
              {step.piece}
              {step.held ? (
                <span className="text-muted-foreground"> — held</span>
              ) : null}
            </li>
          ))}
        </ol>
      ) : null}
    </div>
  );
}
