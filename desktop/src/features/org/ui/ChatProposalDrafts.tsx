import * as React from "react";

import { useUsersBatchQuery } from "@/features/profile/hooks";
import { resolveUserLabel } from "@/features/profile/lib/identity";
import { useIdentityQuery } from "@/shared/api/hooks";

import {
  directionBodyLines,
  driDraftSentence,
  draftKindLabel,
  draftSeries,
  draftTitle,
  openChatDrafts,
  type ChatDraftMessage,
} from "../chatDraft";
import { useDraftLifecycle } from "../draftLifecycle";
import { draftPersonPubkey, draftPublishableBy } from "../myDrafts";
import { useOverviewEvents } from "../hooks/useOverviewEvents";
import { useWorkEvents } from "../hooks/useWorkEvents";
import { parseShapersState } from "./overview/parseOverview";
import { ProposalDraftDialog } from "./ProposalDraftDialog";
import { latestWorkItems, parseWorkItem } from "../work/model";

type FooterMap = Record<string, React.ReactNode>;

/**
 * Draft cards under the agent's messages, in the DM and in every channel.
 * Clicking one opens the same editor. Publish is the same for one Shaper
 * or many.
 */
export function ChatProposalDrafts({
  messages,
  orgAgentPubkey,
  onFooters,
  onSentences,
}: {
  messages: readonly ChatDraftMessage[];
  orgAgentPubkey: string | null;
  onFooters: (signature: string, footers: FooterMap) => void;
  onSentences?: (signature: string, sentences: Record<string, string>) => void;
}) {
  const viewer = useIdentityQuery().data?.pubkey ?? null;
  const overview = useOverviewEvents();
  const work = useWorkEvents();
  const lifecycle = useDraftLifecycle(viewer);
  const [unrecorded, setUnrecorded] = React.useState<ReadonlySet<string>>(
    () => new Set(),
  );
  const closed = React.useMemo(
    () =>
      unrecorded.size === 0
        ? lifecycle.closed
        : new Set([...lifecycle.closed, ...unrecorded]),
    [lifecycle.closed, unrecorded],
  );
  const [openId, setOpenId] = React.useState<string | null>(null);
  const shapers = React.useMemo(
    () => parseShapersState(overview.events),
    [overview.events],
  );
  const workItems = React.useMemo(() => {
    const rows = [];
    for (const event of latestWorkItems(work.events).values()) {
      const item = parseWorkItem(event);
      if (!item) continue;
      rows.push({
        id: item.id,
        title: item.title,
        state: item.state,
        dri: item.dri,
      });
    }
    return rows;
  }, [work.events]);
  const drafts = React.useMemo(() => {
    if (!orgAgentPubkey) return [];
    return openChatDrafts(messages, orgAgentPubkey, closed, workItems);
  }, [closed, messages, orgAgentPubkey, workItems]);
  const titles = React.useMemo(() => {
    const map = new Map<string, string>();
    for (const item of workItems) map.set(item.id, item.title);
    return map;
  }, [workItems]);
  const pubkeys = React.useMemo(() => {
    const found = new Set<string>();
    for (const draft of drafts) {
      const pubkey = draftPersonPubkey(draft);
      if (pubkey) found.add(pubkey);
    }
    return [...found];
  }, [drafts]);
  const profiles = useUsersBatchQuery(pubkeys).data?.profiles;
  const openDraft = React.useMemo(() => {
    const selected = drafts.find((draft) => draft.messageId === openId);
    if (!selected) return null;
    const series = draftSeries(selected);
    return (
      [...drafts].reverse().find((draft) => draftSeries(draft) === series) ??
      selected
    );
  }, [drafts, openId]);

  const canPublish = openDraft
    ? draftPublishableBy(openDraft, viewer, shapers?.shapers)
    : false;
  const personPubkey = openDraft ? draftPersonPubkey(openDraft) : null;
  const personName = personPubkey
    ? resolveUserLabel({
        pubkey: personPubkey,
        currentPubkey: viewer ?? undefined,
        profiles,
      })
    : null;

  const sentences = React.useMemo(() => {
    const record: Record<string, string> = {};
    for (const draft of drafts) {
      if (draft.kind !== "dri") continue;
      const title = titles.get(draft.itemId)?.trim();
      if (!title) continue;
      const label = resolveUserLabel({
        pubkey: draft.pubkey,
        currentPubkey: viewer ?? undefined,
        preferResolvedSelfLabel: true,
        profiles,
      });
      const holderName = label.startsWith("npub") ? null : label;
      record[draft.messageId] = driDraftSentence(title, holderName);
    }
    return record;
  }, [drafts, profiles, titles, viewer]);
  const sentenceSignature = Object.entries(sentences)
    .map(([id, text]) => `${id}:${text}`)
    .join("|");

  const signature = drafts
    .map((draft) => `${draft.messageId}:${draftSeries(draft)}`)
    .join("|");
  const footers = React.useMemo(() => {
    const record: FooterMap = {};
    for (const draft of drafts) {
      const title = draftTitle(
        draft,
        titles.get(
          draft.kind === "dri" || draft.kind === "remove-project"
            ? draft.itemId
            : "",
        ),
      );
      record[draft.messageId] = (
        <button
          className="mt-2 block max-w-xl rounded-xl border border-border/60 bg-card px-4 py-3 text-left shadow-xs hover:bg-muted/40 focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring"
          data-testid="org-chat-draft"
          onClick={() => setOpenId(draft.messageId)}
          type="button"
        >
          <span className="block text-xs font-medium text-muted-foreground">
            Draft · {draftKindLabel(draft)}
          </span>
          {draft.kind === "direction" || draft.kind === "revise-direction" ? (
            draft.slug === "objectives" || draft.slug === "strategy" ? (
              <span className="mt-2 block space-y-1.5 text-sm text-foreground">
                {directionBodyLines(draft.slug, draft.body).map((line) => (
                  <span
                    className="block rounded-md border border-border/60 bg-background/40 px-3 py-1.5"
                    key={line}
                  >
                    {line}
                  </span>
                ))}
              </span>
            ) : (
              <span className="mt-1 block whitespace-pre-wrap text-sm text-foreground">
                {draft.body}
              </span>
            )
          ) : (
            <span className="mt-1 block text-sm text-foreground">{title}</span>
          )}
          <span className="mt-2 block text-xs font-medium text-foreground">
            Open
          </span>
        </button>
      );
    }
    return record;
  }, [drafts, titles]);

  React.useEffect(() => {
    onFooters(signature, footers);
  }, [footers, onFooters, signature]);

  React.useEffect(() => {
    return () => onFooters("", {});
  }, [onFooters]);

  React.useEffect(() => {
    onSentences?.(sentenceSignature, sentences);
  }, [onSentences, sentenceSignature, sentences]);

  React.useEffect(() => {
    return () => onSentences?.("", {});
  }, [onSentences]);

  const remember = (messageId: string) => {
    if (!lifecycle.markPublished(messageId)) {
      // Storage refused it. Still hide the card in this view.
      setUnrecorded((current) => new Set([...current, messageId]));
    }
    setOpenId(null);
  };

  return (
    <ProposalDraftDialog
      canPublish={canPublish}
      draft={openId ? openDraft : null}
      itemTitle={
        openDraft &&
        (openDraft.kind === "dri" || openDraft.kind === "remove-project")
          ? (titles.get(openDraft.itemId) ?? null)
          : null
      }
      onOpenChange={(open) => {
        if (!open) setOpenId(null);
      }}
      onPublished={remember}
      personName={personName}
    />
  );
}
