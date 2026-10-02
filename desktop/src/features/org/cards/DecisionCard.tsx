import { Link } from "@tanstack/react-router";

import { proposalChatLink } from "@/features/org/proposalChat";
import { proposalDetail } from "@/features/org/proposalDetail";
import { formatWorkDate } from "@/features/org/work/model";

import { ActionButton } from "./CardActions";
import { OrgCardShell } from "./OrgCardShell";
import type { OrgCardModel, OrgEventLike } from "./types";
import { useCardCommand } from "./useCardCommand";

export function ProposalDetailPanel({ event }: { event: OrgEventLike }) {
  const detail = proposalDetail(event);
  if (!detail) return null;
  const link = proposalChatLink(event);
  return (
    <div className="flex flex-col gap-2">
      {detail.body ? (
        <p className="whitespace-pre-wrap text-sm leading-relaxed text-foreground">
          {detail.body}
        </p>
      ) : null}
      {detail.dueAt !== null ? (
        <p className="text-xs text-muted-foreground">
          <time dateTime={new Date(detail.dueAt * 1000).toISOString()}>
            Review {formatWorkDate(detail.dueAt)}
          </time>
        </p>
      ) : null}
      {link?.kind === "direction" ? (
        <Link
          className="text-sm font-medium text-foreground underline"
          params={{ slug: link.slug }}
          to="/org/direction/$slug"
        >
          {link.label}
        </Link>
      ) : null}
      {link?.kind === "project" ? (
        <Link
          className="text-sm font-medium text-foreground underline"
          params={{ itemId: link.itemId }}
          to="/org/work/$itemId"
        >
          {link.label}
        </Link>
      ) : null}
    </div>
  );
}

export function DecisionCard({ model }: { model: OrgCardModel }) {
  const { commands, error, pending, publish } = useCardCommand();
  const proposal = model.proposalId;
  const busy = pending !== null;
  const detail = proposalDetail(model.event);

  return (
    <OrgCardShell
      detail={detail ? <ProposalDetailPanel event={model.event} /> : undefined}
      model={model}
      actions={
        <>
          {proposal ? (
            <>
              <ActionButton
                disabled={busy}
                label="Agree"
                onClick={() =>
                  void publish(
                    "Agree",
                    commands.buildIoVote({ proposal, vote: "agree" }),
                  )
                }
                testId="org-card-agree"
              />
              <ActionButton
                disabled={busy}
                label="Decline"
                onClick={() =>
                  void publish(
                    "Decline",
                    commands.buildIoVote({ proposal, vote: "decline" }),
                  )
                }
                testId="org-card-decline"
                variant="secondary"
              />
            </>
          ) : null}
          {error ? (
            <p className="w-full text-xs text-destructive">{error}</p>
          ) : null}
        </>
      }
    />
  );
}
