import * as React from "react";

import { cn } from "@/shared/lib/cn";
import { Button } from "@/shared/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/shared/ui/dialog";
import { Textarea } from "@/shared/ui/textarea";

import { ORG_PAPER_CLASS } from "../orgPaper";
import type { DirectionSlug } from "../../commands";
import { useOrgCommands } from "../../useOrgCommands";

import { DIRECTION_LABEL } from "./overviewCopy";

type DirectionFormDialogProps = {
  slug: DirectionSlug;
  base: number;
  open: boolean;
  onOpenChange: (open: boolean) => void;
};

export function DirectionFormDialog({
  slug,
  base,
  open,
  onOpenChange,
}: DirectionFormDialogProps) {
  const commands = useOrgCommands();
  const [body, setBody] = React.useState("");
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const label = DIRECTION_LABEL[slug];
  const bodyId = `org-direction-body-${slug}`;

  React.useEffect(() => {
    if (!open) return;
    setBody("");
    setError(null);
    setBusy(false);
  }, [open]);

  const onSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await commands.publish(
        commands.buildIoDirectionPropose({
          slug,
          base,
          body,
          voteAgree: false,
        }),
      );
      onOpenChange(false);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Failed to propose.");
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog onOpenChange={onOpenChange} open={open}>
      <DialogContent className={cn("max-w-lg", ORG_PAPER_CLASS)}>
        <DialogHeader>
          <DialogTitle>Propose a new {label.title.toLowerCase()}</DialogTitle>
          <DialogDescription>
            Opens a direction proposal at base {base}. Other Shapers confirm it
            on My Work.
          </DialogDescription>
        </DialogHeader>
        <form className="space-y-3" onSubmit={(event) => void onSubmit(event)}>
          <div className="space-y-1.5">
            <label className="text-sm font-medium" htmlFor={bodyId}>
              Body
            </label>
            <Textarea
              data-testid={`org-direction-body-${slug}`}
              id={bodyId}
              onChange={(event) => setBody(event.target.value)}
              required
              rows={6}
              value={body}
            />
          </div>
          {error ? (
            <p className="text-sm text-destructive" role="alert">
              {error}
            </p>
          ) : null}
          <DialogFooter>
            <Button
              data-testid={`org-direction-submit-${slug}`}
              disabled={busy || body.trim().length === 0}
              type="submit"
            >
              Propose
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
