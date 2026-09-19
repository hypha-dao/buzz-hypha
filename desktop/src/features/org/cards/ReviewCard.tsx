import * as React from "react";

import { ActionButton, DeclineChips } from "./CardActions";
import { reviewProjectInput } from "./draftPayload";
import { OrgCardShell } from "./OrgCardShell";
import type { DeclineReason, OrgCardModel } from "./types";
import { useCardCommand } from "./useCardCommand";

const WEEK_SECS = 7 * 24 * 60 * 60;

export function ReviewCard({ model }: { model: OrgCardModel }) {
  const { commands, error, pending, publish } = useCardCommand();
  const [reason, setReason] = React.useState<DeclineReason | null>(
    "already_covered",
  );
  const busy = pending !== null;
  const item = model.itemId;

  return (
    <OrgCardShell
      model={model}
      actions={
        <>
          <ActionButton
            disabled={busy}
            label="Open the follow-up"
            onClick={() =>
              void publish(
                "Open the follow-up",
                commands.buildIoProjectPropose(reviewProjectInput(model)),
              )
            }
            testId="org-card-open-follow-up"
          />
          <DeclineChips
            disabled={busy}
            onSelect={setReason}
            selected={reason}
          />
          <ActionButton
            disabled={busy || reason === null}
            label="Nothing more"
            onClick={() =>
              void publish(
                "Nothing more",
                commands.buildIoDraftDecide({
                  draftId: model.event.id,
                  outcome: "decline",
                  reason: reason ?? undefined,
                }),
              )
            }
            testId="org-card-nothing-more"
            variant="secondary"
          />
          {item ? (
            <ActionButton
              disabled={busy}
              label="Keep open until"
              onClick={() =>
                void publish(
                  "Keep open until",
                  commands.buildIoSetDue(
                    item,
                    Math.floor(Date.now() / 1000) + WEEK_SECS,
                  ),
                )
              }
              testId="org-card-keep-open"
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
