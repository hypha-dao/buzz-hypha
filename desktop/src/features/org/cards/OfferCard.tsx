import * as React from "react";

import { KIND_IO_DRAFT } from "@/shared/constants/kinds";

import { suggestedPubkey } from "./parse";
import { ActionButton, DeclineChips } from "./CardActions";
import { OrgCardShell } from "./OrgCardShell";
import type { DeclineReason, OrgCardModel } from "./types";
import { useCardCommand } from "./useCardCommand";

export function OfferCard({ model }: { model: OrgCardModel }) {
  const { commands, error, pending, publish } = useCardCommand();
  const [reason, setReason] = React.useState<DeclineReason | null>(null);
  const busy = pending !== null;
  const item = model.itemId;
  const isDraft = model.event.kind === KIND_IO_DRAFT;
  const suggested = suggestedPubkey(model.event);
  const actionable = model.column === "needs_answer";
  const showOffer = actionable && isDraft && Boolean(item && suggested);
  const showAccept = actionable && Boolean(item);
  const showDraftDecline = actionable && isDraft;
  const showNotNow = actionable && !isDraft && Boolean(item);

  return (
    <OrgCardShell
      model={model}
      actions={
        <>
          {showOffer && item && suggested ? (
            <ActionButton
              disabled={busy}
              label="Offer"
              onClick={() =>
                void publish(
                  "Offer",
                  commands.buildIoOffer({
                    item,
                    pubkey: suggested,
                    draftId: model.event.id,
                  }),
                )
              }
              testId="org-card-offer"
            />
          ) : null}
          {showAccept && item ? (
            <ActionButton
              disabled={busy}
              label="Accept"
              onClick={() =>
                void publish("Accept", commands.buildIoAccept(item))
              }
              testId="org-card-accept"
            />
          ) : null}
          {showDraftDecline ? (
            <>
              <DeclineChips
                disabled={busy}
                onSelect={setReason}
                selected={reason}
              />
              <ActionButton
                disabled={busy || reason === null}
                label="Decline"
                onClick={() =>
                  void publish(
                    "Decline",
                    commands.buildIoDraftDecide({
                      draftId: model.event.id,
                      outcome: "decline",
                      reason: reason ?? undefined,
                    }),
                  )
                }
                testId="org-card-decline"
                variant="secondary"
              />
            </>
          ) : null}
          {showNotNow && item ? (
            <ActionButton
              disabled={busy}
              label="Not now"
              onClick={() =>
                void publish("Not now", commands.buildIoDecline(item))
              }
              testId="org-card-not-now"
              variant="outline"
            />
          ) : null}
          {error ? (
            <p className="w-full text-xs text-destructive">{error}</p>
          ) : null}
        </>
      }
    />
  );
}
