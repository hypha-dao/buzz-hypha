import * as React from "react";
import { toast } from "sonner";

import { useAppNavigation } from "@/app/navigation/useAppNavigation";
import { useIdentityQuery } from "@/shared/api/hooks";
import { mintInvite } from "@/shared/api/invites";
import { useCommunities } from "@/features/communities/useCommunities";
import {
  ORG_ONBOARDING_COPY,
  readOrgOnboardingStage,
  writeOrgOnboardingStage,
  type OrgOnboardingChoice,
  type OrgOnboardingStage,
} from "@/features/org/orgOnboarding";
import { useOrgCommands } from "@/features/org/useOrgCommands";
import { Button } from "@/shared/ui/button";
import { normalizePubkey } from "@/shared/lib/pubkey";

type OrgOnboardingGuideProps = {
  /** When false, render nothing (not the PA DM). */
  active: boolean;
};

/**
 * Guided first-run panel in the Personal Assistant DM: welcome → sole Shaper
 * vs others → real `50001` bootstrap → invite / direction next steps.
 */
export function OrgOnboardingGuide({ active }: OrgOnboardingGuideProps) {
  const identityQuery = useIdentityQuery();
  const { activeCommunity } = useCommunities();
  const commands = useOrgCommands();
  const { goOrg } = useAppNavigation();
  const pubkey = identityQuery.data?.pubkey
    ? normalizePubkey(identityQuery.data.pubkey)
    : null;
  const communityScope = activeCommunity?.relayUrl ?? null;

  const [stage, setStage] = React.useState<OrgOnboardingStage>(
    () => readOrgOnboardingStage(pubkey, communityScope) ?? "welcome",
  );
  const [busy, setBusy] = React.useState(false);
  const [bootstrapDone, setBootstrapDone] = React.useState(false);
  const [inviteUrl, setInviteUrl] = React.useState<string | null>(null);

  React.useEffect(() => {
    const stored = readOrgOnboardingStage(pubkey, communityScope);
    if (stored) setStage(stored);
  }, [pubkey, communityScope]);

  const advance = React.useCallback(
    (next: OrgOnboardingStage) => {
      setStage(next);
      writeOrgOnboardingStage(pubkey, communityScope, next);
    },
    [communityScope, pubkey],
  );

  const choose = React.useCallback(
    (choice: OrgOnboardingChoice) => {
      advance(choice);
    },
    [advance],
  );

  const runBootstrap = React.useCallback(async () => {
    if (!pubkey || busy) return;
    setBusy(true);
    try {
      await commands.publish(
        commands.buildIoShapersPropose({ op: "add", pubkey }, true),
      );
      setBootstrapDone(true);
      toast.success("Bootstrapped — #shapers created on the relay");
    } catch (error) {
      toast.error(
        error instanceof Error
          ? error.message
          : "Bootstrap failed — is the relay up and the owner key signing?",
      );
    } finally {
      setBusy(false);
    }
  }, [busy, commands, pubkey]);

  const copyInvite = React.useCallback(async () => {
    setBusy(true);
    try {
      const invite = await mintInvite({
        ttlSecs: 60 * 60 * 24 * 7,
        maxUses: 20,
      });
      const url = invite.url ?? invite.code;
      setInviteUrl(url);
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(url);
      }
      toast.success("Invite link copied");
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Could not mint an invite",
      );
    } finally {
      setBusy(false);
    }
  }, []);

  if (!active || stage === "done") return null;

  return (
    <section
      aria-label="Organization onboarding"
      className="pointer-events-auto mx-3 mb-2 rounded-xl border border-border/70 bg-card/95 px-4 py-3 shadow-sm"
      data-testid="org-onboarding-guide"
    >
      {stage === "welcome" || stage === "shaper-choice" ? (
        <div className="flex flex-col gap-3">
          <div>
            <h2 className="text-sm font-medium text-foreground">
              {ORG_ONBOARDING_COPY.welcomeTitle}
            </h2>
            <p className="mt-1 text-sm text-muted-foreground">
              {ORG_ONBOARDING_COPY.welcomeBody}
            </p>
          </div>
          <div>
            <p className="text-sm font-medium text-foreground">
              {ORG_ONBOARDING_COPY.shaperQuestion}
            </p>
            <p className="mt-1 text-sm text-muted-foreground">
              {ORG_ONBOARDING_COPY.shaperExplain}
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            <Button
              data-testid="org-onboarding-alone"
              disabled={busy}
              onClick={() => choose("alone")}
              size="sm"
              type="button"
            >
              {ORG_ONBOARDING_COPY.aloneLabel}
            </Button>
            <Button
              data-testid="org-onboarding-others"
              disabled={busy}
              onClick={() => choose("others")}
              size="sm"
              type="button"
              variant="outline"
            >
              {ORG_ONBOARDING_COPY.othersLabel}
            </Button>
          </div>
        </div>
      ) : null}

      {stage === "alone" ? (
        <div className="flex flex-col gap-3">
          <p className="text-sm text-muted-foreground">
            {ORG_ONBOARDING_COPY.aloneNext}
          </p>
          {!bootstrapDone ? (
            <Button
              data-testid="org-onboarding-bootstrap"
              disabled={busy}
              onClick={() => {
                void runBootstrap();
              }}
              size="sm"
              type="button"
            >
              {ORG_ONBOARDING_COPY.bootstrapCta}
            </Button>
          ) : (
            <p className="text-sm text-foreground">
              {ORG_ONBOARDING_COPY.bootstrapDone}
            </p>
          )}
          <p className="text-sm text-muted-foreground">
            {ORG_ONBOARDING_COPY.directionHint}
          </p>
          <div className="flex flex-wrap gap-2">
            <Button
              data-testid="org-onboarding-overview"
              onClick={() => {
                void goOrg();
              }}
              size="sm"
              type="button"
              variant="outline"
            >
              {ORG_ONBOARDING_COPY.overviewCta}
            </Button>
            <Button
              data-testid="org-onboarding-dismiss"
              onClick={() => advance("done")}
              size="sm"
              type="button"
              variant="ghost"
            >
              {ORG_ONBOARDING_COPY.doneLabel}
            </Button>
          </div>
        </div>
      ) : null}

      {stage === "others" ? (
        <div className="flex flex-col gap-3">
          <p className="text-sm text-muted-foreground">
            {ORG_ONBOARDING_COPY.othersNext}
          </p>
          {!bootstrapDone ? (
            <Button
              data-testid="org-onboarding-bootstrap"
              disabled={busy}
              onClick={() => {
                void runBootstrap();
              }}
              size="sm"
              type="button"
            >
              {ORG_ONBOARDING_COPY.bootstrapCta}
            </Button>
          ) : (
            <p className="text-sm text-foreground">
              {ORG_ONBOARDING_COPY.bootstrapDone}
            </p>
          )}
          <div className="flex flex-wrap gap-2">
            <Button
              data-testid="org-onboarding-invite"
              disabled={busy}
              onClick={() => {
                void copyInvite();
              }}
              size="sm"
              type="button"
            >
              {ORG_ONBOARDING_COPY.inviteCta}
            </Button>
            <Button
              data-testid="org-onboarding-overview"
              onClick={() => {
                void goOrg();
              }}
              size="sm"
              type="button"
              variant="outline"
            >
              {ORG_ONBOARDING_COPY.overviewCta}
            </Button>
            <Button
              data-testid="org-onboarding-dismiss"
              onClick={() => advance("done")}
              size="sm"
              type="button"
              variant="ghost"
            >
              {ORG_ONBOARDING_COPY.doneLabel}
            </Button>
          </div>
          {inviteUrl ? (
            <p
              className="truncate text-2xs text-muted-foreground"
              data-testid="org-onboarding-invite-url"
            >
              {inviteUrl}
            </p>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}
