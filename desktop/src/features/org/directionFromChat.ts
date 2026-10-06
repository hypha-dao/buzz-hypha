/**
 * Sole-Shaper direction from the Org. Agent DM.
 *
 * The agent names the artifact with a `direction` tag on its kind 9 reply.
 * The member's client signs `50002` (the agent is not a Shaper). In the DM
 * the opener's agree confirms it. In #shapers it stays open for a vote.
 */

export type ChatDirectionSlug =
  | "mission"
  | "vision"
  | "situation"
  | "objectives"
  | "strategy";

const SLUGS = new Set<ChatDirectionSlug>([
  "mission",
  "vision",
  "situation",
  "objectives",
  "strategy",
]);

export type DirectionChatMessage = {
  id: string;
  createdAt: number;
  pubkey?: string;
  signerPubkey?: string;
  body: string;
  tags?: readonly (readonly string[])[];
  pending?: boolean;
};

export type DirectionChatLine = {
  id?: string;
  text: string;
};

export type DirectionChatHead = {
  slug: ChatDirectionSlug;
  version: number;
  body: string;
  lines?: readonly DirectionChatLine[];
};

export type DirectionStatement =
  | {
      kind: "propose";
      agentEventId: string;
      slug: ChatDirectionSlug;
      body: string;
      lines?: DirectionChatLine[];
      base: number;
    }
  | { kind: "skip"; agentEventId: string };

export function directionSlugOf(
  tags: readonly (readonly string[])[] | undefined,
): ChatDirectionSlug | null {
  const value = tags?.find((tag) => tag[0] === "direction")?.[1];
  if (!value || !SLUGS.has(value as ChatDirectionSlug)) return null;
  return value as ChatDirectionSlug;
}

/** A real artifact, not "alone", "software", or "yes thats good, set it". */
export function isDirectionBody(body: string): boolean {
  const text = body.trim();
  if (text.length < 12) return false;
  const words = text.split(/\s+/).filter(Boolean);
  if (words.length < 3) return false;
  if (/^(yes|yeah|yep|ok|okay|sure)\b/i.test(text) && words.length <= 8) {
    return false;
  }
  return true;
}

/** The sentence on a `direction` tag, when the agent sent one. */
export function directionBodyOf(
  tags: readonly (readonly string[])[] | undefined,
): string | null {
  const body = tags?.find((tag) => tag[0] === "direction")?.[2]?.trim();
  if (!body || !isDirectionBody(body)) return null;
  return body;
}

