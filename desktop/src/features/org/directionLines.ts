/**
 * Objective and strategy lines the member's client puts on a `50002`.
 * The agent drafts prose; this is the shape the relay accepts.
 */

export type StrategyLineType = "bet" | "rule" | "refusal";

export type DirectionCommandLine = {
  text: string;
  date?: number;
  done_when?: string;
  type?: StrategyLineType;
};

export type DirectionLineSlug = "objectives" | "strategy";

const DONE_WHEN_MAX = 200;

/** Lines for a direction proposal, or the reason a vague line cannot be one. */
export function linesForDirectionPropose(
  slug: DirectionLineSlug,
  body: string,
): { lines: DirectionCommandLine[] } | { error: string } {
  const rows = body
    .split(/\n+/)
    .map((line) => line.trim())
    .filter((line) => line.length >= 12);
  if (rows.length === 0) {
    return {
      error:
        slug === "objectives"
          ? "Add at least one objective."
          : "Add at least one strategy.",
    };
  }
  const lines: DirectionCommandLine[] = [];
  for (const row of rows) {
    const parsed =
      slug === "objectives" ? parseObjective(row) : parseStrategy(row);
    if (!parsed) {
      return {
        error:
          slug === "objectives"
            ? "Each objective needs a date and a done when someone could check."
            : "Each strategy line needs Type: bet, rule, or refusal.",
      };
    }
    lines.push(parsed);
  }
  return { lines };
}

function parseObjective(row: string): DirectionCommandLine | null {
  const mark = row.toLowerCase().indexOf("done when:");
  const by = row.toLowerCase().match(/by:\s*(\d{4}-\d{2}-\d{2})/);
  if (mark < 0 || !by?.[1]) return null;
  const text = row
    .slice(0, mark)
    .trim()
    .replace(/[.\s]+$/, "");
  const after = row.slice(mark + "done when:".length).trim();
  const check =
    after
      .split(/\.?\s*by:/i)[0]
      ?.trim()
      .replace(/\.+$/, "") ?? "";
  if (text.length < 3 || check.length < 8 || check.length > DONE_WHEN_MAX) {
    return null;
  }
  const date = Date.parse(`${by[1]}T00:00:00Z`);
  if (Number.isNaN(date)) return null;
  return { text, done_when: check, date: Math.floor(date / 1000) };
}

function parseStrategy(row: string): DirectionCommandLine | null {
  const match = row.match(/^(.*)\btype:\s*(bet|rule|refusal)\s*$/i);
  if (!match?.[1] || !match[2]) return null;
  const text = match[1].trim().replace(/[.\s]+$/, "");
  if (text.length < 3) return null;
  return { text, type: match[2].toLowerCase() as StrategyLineType };
}
