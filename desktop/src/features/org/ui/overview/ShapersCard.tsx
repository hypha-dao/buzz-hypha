import * as React from "react";

import type { UserProfileSummary } from "@/shared/api/types";
import { Button } from "@/shared/ui/button";
import { Card } from "@/shared/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/shared/ui/dialog";
import { Input } from "@/shared/ui/input";
import { Textarea } from "@/shared/ui/textarea";

import { useOrgCommands } from "../../useOrgCommands";

import { OrgPersonName } from "./OrgPersonName";
import { RULE_KINDS, type RuleKind } from "./overviewCopy";
import type { ShapersState } from "./parseOverview";

type DialogKind = "add" | "step-down" | "rules" | null;

type ShapersCardProps = {
  shapers: ShapersState | null;
  isShaper: boolean;
  profiles: Record<string, UserProfileSummary>;
};

const RULE_OPTIONS = ["majority", "all", "1", "2"] as const;

function formatRule(value: string | number | undefined): string {
  if (value === undefined) return "majority";
  return String(value);
}

export function ShapersCard({ shapers, isShaper, profiles }: ShapersCardProps) {
  const commands = useOrgCommands();
  const [dialog, setDialog] = React.useState<DialogKind>(null);
  const [pubkey, setPubkey] = React.useState("");
  const [why, setWhy] = React.useState("");
  const [rules, setRules] = React.useState<Record<RuleKind, string>>({
    direction: "majority",
    project: "majority",
    dri: "majority",
    shapers: "majority",
  });
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    if (!dialog) return;
    setPubkey("");
    setWhy("");
    setError(null);
    setBusy(false);
    setRules({
      direction: formatRule(shapers?.rules.direction),
      project: formatRule(shapers?.rules.project),
      dri: formatRule(shapers?.rules.dri),
      shapers: formatRule(shapers?.rules.shapers),
    });
  }, [dialog, shapers]);

  const close = () => setDialog(null);

  const onAdd = async (event: React.FormEvent) => {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await commands.publish(
        commands.buildIoShapersPropose(
          { op: "add", pubkey: pubkey.trim(), why: why.trim() || undefined },
          true,
        ),
      );
      close();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Failed to add.");
    } finally {
      setBusy(false);
    }
  };

  const onStepDown = async (event: React.FormEvent) => {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await commands.publish(
        commands.buildIoShaperStepDown(why.trim() || undefined),
      );
      close();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Failed to step down.");
    } finally {
      setBusy(false);
    }
  };

  const onRules = async (event: React.FormEvent) => {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const next: Record<string, string | number> = { ...shapers?.rules };
      for (const kind of RULE_KINDS) {
        const value = rules[kind];
        next[kind] = /^\d+$/.test(value) ? Number.parseInt(value, 10) : value;
      }
      await commands.publish(
        commands.buildIoShapersPropose({ op: "rules", rules: next }, true),
      );
      close();
    } catch (cause) {
      setError(
        cause instanceof Error ? cause.message : "Failed to change rules.",
      );
    } finally {
      setBusy(false);
    }
  };

  const agentLine = shapers?.agent
    ? shapers.agentHosted
      ? "Hosted by the relay operator"
      : "Run by the Shapers"
    : "Not set yet.";

  return (
    <Card className="p-4" data-testid="org-shapers-card">
      <h2 className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
        Shapers
      </h2>
      {shapers ? (
        <ul className="mt-2 space-y-1" data-testid="org-shapers-members">
          {shapers.shapers.map((member) => (
            <li className="text-sm" key={member}>
              <OrgPersonName profiles={profiles} pubkey={member} />
            </li>
          ))}
        </ul>
      ) : (
        <p className="mt-2 text-sm text-muted-foreground">{agentLine}</p>
      )}
      {shapers ? (
        <>
          <p className="mt-3 text-2xs font-medium uppercase tracking-wider text-muted-foreground">
            Rules
          </p>
          <ul className="mt-1 space-y-0.5" data-testid="org-shapers-rules">
            {RULE_KINDS.map((kind) => (
              <li className="text-sm" key={kind}>
                {kind}: {formatRule(shapers.rules[kind])}
              </li>
            ))}
          </ul>
          <p className="mt-3 text-sm" data-testid="org-shapers-agent">
            Org agent: {agentLine}
            {shapers.agent ? (
              <>
                {" "}
                (
                <OrgPersonName profiles={profiles} pubkey={shapers.agent} />)
              </>
            ) : null}
          </p>
        </>
      ) : null}
      {isShaper ? (
        <div className="mt-3 flex flex-wrap gap-2">
          <Button
            data-testid="org-shapers-add"
            onClick={() => setDialog("add")}
            size="sm"
            type="button"
          >
            Add a Shaper
          </Button>
          <Button
            data-testid="org-shapers-step-down"
            onClick={() => setDialog("step-down")}
            size="sm"
            type="button"
            variant="outline"
          >
            Step down
          </Button>
          <Button
            data-testid="org-shapers-rules-change"
            onClick={() => setDialog("rules")}
            size="sm"
            type="button"
            variant="outline"
          >
            Change the rules
          </Button>
        </div>
      ) : null}

      <Dialog onOpenChange={(open) => !open && close()} open={dialog === "add"}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Add a Shaper</DialogTitle>
            <DialogDescription>
              Opens a shapers proposal. The named person joins only after they
              accept the seat.
            </DialogDescription>
          </DialogHeader>
          <form className="space-y-3" onSubmit={(event) => void onAdd(event)}>
            <div className="space-y-1.5">
              <label
                className="text-sm font-medium"
                htmlFor="org-shaper-pubkey"
              >
                Pubkey
              </label>
              <Input
                autoComplete="off"
                data-testid="org-shaper-pubkey"
                id="org-shaper-pubkey"
                onChange={(event) => setPubkey(event.target.value)}
                required
                value={pubkey}
              />
            </div>
            <div className="space-y-1.5">
              <label
                className="text-sm font-medium"
                htmlFor="org-shaper-add-why"
              >
                Why
              </label>
              <Textarea
                data-testid="org-shaper-add-why"
                id="org-shaper-add-why"
                onChange={(event) => setWhy(event.target.value)}
                rows={2}
                value={why}
              />
            </div>
            {error ? (
              <p className="text-sm text-destructive" role="alert">
                {error}
              </p>
            ) : null}
            <DialogFooter>
              <Button
                data-testid="org-shaper-add-submit"
                disabled={busy || pubkey.trim().length === 0}
                type="submit"
              >
                Add a Shaper
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <Dialog
        onOpenChange={(open) => !open && close()}
        open={dialog === "step-down"}
      >
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Step down</DialogTitle>
            <DialogDescription>
              Leaves the Shaper set. The last Shaper cannot step down.
            </DialogDescription>
          </DialogHeader>
          <form
            className="space-y-3"
            onSubmit={(event) => void onStepDown(event)}
          >
            <div className="space-y-1.5">
              <label
                className="text-sm font-medium"
                htmlFor="org-shaper-step-why"
              >
                Why
              </label>
              <Textarea
                data-testid="org-shaper-step-why"
                id="org-shaper-step-why"
                onChange={(event) => setWhy(event.target.value)}
                rows={2}
                value={why}
              />
            </div>
            {error ? (
              <p className="text-sm text-destructive" role="alert">
                {error}
              </p>
            ) : null}
            <DialogFooter>
              <Button
                data-testid="org-shaper-step-submit"
                disabled={busy}
                type="submit"
                variant="destructive"
              >
                Step down
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <Dialog
        onOpenChange={(open) => !open && close()}
        open={dialog === "rules"}
      >
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Change the rules</DialogTitle>
            <DialogDescription>
              A rules change always needs every Shaper. Open proposals keep
              their stored bar.
            </DialogDescription>
          </DialogHeader>
          <form className="space-y-3" onSubmit={(event) => void onRules(event)}>
            {RULE_KINDS.map((kind) => {
              const fieldId = `org-rule-${kind}`;
              return (
                <div className="space-y-1.5" key={kind}>
                  <label className="text-sm font-medium" htmlFor={fieldId}>
                    {kind}
                  </label>
                  <select
                    className="flex h-9 w-full rounded-lg border border-input/40 bg-background px-3 text-sm"
                    data-testid={fieldId}
                    id={fieldId}
                    onChange={(event) =>
                      setRules((current) => ({
                        ...current,
                        [kind]: event.target.value,
                      }))
                    }
                    value={rules[kind]}
                  >
                    {RULE_OPTIONS.map((option) => (
                      <option key={option} value={option}>
                        {option}
                      </option>
                    ))}
                  </select>
                </div>
              );
            })}
            {error ? (
              <p className="text-sm text-destructive" role="alert">
                {error}
              </p>
            ) : null}
            <DialogFooter>
              <Button
                data-testid="org-shaper-rules-submit"
                disabled={busy}
                type="submit"
              >
                Change the rules
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </Card>
  );
}