/** A quoted sentence inside the agent's reply. */
export function quotedArtifact(text: string): string | null {
  const matches = text.matchAll(/["“]([^"“”]{12,})["”]/g);
  let found: string | null = null;
  for (const match of matches) {
    const body = match[1]?.trim() ?? "";
    if (isDirectionBody(body)) found = body;
  }
  return found;
}

function signerOf(message: DirectionChatMessage): string | undefined {
  return (message.signerPubkey ?? message.pubkey)?.trim().toLowerCase();
}

function sameText(left: string, right: string): boolean {
  return left.trim().replace(/\s+/g, " ") === right.trim().replace(/\s+/g, " ");
}

function lined(slug: ChatDirectionSlug): slug is "objectives" | "strategy" {
  return slug === "objectives" || slug === "strategy";
}

function atomicTexts(text: string): string[] {
  const unique: string[] = [];
  for (const part of text.split(/\n+/)) {
    const next = part.trim();
    if (!isDirectionBody(next)) continue;
    if (unique.some((line) => sameText(line, next))) continue;
    unique.push(next);
  }
  return unique;
}

/** Lines already on the head. A repeated blob counts once per sentence. */
export function headLines(
  head: DirectionChatHead | undefined,
): DirectionChatLine[] {
  if (!head) return [];
  const source =
    head.lines && head.lines.length > 0 ? head.lines : [{ text: head.body }];
  const unique: DirectionChatLine[] = [];
  for (const line of source) {
    const parts = atomicTexts(line.text);
    if (parts.length === 1) {
      const text = parts[0];
      if (!text || unique.some((item) => sameText(item.text, text))) continue;
      unique.push({ id: line.id, text });
      continue;
    }
    for (const text of parts) {
      if (unique.some((item) => sameText(item.text, text))) continue;
      unique.push({ text });
    }
  }
  return unique;
}

/** Chat order, keeping ids already stored for the same text. */
export function mergeLines(
  existing: readonly DirectionChatLine[],
  texts: readonly string[],
): DirectionChatLine[] {
  const atoms = texts.flatMap((text) => atomicTexts(text));
  const unused = [...existing];
  const merged: DirectionChatLine[] = [];
  for (const text of atoms) {
    const index = unused.findIndex((line) => sameText(line.text, text));
    if (index >= 0) {
      const found = unused[index];
      if (found) merged.push(found);
      unused.splice(index, 1);
    } else if (!merged.some((line) => sameText(line.text, text))) {
      merged.push({ text });
    }
  }
  for (const left of unused) {
    const blob = atoms.some(
      (atom) => !sameText(left.text, atom) && left.text.includes(atom),
    );
    if (blob || merged.some((line) => sameText(line.text, left.text))) {
      continue;
    }
    merged.push(left);
  }
  return merged;
}

/** A new objective or strategy point, kept with the ones already set. */
export function appendLine(
  existing: readonly DirectionChatLine[],
  text: string,
): DirectionChatLine[] | null {
  const next = text.trim();
  if (!isDirectionBody(next)) return null;
  if (existing.some((line) => sameText(line.text, next))) return null;
  return [...existing, { text: next }];
}

function singleSentence(text: string | null): string | null {
  if (!text || text.includes("\n")) return null;
  const trimmed = text.trim();
  return isDirectionBody(trimmed) ? trimmed : null;
}

function sameLines(
  left: readonly DirectionChatLine[],
  right: readonly DirectionChatLine[],
): boolean {
  return (
    left.length === right.length &&
    left.every((line, index) => sameText(line.text, right[index]?.text ?? ""))
  );
}

function announcedLine(
  text: string,
  slug: "objectives" | "strategy",
): string | null {
  const quote = quotedArtifact(text);
  if (!quote) return null;
  const pattern =
    slug === "objectives"
      ? /\bobjective(?:\s+(?:one|two|three|four|five|\d+))?\s+set\b/i
      : /\bstrategy(?:\s+(?:one|two|three|four|five|\d+))?\s+set\b/i;
  return pattern.test(text) ? quote : null;
}

function chatLines(
  messages: readonly DirectionChatMessage[],
  agent: string,
  slug: "objectives" | "strategy",
): string[] {
  const lines: string[] = [];
  for (const turn of messages) {
    if (signerOf(turn) !== agent) continue;
    const tagged =
      directionSlugOf(turn.tags) === slug
        ? singleSentence(directionBodyOf(turn.tags))
        : null;
    const text = announcedLine(turn.body, slug) ?? tagged;
    if (!text || lines.some((line) => sameText(line, text))) continue;
    lines.push(text);
  }
  return lines;
}

function withLines(
  messageId: string,
  slug: "objectives" | "strategy",
  head: DirectionChatHead | undefined,
  lines: DirectionChatLine[],
): DirectionStatement {
  return {
    kind: "propose",
    agentEventId: messageId,
    slug,
    body: lines.map((line) => line.text).join("\n"),
    lines,
    base: head?.version ?? 0,
  };
}

/**
 * The earliest unhandled direction tag in this DM.
 * `wait` while Shapers have not loaded. `null` when there is nothing to do.
 */
export function nextDirectionStatement(input: {
  messages: readonly DirectionChatMessage[];
  currentPubkey: string;
  orgAgentPubkey: string;
  shaperPubkeys: readonly string[] | null;
  heads: readonly DirectionChatHead[];
  handledIds: ReadonlySet<string>;
  /** A shared room attributes a tag with no `from` to the previous human line. */
  room?: "dm" | "shapers" | "channel";
}): DirectionStatement | "wait" | null {
  const me = input.currentPubkey.trim().toLowerCase();
  const agent = input.orgAgentPubkey.trim().toLowerCase();
  const ordered = [...input.messages].sort(
    (left, right) => left.createdAt - right.createdAt,
  );
  for (let index = 0; index < ordered.length; index += 1) {
    const message = ordered[index];
    if (!message || message.pending || input.handledIds.has(message.id)) {
      continue;
    }
    if (signerOf(message) !== agent) continue;
    const from = message.tags
      ?.find((tag) => tag[0] === "from")?.[1]
      ?.trim()
      .toLowerCase();
    if (from && from !== me) {
      return { kind: "skip", agentEventId: message.id };
    }
    if (!from && input.room && input.room !== "dm") {
      const previousHuman = [...ordered.slice(0, index)]
        .reverse()
        .find((turn) => signerOf(turn) && signerOf(turn) !== agent);
      if (!previousHuman || signerOf(previousHuman) !== me) {
        return { kind: "skip", agentEventId: message.id };
      }
    }
    const slug = directionSlugOf(message.tags);
    if (!slug) continue;
    if (input.shaperPubkeys === null) return "wait";
    const shapers = new Set(
      input.shaperPubkeys.map((pubkey) => pubkey.trim().toLowerCase()),
    );
    if (!shapers.has(me)) return { kind: "skip", agentEventId: message.id };
    const prior = [...ordered.slice(0, index)]
      .reverse()
      .find((turn) => signerOf(turn) === me);
    const priorBody =
      prior && isDirectionBody(prior.body) ? prior.body.trim() : "";
    const quoted = message.body.toLowerCase().includes(slug)
      ? quotedArtifact(message.body)
      : null;
    const body = directionBodyOf(message.tags) ?? quoted ?? priorBody;
    if (!isDirectionBody(body)) {
      return { kind: "skip", agentEventId: message.id };
    }
    const head = input.heads.find((slot) => slot.slug === slug);
    if (lined(slug)) {
      const existing = headLines(head);
      const fresh = singleSentence(body);
      if (!fresh || existing.some((line) => line.text.includes(fresh))) {
        return { kind: "skip", agentEventId: message.id };
      }
      const lines = appendLine(existing, fresh);
      if (!lines) return { kind: "skip", agentEventId: message.id };
      return withLines(message.id, slug, head, lines);
    }
    if (head && sameText(head.body, body)) {
      return { kind: "skip", agentEventId: message.id };
    }
    return {
      kind: "propose",
      agentEventId: message.id,
      slug,
      body,
      base: head?.version ?? 0,
    };
  }
  if (input.shaperPubkeys === null) return null;
  const shapers = new Set(
    input.shaperPubkeys.map((pubkey) => pubkey.trim().toLowerCase()),
  );
  if (!shapers.has(me)) return null;
  for (const slug of ["objectives", "strategy"] as const) {
    const head = input.heads.find((slot) => slot.slug === slug);
    const recorded = headLines(head);
    const cleaned = mergeLines(recorded, chatLines(ordered, agent, slug));
    if (cleaned.length === 0 || sameLines(recorded, cleaned)) continue;
    const fixId = `fix:${slug}:${head?.version ?? 0}:${cleaned.length}`;
    if (input.handledIds.has(fixId)) continue;
    return withLines(fixId, slug, head, cleaned);
  }
  for (const head of input.heads) {
    if (isDirectionBody(head.body)) continue;
    const fixId = `fix:${head.slug}:${head.version}`;
    if (input.handledIds.has(fixId)) continue;
    const source = [...ordered].reverse().find((turn) => {
      if (signerOf(turn) !== agent) return false;
      return turn.body.toLowerCase().includes(head.slug);
    });
    const body = source ? quotedArtifact(source.body) : null;
    if (!body || sameText(body, head.body)) continue;
    return {
      kind: "propose",
      agentEventId: fixId,
      slug: head.slug,
      body,
      base: head.version,
    };
  }
  return null;
}
