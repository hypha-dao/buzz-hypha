import * as React from "react";

import { ActionButton, DeclineChips } from "./CardActions";
import { OrgCardShell } from "./OrgCardShell";
import type { DeclineReason, OrgCardModel } from "./types";
import { useCardCommand } from "./useCardCommand";

export function DoneCard({ model }: { model: OrgCardModel }) {
  const { commands, error, pending, publish } = useCardCommand();
  const [reason, setReason] = React.useState<DeclineReason | null>("not_now");
  const busy = pending !== null;
  const item = model.itemId;

  return (
    <OrgCardShell
      model={model}
      actions={
        <>
          {item ? (
            <ActionButton
              disabled={busy}
              label="Mark done"
              onClick={() =>
                void publish(
                  "Mark done",
                  commands.buildIoDone({ item, draftId: model.event.id }),
                )
              }
              testId="org-card-mark-done"
            />
          ) : null}
          <DeclineChips
            disabled={busy}
            onSelect={setReason}
            selected={reason}
          />
          <ActionButton
            disabled={busy || reason === null}
            label="Not yet"
            onClick={() =>
              void publish(
                "Not yet",
                commands.buildIoDraftDecide({
                  draftId: model.event.id,
                  outcome: "decline",
                  reason: reason ?? undefined,
                }),
              )
            }
            testId="org-card-not-yet"
            variant="outline"
          />
          {error ? (
            <p className="w-full text-xs text-destructive">{error}</p>
          ) : null}
        </>
      }
    />
  );
}
