import { ActionButton } from "./CardActions";
import { OrgCardShell } from "./OrgCardShell";
import type { OrgCardModel } from "./types";
import { useCardCommand } from "./useCardCommand";

export function DecisionCard({ model }: { model: OrgCardModel }) {
  const { commands, error, pending, publish } = useCardCommand();
  const proposal = model.proposalId;
  const busy = pending !== null;

  return (
    <OrgCardShell
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
