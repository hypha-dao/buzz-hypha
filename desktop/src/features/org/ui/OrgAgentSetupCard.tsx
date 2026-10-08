import * as React from "react";

import { useIdentityQuery } from "@/shared/api/hooks";
import { mintInvite } from "@/shared/api/invites";
import { writeTextToClipboard } from "@/shared/lib/clipboard";
import { Button } from "@/shared/ui/button";
import { normalizePubkey } from "@/shared/lib/pubkey";

import { useOrgCommands } from "../useOrgCommands";
import { fetchShapersSnapshot } from "../useOrgAgent";
import {
  parseOrgSetupMessage,
  readSetupResult,
  runOrgSetup,
  writeSetupResult,
  type OrgSetup,
} from "../orgSetup";

type SetupMessage = {
  id: string;
  pubkey?: string;
  signerPubkey?: string;
  tags?: string[][];
};

type SetupView = {
  error: string | null;
  setup: OrgSetup;
  url: string | null;
};

const inFlight = new Set<string>();

function decisionRules(need: number, of: number) {
  const share = `${need}/${of}`;
  return {
    direction: share,
    dri: share,
    join: share,
    money: share,
    project: share,
    shapers: share,
  };
}

/**
 * Creates `#shapers` and, when they invited others, a copyable invite link
 * under the org agent's message.
 */
export function OrgAgentSetupCard({
  messages,
  onFooters,
  orgAgentPubkey,
}: {
  messages: readonly SetupMessage[];
  onFooters: (
    signature: string,
    footers: Record<string, React.ReactNode>,
  ) => void;
  orgAgentPubkey: string | null;
}) {
  const identity = useIdentityQuery();
  const commands = useOrgCommands();
  const pubkey = identity.data?.pubkey
    ? normalizePubkey(identity.data.pubkey)
    : null;
  const [views, setViews] = React.useState<Record<string, SetupView>>({});
  const [copiedId, setCopiedId] = React.useState<string | null>(null);

  const targets = React.useMemo(() => {
    const found: { id: string; setup: OrgSetup }[] = [];
    for (const message of messages) {
      const setup = parseOrgSetupMessage(message, orgAgentPubkey);
      if (setup) found.push({ id: message.id, setup });
    }
    return found;
  }, [messages, orgAgentPubkey]);

  const run = React.useCallback(
    async (messageId: string, setup: OrgSetup) => {
      if (!pubkey || inFlight.has(messageId)) return;
      const stored = readSetupResult(messageId);
      if (stored) {
        setViews((current) => ({
          ...current,
          [messageId]: { error: null, setup, url: stored.url },
        }));
        return;
      }
      inFlight.add(messageId);
      setViews((current) => ({
        ...current,
        [messageId]: { error: null, setup, url: null },
      }));
      try {
        const snapshot = await fetchShapersSnapshot();
        const result = await runOrgSetup({
          bootstrapped: snapshot.bootstrapped,
          mint: (maxUses) => mintInvite({ maxUses, ttlSecs: 60 * 60 * 24 * 7 }),
          publishBootstrap: () =>
            commands.publish(
              commands.buildIoShapersPropose({ op: "add", pubkey }, true),
            ),
          publishRules: (need, of) =>
            commands.publish(
              commands.buildIoShapersPropose(
                { op: "rules", rules: decisionRules(need, of) },
                true,
              ),
            ),
          setup,
        });
        writeSetupResult(messageId, result.url);
        setViews((current) => ({
          ...current,
          [messageId]: { error: null, setup, url: result.url },
        }));
      } catch (error) {
        setViews((current) => ({
          ...current,
          [messageId]: {
            error:
              error instanceof Error
                ? error.message
                : "Could not finish Shaper setup",
            setup,
            url: null,
          },
        }));
      } finally {
        inFlight.delete(messageId);
      }
    },
    [commands, pubkey],
  );

  React.useEffect(() => {
    for (const target of targets) {
      void run(target.id, target.setup);
    }
  }, [run, targets]);

  const signature = targets
    .map((target) => {
      const view = views[target.id];
      return `${target.id}:${view?.url ?? ""}:${view?.error ?? ""}`;
    })
    .join("|");

  const footers = React.useMemo(() => {
    const record: Record<string, React.ReactNode> = {};
    for (const target of targets) {
      const view = views[target.id];
      if (view?.setup.kind !== "invite") {
        if (view?.error && view.setup.kind === "alone") {
          record[target.id] = (
            <SetupError
              message={view.error}
              onRetry={() => {
                void run(target.id, view.setup);
              }}
            />
          );
        }
        continue;
      }
      record[target.id] = (
        <InviteLink
          copied={copiedId === target.id}
          error={view.error}
          onCopy={async () => {
            if (!view.url) return;
            await writeTextToClipboard(view.url);
            setCopiedId(target.id);
          }}
          onRetry={() => {
            void run(target.id, view.setup);
          }}
          url={view.url}
        />
      );
    }
    return record;
  }, [copiedId, run, targets, views]);

  React.useEffect(() => {
    onFooters(signature, footers);
  }, [footers, onFooters, signature]);

  React.useEffect(() => {
    return () => onFooters("", {});
  }, [onFooters]);

  return null;
}

function SetupError({
  message,
  onRetry,
}: {
  message: string;
  onRetry: () => void;
}) {
  return (
    <div className="mt-2 max-w-md" data-testid="org-shaper-setup-error">
      <p className="text-sm text-destructive">{message}</p>
      <Button className="mt-2" onClick={onRetry} size="sm" type="button">
        Try again
      </Button>
    </div>
  );
}

function InviteLink({
  copied,
  error,
  onCopy,
  onRetry,
  url,
}: {
  copied: boolean;
  error: string | null;
  onCopy: () => Promise<void>;
  onRetry: () => void;
  url: string | null;
}) {
  const inputId = React.useId();
  return (
    <div
      className="mt-3 max-w-md rounded-lg border border-border/70 bg-card px-3 py-3"
      data-testid="org-shaper-invite"
    >
      {url ? (
        <>
          <label
            className="text-sm font-medium text-foreground"
            htmlFor={inputId}
          >
            Invite link
          </label>
          <div className="mt-2 flex items-center gap-2">
            <input
              className="min-w-0 flex-1 rounded-md border border-border bg-background px-2 py-1.5 text-sm text-foreground"
              data-testid="org-shaper-invite-url"
              id={inputId}
              readOnly
              value={url}
            />
            <Button
              data-testid="org-shaper-invite-copy"
              onClick={() => {
                void onCopy();
              }}
              size="sm"
              type="button"
            >
              {copied ? "Copied" : "Copy"}
            </Button>
          </div>
        </>
      ) : error ? (
        <SetupError message={error} onRetry={onRetry} />
      ) : (
        <p className="text-sm text-muted-foreground">
          Creating the Shapers channel and your invite link…
        </p>
      )}
    </div>
  );
}
