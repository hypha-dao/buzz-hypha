/**
 * Entrance timing for Overview cards, matching the org-preview direction
 * choreography: cards settle in a stagger, a rule draws under the title,
 * each line writes in, and the footer stamp lands last.
 */

const CARD_STAGGER_MS = 140;
const MARK_AFTER_CARD_MS = 120;
const RULE_AFTER_CARD_MS = 180;
const FIRST_LINE_MS = 320;
const LINE_STAGGER_MS = 170;
const STAMP_AFTER_LAST_LINE_MS = 220;

export function overviewCardDelayMs(index: number): number {
  return index * CARD_STAGGER_MS;
}

export function overviewMarkDelayMs(cardIndex: number): number {
  return overviewCardDelayMs(cardIndex) + MARK_AFTER_CARD_MS;
}

export function overviewRuleDelayMs(cardIndex: number): number {
  return overviewCardDelayMs(cardIndex) + RULE_AFTER_CARD_MS;
}

export function overviewLineDelayMs(
  cardIndex: number,
  lineIndex: number,
): number {
  return (
    overviewCardDelayMs(cardIndex) + FIRST_LINE_MS + lineIndex * LINE_STAGGER_MS
  );
}

export function overviewStampDelayMs(
  cardIndex: number,
  lineCount: number,
): number {
  return overviewLineDelayMs(cardIndex, lineCount) + STAMP_AFTER_LAST_LINE_MS;
}
