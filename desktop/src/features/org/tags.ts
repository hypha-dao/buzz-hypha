/**
 * Protocol §4 / §6.5 tag names and markers. Single letters only (V2 / D11).
 * Do not invent tags — this is the C-1 / Protocol set the builders use.
 */

export const TAG_ITEM = "i";
export const TAG_PARENT = "u";
export const TAG_STATUS = "s";
export const TAG_NEEDS = "n";
export const TAG_TYPE = "t";
export const TAG_OP = "op";
export const TAG_VOTE = "vote";
export const TAG_BASE = "base";
export const TAG_DUE = "due";
export const TAG_OUTCOME = "outcome";
export const TAG_REASON = "reason";
export const TAG_WEEK = "week";
export const TAG_BAND = "band";
export const TAG_TX = "tx";

export const MARKER_DRAFT = "draft";
export const MARKER_RECEIPT = "receipt";

export const NEEDS_SHAPER = "shaper";
export const TYPE_PROJECT = "project";
export const STATUS_OPEN = "open";

export const SHAPERS_D_TAG = "shapers";

/** `["e", <draft>, "", "draft"]` — the command settles that draft (§3.2). */
export function draftTag(draftId: string): string[] {
  return ["e", draftId, "", MARKER_DRAFT];
}

/** `["e", <message-id>, "", "receipt"]` — a done-from-talk receipt (§5.5). */
export function receiptTag(eventId: string): string[] {
  return ["e", eventId, "", MARKER_RECEIPT];
}
