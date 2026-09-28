/**
 * Local org-onboarding stage for the Personal Assistant DM guide.
 * Stages are localStorage-scoped per community+pubkey so a finished flow
 * does not reappear after reboot.
 */

export type OrgOnboardingStage =
  | "welcome"
  | "shaper-choice"
  | "alone"
  | "others"
  | "done";

export type OrgOnboardingChoice = "alone" | "others";

const STORAGE_PREFIX = "buzz-org-onboarding.v1";

export function orgOnboardingStorageKey(
  pubkey: string,
  communityScope: string,
) {
  return `${STORAGE_PREFIX}:${encodeURIComponent(communityScope)}:${pubkey}`;
}

export function readOrgOnboardingStage(
  pubkey: string | null | undefined,
  communityScope: string | null | undefined,
): OrgOnboardingStage | null {
  if (typeof window === "undefined" || !pubkey || !communityScope) return null;
  try {
    const raw = window.localStorage.getItem(
      orgOnboardingStorageKey(pubkey, communityScope),
    );
    if (
      raw === "welcome" ||
      raw === "shaper-choice" ||
      raw === "alone" ||
      raw === "others" ||
      raw === "done"
    ) {
      return raw;
    }
    return null;
  } catch {
    return null;
  }
}

export function writeOrgOnboardingStage(
  pubkey: string | null | undefined,
  communityScope: string | null | undefined,
  stage: OrgOnboardingStage,
) {
  if (typeof window === "undefined" || !pubkey || !communityScope) return;
  try {
    window.localStorage.setItem(
      orgOnboardingStorageKey(pubkey, communityScope),
      stage,
    );
  } catch {
    // Best-effort; the DM conversation is the durable record.
  }
}

export const ORG_ONBOARDING_COPY = {
  welcomeTitle: "Welcome to your organization",
  welcomeBody:
    "Congratulations on starting a new journey. When you are ready, we can make this community an intelligent organization — AI drafts, you decide; work is offered, never assigned.",
  shaperQuestion: "Will you shape it alone, or with others?",
  shaperExplain:
    "Shapers are like a board. They set direction — mission, vision, objectives, strategy — and approve big projects. Exactly one Shaper set starts with you; you can grow it later.",
  aloneLabel: "Just me for now",
  othersLabel: "I'll invite others",
  aloneNext:
    "You are the first Shaper. Bootstrap creates the private #shapers room on the relay (real kind 50001 — not a fake client channel). Then we define direction together in this chat.",
  othersNext:
    "Bootstrap first so #shapers exists on the relay. Then mint an invite and propose each person as a Shaper once they join. Direction stays with the Shapers together.",
  bootstrapCta: "Bootstrap as first Shaper",
  bootstrapDone: "Bootstrapped — #shapers is live on the relay.",
  inviteCta: "Copy invite link",
  directionHint:
    "Tell me about the purpose of this community in a sentence or two, and I will draft a mission. Or open Overview → Direction when you prefer the form.",
  overviewCta: "Open Overview",
  doneLabel: "Got it — continue in chat",
} as const;
