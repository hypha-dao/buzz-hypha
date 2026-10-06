/**
 * A chat draft, shaped like the page it becomes once published: the same
 * title, brief, lines, and facts as the proposal page, before any vote.
 */

import {
  directionBodyLines,
  draftTitle,
  RULE_KINDS,
  type ChatDraft,
} from "./chatDraft";

export type DraftFact =
  | { label: string; kind: "person"; pubkey: string | null; empty: string }
  | { label: string; kind: "date"; at: number | null; empty: string };

export type DraftPageModel = {
  eyebrow: string;
  title: string;
  briefHeading: string;
  brief: string;
  lines: { id: string; text: string }[];
  crumb: string;
  facts: DraftFact[];
};

const DIRECTION_NAME: Record<string, string> = {
  mission: "Mission",
  vision: "Vision",
  situation: "Situation",
  objectives: "Objectives",
  strategy: "Strategy",
};

function numbered(texts: readonly string[]): DraftPageModel["lines"] {
  return texts.map((text, index) => ({ id: `line-${index + 1}`, text }));
}

function days(secs: number): string {
  const count = Math.max(1, Math.round(secs / 86_400));
  return count === 1 ? "1 day" : `${count} days`;
}

export function draftPageModel(
  draft: ChatDraft,
  itemTitle?: string | null,
): DraftPageModel {
  switch (draft.kind) {
    case "project":
    case "revise-project": {
      const facts: DraftFact[] = [];
      if (draft.kind === "project") {
        facts.push({
          label: "Holds it",
          kind: "person",
          pubkey: draft.suggestedDri,
          empty: "Not yet",
        });
      }
      facts.push({
        label: "Review",
        kind: "date",
        at: draft.dueAt,
        empty: "Not set",
      });
      return {
        eyebrow:
          draft.kind === "project" ? "Project draft" : "Project revision draft",
        title: draft.title,
        briefHeading: "Brief",
        brief: draft.brief === draft.title ? "" : draft.brief,
        lines: [],
        crumb: draft.title,
        facts,
      };
    }
    case "direction":
    case "revise-direction": {
      const name = DIRECTION_NAME[draft.slug] ?? "Direction";
      const lined = draft.slug === "objectives" || draft.slug === "strategy";
      const lines = lined ? directionBodyLines(draft.slug, draft.body) : [];
      return {
        eyebrow:
          draft.kind === "direction"
            ? `${name} draft`
            : `${name} revision draft`,
        title: lines.length > 0 ? name : draft.body || name,
        briefHeading: "Draft",
        brief: "",
        lines: numbered(lines),
        crumb: name,
        facts: [],
      };
    }
    case "dri": {
      const title = draftTitle(draft, itemTitle);
      return {
        eyebrow: "DRI draft",
        title,
        briefHeading: "Brief",
        brief: "",
        lines: [],
        crumb: title,
        facts: [
          {
            label: "Holds it",
            kind: "person",
            pubkey: draft.pubkey,
            empty: "Not yet",
          },
        ],
      };
    }
    case "remove-project": {
      const title = draftTitle(draft, itemTitle);
      return {
        eyebrow: "Remove project draft",
        title,
        briefHeading: "Brief",
        brief: "",
        lines: [],
        crumb: title,
        facts: [],
      };
    }
    case "shapers-add":
    case "shapers-remove": {
      const title = draftTitle(draft);
      return {
        eyebrow: "Shapers draft",
        title,
        briefHeading: "Why",
        brief: draft.why,
        lines: [],
        crumb: title,
        facts: [
          {
            label: "Member",
            kind: "person",
            pubkey: draft.pubkey,
            empty: "Not named",
          },
        ],
      };
    }
    case "shapers-rules": {
      const title = draftTitle(draft);
      const rows: string[] = [];
      for (const key of RULE_KINDS) {
        const value = draft.rules[key];
        if (value) rows.push(`${key === "dri" ? "DRI" : key} rule: ${value}`);
      }
      if (draft.decisionWindowSecs) {
        rows.push(`Decision window: ${days(draft.decisionWindowSecs)}`);
      }
      if (draft.offerWindowSecs) {
        rows.push(`Offer window: ${days(draft.offerWindowSecs)}`);
      }
      return {
        eyebrow: "Decision rules draft",
        title,
        briefHeading: "Brief",
        brief: "",
        lines: numbered(rows),
        crumb: title,
        facts: [],
      };
    }
    case "shapers-agent": {
      const title = draftTitle(draft);
      return {
        eyebrow: "Org agent draft",
        title,
        briefHeading: "Why",
        brief: draft.why,
        lines: [],
        crumb: title,
        facts: [
          {
            label: "Agent",
            kind: "person",
            pubkey: draft.pubkey,
            empty: "The hosted agent",
          },
        ],
      };
    }
  }
}
