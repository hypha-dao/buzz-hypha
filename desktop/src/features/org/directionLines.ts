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

/** `Type: bet` at the end of a drafted line, including a trailing period. */
const STRATEGY_TYPE_AT_END = /(?:^|\s)type:\s*(bet|rule|refusal)\b[.!,]?\s*$/i;

/**
 * The strategy prose, and the type when the draft wrote one.
 * The type is metadata. It is not part of the line a person edits.
 */
export function readStrategyLine(row: string): {
  text: string;
  type: StrategyLineType | null;
} {
  const match = row.match(STRATEGY_TYPE_AT_END);
  if (!match?.[1] || match.index === undefined) {
    return { text: row.trim(), type: null };
  }
  const text = row
    .slice(0, match.index)
    .trim()
    .replace(/[.\s]+$/, "");
  return { text, type: match[1].toLowerCase() as StrategyLineType };
}

/** Strategy prose for a box. A type marker is left out. */
export function strategyDisplayText(row: string): string {
  const read = readStrategyLine(row);
  return read.type ? read.text : row.trim();
}

function sameStrategyText(left: string, right: string): boolean {
  const norm = (value: string) =>
    value
      .trim()
      .replace(/[.\s]+$/, "")
      .toLowerCase();
  return norm(left) === norm(right);
}

/**
 * One box per strategy. Confirmed lines keep their type. A drafted
 * `Type:` marker is stored beside the line and removed from the text.
 */
export function presentStrategyDraft(
  chunks: readonly string[],
  existing: readonly { text: string; lineType?: StrategyLineType | null }[],
): { text: string; type: StrategyLineType }[] {
  const lines: { text: string; type: StrategyLineType; fromMark: boolean }[] =
    [];
  const push = (text: string, type: StrategyLineType, fromMark: boolean) => {
    const clean = text.trim();
    if (clean.length < 3) return;
    const found = lines.find((line) => sameStrategyText(line.text, clean));
    if (found) {
      if (fromMark) {
        found.type = type;
        found.fromMark = true;
      }
      return;
    }
    lines.push({ text: clean, type, fromMark });
  };
  for (const line of existing) {
    const read = readStrategyLine(line.text);
    push(
      read.type ? read.text : line.text.trim(),
      line.lineType ?? read.type ?? "bet",
      Boolean(line.lineType) || Boolean(read.type),
    );
  }
  for (const chunk of chunks) {
    const read = readStrategyLine(chunk);
    if (read.type && read.text.length < 3) {
      const prev = lines[lines.length - 1];
      if (prev) {
        prev.type = read.type;
        prev.fromMark = true;
      }
      continue;
    }
    const text = read.type ? read.text : chunk.trim();
    if (text.length < 12) continue;
    push(text, read.type ?? "bet", Boolean(read.type));
  }
  return lines.map(({ text, type }) => ({ text, type }));
}

/** Publish lines from the boxes. Types stay the ones kept beside them. */
export function strategyLinesForPublish(
  body: string,
  types: readonly StrategyLineType[],
): { lines: DirectionCommandLine[] } | { error: string } {
  const rows = body.length > 0 ? body.split("\n") : [];
  const lines: DirectionCommandLine[] = [];
  for (let index = 0; index < rows.length; index += 1) {
    const row = rows[index] ?? "";
    const read = readStrategyLine(row);
    const text = read.type ? read.text : row.trim();
    if (text.length < 12) continue;
    lines.push({ text, type: read.type ?? types[index] ?? "bet" });
  }
  if (lines.length === 0) return { error: "Add at least one strategy." };
  return { lines };
}

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
            : "Each strategy needs a line of its own.",
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
  const read = readStrategyLine(row);
  const text = read.type ? read.text : row.trim();
  if (text.length < 3) return null;
  return { text, type: read.type ?? "bet" };
}
