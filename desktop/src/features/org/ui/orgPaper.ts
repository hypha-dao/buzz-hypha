/** Class that remaps the app to the org-preview paper palette. */
export const ORG_PAPER_CLASS = "org-paper";

/**
 * Every door — Inbox, channels, DMs, Agents, Projects, and the org doors —
 * uses the preview's black, white, and gray canvas.
 */
export function applyPaperSurface(root: {
  classList: { add: (name: string) => void };
}): void {
  root.classList.add(ORG_PAPER_CLASS);
}
