import * as React from "react";

import { ActionButton, DeclineChips } from "./CardActions";
import {
  directionProposeInput,
  profileSetInput,
  projectProposeInput,
  ticketCreateInput,
} from "./draftPayload";
import { OrgCardShell } from "./OrgCardShell";
import type { DeclineReason, OrgCardModel } from "./types";
import { useCardCommand } from "./useCardCommand";

export function DraftCard({ model }: { model: OrgCardModel }) {
  const { commands, error, pending, publish } = useCardCommand();
  const [reason, setReason] = React.useState<DeclineReason | null>(null);
  const [editing, setEditing] = React.useState(false);
  const [title, setTitle] = React.useState(model.claim);
  const busy = pending !== null;

  const agree = async (nextTitle?: string) => {
    const kind = model.draftKind;
    if (kind === "ticket") {
      await publish(
        "Agree",
        commands.buildIoTicketCreate(ticketCreateInput(model, nextTitle)),
      );
      return;
    }
    if (kind === "direction" || kind === "objectives") {
      await publish(
        "Agree",
        commands.buildIoDirectionPropose(directionProposeInput(model)),
      );
      return;
    }
    if (kind === "profile") {
      await publish(
        "Agree",
        commands.buildIoProfileSet(profileSetInput(model)),
      );
      return;
    }
    await publish(
      "Agree",
      commands.buildIoProjectPropose(projectProposeInput(model, nextTitle)),
    );
  };

  return (
    <OrgCardShell
      extra={
        editing ? (
          <fieldset className="mt-3 border-0 p-0">
            <legend className="sr-only">Edit then agree</legend>
            <label
              className="block text-xs text-muted-foreground"
              htmlFor={`org-edit-${model.event.id}`}
            >
              Title
              <input
                className="mt-1 w-full rounded-md border border-input bg-background px-2 py-1 text-sm"
                id={`org-edit-${model.event.id}`}
                onChange={(event) => setTitle(event.target.value)}
                value={title}
              />
            </label>
            <div className="mt-2">
              <ActionButton
                disabled={busy}
                label="Agree with edit"
                onClick={() => void agree(title)}
                testId="org-card-agree-edit"
              />
            </div>
          </fieldset>
        ) : null
      }
      model={model}
      actions={
        <>
          <ActionButton
            disabled={busy}
            label="Agree"
            onClick={() => void agree()}
            testId="org-card-agree"
          />
          <ActionButton
            disabled={busy}
            label="Edit then agree"
            onClick={() => setEditing(true)}
            testId="org-card-edit-then-agree"
            variant="outline"
          />
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
          {error ? (
            <p className="w-full text-xs text-destructive">{error}</p>
          ) : null}
        </>
      }
    />
  );
}
