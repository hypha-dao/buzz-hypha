import * as React from "react";
import { Link, useNavigate } from "@tanstack/react-router";

import { useLiveDoorEvents } from "@/features/org/hooks";
import { proposalChatLink } from "@/features/org/proposalChat";
import { proposalPage } from "@/features/org/proposalDetail";
import { formatWorkDate } from "@/features/org/work/model";
import { KIND_IO_PROPOSAL } from "@/shared/constants/kinds";
import type { RelayEvent } from "@/shared/api/types";
import { Button } from "@/shared/ui/button";

import { useCardCommand } from "../cards/useCardCommand";
import { ORG_HISTORY_LIMIT } from "../hooks/filters";
import { motionDelay } from "./OrgMotionFrame";
import { overviewStampDelayMs } from "./overview/overviewMotion";
import { HolderName } from "./work/HolderName";
import { ORG_EMPTY_NOT_SET_YET, OrgDoorScreen } from "./OrgDoorScreen";
import { ProposalArticle, ProposalFact } from "./ProposalArticle";

/** A proposal, full page. A passed project continues on the work item page. */
export function ProposalScreen({ proposalId }: { proposalId: string }) {
  const navigate = useNavigate();
  const filters = React.useMemo(
    () => [
      {
        kinds: [KIND_IO_PROPOSAL],
        "#d": [proposalId],
        limit: ORG_HISTORY_LIMIT,
      },
    ],
    [proposalId],
  );
  const { events, isLoading } = useLiveDoorEvents(filters);
  const event = newest(events);
  const page = event ? proposalPage(event) : null;
  const link = event ? proposalChatLink(event) : null;
  const passedItemId = link?.kind === "project" ? link.itemId : null;
  const { commands, error, pending, publish } = useCardCommand();
  const busy = pending !== null;

  React.useEffect(() => {
    if (!passedItemId) return;
    void navigate({
      to: "/org/work/$itemId",
      params: { itemId: passedItemId },
      replace: true,
    });
  }, [navigate, passedItemId]);

  if (!isLoading && !event) {
    return (
      <OrgDoorScreen
        empty={ORG_EMPTY_NOT_SET_YET}
        testId="org-proposal"
        title="Work"
      />
    );
  }

  const title = page?.title || "Proposal";
  const status = page?.status ?? "open";
  const dueAt = page?.dueAt ?? null;
  const canVote = status === "open" && Boolean(event);
  const showWorkFacts = page ? page.directionSlug === null : true;

  return (
    <OrgDoorScreen testId="org-proposal" title={page?.doorTitle ?? "Work"}>
      <div className="min-h-0 flex-1 overflow-y-auto">
        <ProposalArticle
          brief={page?.brief ?? ""}
          briefHeading={page?.directionSlug ? "Proposal" : "Brief"}
          crumb={page?.crumb ?? title}
          eyebrow={page?.eyebrow ?? "Proposal"}
          facts={
            showWorkFacts ? (
              <>
                <ProposalFact enterIndex={2} label="Holds it">
                  {page?.suggestedDri ? (
                    <HolderName pubkey={page.suggestedDri} />
                  ) : (
                    "Not yet"
                  )}
                </ProposalFact>
                <ProposalFact enterIndex={3} label="Review">
                  {dueAt !== null ? (
                    <time dateTime={new Date(dueAt * 1000).toISOString()}>
                      {formatWorkDate(dueAt)}
                    </time>
                  ) : (
                    "Not set"
                  )}
                </ProposalFact>
              </>
            ) : null
          }
          lines={page?.lines ?? []}
          parentTo={page?.parentTo ?? "/org/work"}
          status={statusLabel(status)}
          testId="org-proposal"
          title={title}
        >
          {page?.directionSlug && status === "passed" ? (
            <p>
              <Link
                className="text-sm font-medium text-foreground underline"
                params={{ slug: page.directionSlug }}
                to="/org/direction/$slug"
              >
                Open {page.crumb.toLowerCase()}
              </Link>
            </p>
          ) : null}

          {canVote ? (
            <div
              className="org-dir-stamp flex flex-wrap gap-2"
              style={motionDelay(overviewStampDelayMs(0, 2))}
            >
              <Button
                data-testid="org-proposal-agree"
                disabled={busy}
                onClick={() =>
                  void publish(
                    "Agree",
                    commands.buildIoVote({
                      proposal: proposalId,
                      vote: "agree",
                    }),
                  )
                }
                type="button"
              >
                Agree
              </Button>
              <Button
                data-testid="org-proposal-decline"
                disabled={busy}
                onClick={() =>
                  void publish(
                    "Decline",
                    commands.buildIoVote({
                      proposal: proposalId,
                      vote: "decline",
                    }),
                  )
                }
                type="button"
                variant="secondary"
              >
                Decline
              </Button>
            </div>
          ) : null}
          {error ? (
            <p className="text-sm text-destructive" role="alert">
              {error}
            </p>
          ) : null}
        </ProposalArticle>
      </div>
    </OrgDoorScreen>
  );
}

function newest(events: readonly RelayEvent[]): RelayEvent | null {
  let best: RelayEvent | null = null;
  for (const event of events) {
    if (!best || event.created_at >= best.created_at) best = event;
  }
  return best;
}

function statusLabel(status: string): string {
  if (status === "open") return "needs a vote";
  if (status === "passed") return "passed";
  return status;
}
