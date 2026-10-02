import * as React from "react";

import { useUsersBatchQuery } from "@/features/profile/hooks";
import { resolveUserLabel } from "@/features/profile/lib/identity";
import { useIdentityQuery } from "@/shared/api/hooks";

import {
  driDraftSentence,
  draftKindLabel,
  draftSeries,
  draftTitle,
  openChatDrafts,
  type ChatDraft,
  type ChatDraftMessage,
} from "../chatDraft";
import { useOverviewEvents } from "../hooks/useOverviewEvents";
import { useWorkEvents } from "../hooks/useWorkEvents";
import { parseShapersState } from "./overview/parseOverview";
import { ProposalDraftDialog } from "./ProposalDraftDialog";
import { latestWorkItems, parseWorkItem } from "../work/model";

const STORAGE_KEY = "buzz.org.publishedDrafts";
const MAX_PUBLISHED = 200;

type FooterMap = Record<string, React.ReactNode>;

function loadPublished(): Set<string> {
  try {
    const raw = sessionStorage.getItem(STORAGE_KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : [];
    if (!Array.isArray(parsed)) return new Set();
    return new Set(parsed.filter((id): id is string => typeof id === "string"));
  } catch {
    return new Set();
  }
}

function pubkeyOf(draft: ChatDraft): string | null {
  switch (draft.kind) {
    case "project":
      return draft.suggestedDri;
    case "dri":
    case "shapers-add":
    case "shapers-remove":
    case "shapers-agent":
      return draft.pubkey;
    default:
      return null;
  }
}

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
  const [published, setPublished] =
    React.useState<ReadonlySet<string>>(loadPublished);
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
    return openChatDrafts(messages, orgAgentPubkey, published, workItems);
  }, [messages, orgAgentPubkey, published, workItems]);
  const titles = React.useMemo(() => {
    const map = new Map<string, string>();
    for (const item of workItems) map.set(item.id, item.title);
    return map;
  }, [workItems]);
  const pubkeys = React.useMemo(() => {
    const found = new Set<string>();
    for (const draft of drafts) {
      const pubkey = pubkeyOf(draft);
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

  const viewerIsShaper = Boolean(
    viewer &&
      shapers?.shapers.some(
        (pubkey) => pubkey.trim().toLowerCase() === viewer.trim().toLowerCase(),
      ),
  );
  const canPublish = Boolean(
    openDraft &&
      viewer &&
      (openDraft.kind === "dri"
        ? openDraft.from === viewer.trim().toLowerCase()
        : viewerIsShaper),
  );
  const personPubkey = openDraft ? pubkeyOf(openDraft) : null;
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
          <span className="mt-1 block text-sm text-foreground">{title}</span>
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
    setPublished((current) => {
      const next = new Set(current);
      next.add(messageId);
      const kept =
        next.size > MAX_PUBLISHED
          ? new Set([...next].slice(-MAX_PUBLISHED))
          : next;
      try {
        sessionStorage.setItem(STORAGE_KEY, JSON.stringify([...kept]));
      } catch {
        // The in-memory set still hides the draft in this view.
      }
      return kept;
    });
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
