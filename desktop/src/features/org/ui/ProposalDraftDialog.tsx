import { Link } from "@tanstack/react-router";
import { Check, ChevronDown } from "lucide-react";
import * as React from "react";

import { channelNamesMatch } from "@/features/channels/lib/canonicalChannelName";
import { useChannelsQuery } from "@/features/channels/hooks";
import { useRelayMembersQuery } from "@/features/community-members/hooks";
import { useSendMessageMutation } from "@/features/messages/hooks";
import { useProfileQuery, useUsersBatchQuery } from "@/features/profile/hooks";
import { useIdentityQuery } from "@/shared/api/hooks";
import type { Channel, Profile, UserProfileSummary } from "@/shared/api/types";
import { cn } from "@/shared/lib/cn";
import { normalizePubkey, truncateNpub } from "@/shared/lib/pubkey";
import { Button } from "@/shared/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/shared/ui/dropdown-menu";
import { UserAvatar } from "@/shared/ui/UserAvatar";
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

import {
  announcementLabel,
  commandForDraft,
  directionChunks,
  directionDraftText,
  dateInputToUnix,
  dateInputValue,
  draftKindLabel,
  draftTitle,
  emptyRules,
  proposalAnnouncement,
  RULE_KINDS,
  type ChatDraft,
  type CodebaseItem,
  type RuleKind,
} from "../chatDraft";
import { publishOrgCommand, type DirectionSlug } from "../commands";
import {
  linesForDirectionPropose,
  presentStrategyDraft,
  readStrategyLine,
  strategyLinesForPublish,
  type StrategyLineType,
} from "../directionLines";

import { useOverviewEvents } from "../hooks/useOverviewEvents";
import { useOrgAgentPubkey } from "../useOrgAgent";
import { ORG_PAPER_CLASS } from "./orgPaper";
import { directionSlots, parseShapersState } from "./overview/parseOverview";

type ProposalDraftDialogProps = {
  draft: ChatDraft | null;
  canPublish: boolean;
  itemTitle: string | null;
  personName: string | null;
  onOpenChange: (open: boolean) => void;
  onPublished: (messageId: string) => void;
};

function shapersChannel(
  channels: readonly Channel[] | undefined,
  roomId: string | null,
): Channel | null {
  if (roomId) {
    const byId = channels?.find((channel) => channel.id === roomId);
    if (byId) return byId;
  }
  return (
    channels?.find(
      (channel) =>
        channel.channelType !== "dm" &&
        channelNamesMatch(channel.name, "shapers"),
    ) ?? null
  );
}

function lined(slug: DirectionSlug): boolean {
  return slug === "objectives" || slug === "strategy";
}

export function ProposalDraftDialog({
  draft,
  canPublish,
  itemTitle,
  personName,
  onOpenChange,
  onPublished,
}: ProposalDraftDialogProps) {
  const overview = useOverviewEvents();
  const identity = useIdentityQuery().data;
  const orgAgentPubkey = useOrgAgentPubkey();
  const picksHolder = draft?.kind === "project" || draft?.kind === "dri";
  const members = useRelayMembersQuery(picksHolder).data;
  const memberPubkeys = React.useMemo(
    () => (members ?? []).map((member) => member.pubkey),
    [members],
  );
  const profiles = useUsersBatchQuery(memberPubkeys, {
    enabled: picksHolder,
  }).data?.profiles;
  const selfProfile = useProfileQuery(picksHolder).data;
  const channels = useChannelsQuery().data;
  const shapersRoomId = parseShapersState(overview.events)?.room ?? null;
  const room = shapersChannel(channels, shapersRoomId);
  const publishedCommand = React.useRef<string | null>(null);
  const publishedFor = React.useRef<string | null>(null);
  const send = useSendMessageMutation(room, identity);
  const [title, setTitle] = React.useState("");
  const [brief, setBrief] = React.useState("");
  const [body, setBody] = React.useState("");
  const [strategyTypes, setStrategyTypes] = React.useState<StrategyLineType[]>(
    [],
  );
  const [dueValue, setDueValue] = React.useState("");
  const [rules, setRules] = React.useState(emptyRules);
  const [decisionDays, setDecisionDays] = React.useState("");
  const [offerDays, setOfferDays] = React.useState("");
  const [agentPubkey, setAgentPubkey] = React.useState("");
  const [holder, setHolder] = React.useState("");
  const [codebaseItems, setCodebaseItems] = React.useState<
    (CodebaseItem & { key: string })[]
  >([]);
  const [dirty, setDirty] = React.useState(false);
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const seeded = React.useRef<string | null>(null);

  const directionBase = React.useMemo(() => {
    if (
      !draft ||
      (draft.kind !== "direction" && draft.kind !== "revise-direction")
    ) {
      return 0;
    }
    if (draft.kind === "revise-direction") return draft.base;
    const head = directionSlots(overview.events).find(
      (slot) => slot.slug === draft.slug,
    )?.head;
    return head?.version ?? 0;
  }, [draft, overview.events]);

  const draftId = draft?.messageId ?? null;
  if (publishedFor.current !== draftId) {
    publishedFor.current = draftId;
    publishedCommand.current = null;
  }
  React.useEffect(() => {
    if (!draft || !draftId) {
      seeded.current = null;
      setDirty(false);
      return;
    }
    if (dirty && seeded.current && seeded.current !== draftId) return;
    if (seeded.current === draftId) return;
    seeded.current = draftId;
    setDirty(false);
    setError(null);
    setBusy(false);
    if (draft.kind === "project" || draft.kind === "revise-project") {
      setTitle(draft.title);
      setBrief(draft.brief);
      setDueValue(dateInputValue(draft.dueAt));
      setHolder(draft.kind === "project" ? (draft.suggestedDri ?? "") : "");
      setBody("");
      return;
    }
    if (draft.kind === "dri") {
      setHolder(draft.pubkey);
      setTitle("");
      setBrief("");
      setBody("");
      return;
    }
    if (draft.kind === "direction" || draft.kind === "revise-direction") {
      const head = directionSlots(overview.events).find(
        (slot) => slot.slug === draft.slug,
      )?.head;
      if (draft.slug === "strategy") {
        const existing =
          draft.kind === "direction" && head
            ? head.lines.filter((line) => line.text)
            : [];
        const presented = presentStrategyDraft(
          directionChunks("strategy", draft.body),
          existing,
        );
        const seeded =
          presented.length > 0
            ? presented
            : [{ text: "", type: "bet" as const }];
        setBody(seeded.map((line) => line.text).join("\n"));
        setStrategyTypes(seeded.map((line) => line.type));
      } else {
        const existing =
          draft.kind === "direction" && lined(draft.slug) && head
            ? head.lines.map((line) => line.text).filter(Boolean)
            : [];
        setBody(directionDraftText(draft.slug, draft.body, existing));
        setStrategyTypes([]);
      }
      setTitle("");
      setBrief("");
      return;
    }
    if (draft.kind === "codebases") {
      setCodebaseItems(
        draft.items.map((item) => ({ ...item, key: crypto.randomUUID() })),
      );
      setTitle("");
      setBrief("");
      setBody("");
      return;
    }
    if (draft.kind === "shapers-rules") {
      const live = parseShapersState(overview.events);
      const next = emptyRules();
      if (live?.rules) {
        for (const key of RULE_KINDS) {
          const value = live.rules[key];
          if (typeof value === "string" || typeof value === "number") {
            next[key] = String(value);
          }
        }
      }
      for (const key of RULE_KINDS) {
        const value = draft.rules[key];
        if (value) next[key] = value;
      }
      setRules(next);
      setDecisionDays(
        daysOf(draft.decisionWindowSecs ?? live?.decisionWindowSecs ?? null),
      );
      setOfferDays(
        daysOf(draft.offerWindowSecs ?? live?.offerWindowSecs ?? null),
      );
      setAgentPubkey("");
      setTitle("");
      setBrief("");
      setBody("");
      return;
    }
    if (draft.kind === "shapers-agent") {
      setAgentPubkey(draft.pubkey ?? "");
      setTitle("");
      setBrief("");
      setBody("");
      return;
    }
    setTitle("");
    setBrief("");
    setBody("");
  }, [draft, draftId, dirty, overview.events]);

  if (!draft) return null;
  const dueAt =
    draft.kind === "project" || draft.kind === "revise-project"
      ? dateInputToUnix(dueValue, draft.dueAt)
      : 0;
  const label = draftKindLabel(draft);
  const heading = draftTitle(draft, itemTitle);

  const onPublish = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!canPublish) return;
    if (draft.kind === "dri" && !holder.trim()) return;
    const slug =
      draft.kind === "direction" || draft.kind === "revise-direction"
        ? draft.slug
        : null;
    const hasLine =
      slug !== null &&
      lined(slug) &&
      draftLines(body).some((line) => line.trim().length >= 12);
    if (slug && lined(slug) && !hasLine) {
      setError(
        slug === "objectives"
          ? "Add at least one objective."
          : "Add at least one strategy.",
      );
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const linedSlug =
        slug === "objectives" || slug === "strategy" ? slug : null;
      const parsed =
        linedSlug === "strategy"
          ? strategyLinesForPublish(body, strategyTypes)
          : linedSlug
            ? linesForDirectionPropose(linedSlug, body)
            : null;
      if (parsed && "error" in parsed) {
        setError(parsed.error);
        setBusy(false);
        return;
      }
      const lines = parsed && "lines" in parsed ? parsed.lines : undefined;
      if (!publishedCommand.current) {
        const command = await publishOrgCommand(
          commandForDraft({
            draft,
            title: title.trim(),
            brief: brief.trim(),
            body: body.trim(),
            dueAt,
            why: storedWhy(draft),
            directionBase,
            lines,
            rules,
            decisionWindowSecs: daysToSecs(decisionDays),
            offerWindowSecs: daysToSecs(offerDays),
            agentPubkey: agentPubkey.trim() || null,
            suggestedDri:
              draft.kind === "project" ? holder.trim() || null : undefined,
            holder: draft.kind === "dri" ? holder.trim() : undefined,
            codebases:
              draft.kind === "codebases"
                ? codebaseItems.map(({ kind, name, url, about }) => ({
                    kind,
                    name,
                    url,
                    about,
                  }))
                : undefined,
          }),
        );
        publishedCommand.current = command.id;
      }
      const announcedTitle = draftTitle(
        draft.kind === "project" || draft.kind === "revise-project"
          ? { ...draft, title: title.trim(), brief: brief.trim() }
          : draft.kind === "direction" || draft.kind === "revise-direction"
            ? { ...draft, body: body.trim() }
            : draft,
        itemTitle,
      );
      const announced = proposalAnnouncement(
        announcementLabel(draft),
        announcedTitle,
      );
      if (!room) {
        throw new Error(
          "The proposal is on My work. #shapers is not open yet, so the chat did not get the message.",
        );
      }
      await send.mutateAsync({ targetChannel: room, content: announced });
      onPublished(draft.messageId);
      onOpenChange(false);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Could not publish.");
    } finally {
      setBusy(false);
    }
  };

  const mark =
    (setter: (value: string) => void) =>
    (event: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
      setDirty(true);
      setter(event.target.value);
    };

  return (
    <Dialog onOpenChange={onOpenChange} open>
      <DialogContent
        className={cn(
          "max-h-[calc(100vh-2rem)] max-w-3xl overflow-y-auto",
          ORG_PAPER_CLASS,
        )}
      >
        <DialogHeader>
          <DialogTitle>{label} draft</DialogTitle>
          <DialogDescription>
            {draft.kind === "dri"
              ? "Choose who holds this project. Publish asks the Shapers to name them."
              : draft.kind === "codebases"
                ? "Each repository is a name, a link, and one line on what it is. The landing page is one link on this list. Publish opens a proposal. Strategy stays as it is."
                : "Change the fields here, or tell the agent in the chat. Publish when it is right. It then shows on My work for every Shaper, and in the Shapers chat."}
          </DialogDescription>
        </DialogHeader>
        <form className="space-y-3" onSubmit={(event) => void onPublish(event)}>
          {draft.kind === "project" || draft.kind === "revise-project" ? (
            <>
              <Field label="Title" id="org-draft-title">
                <Input
                  data-testid="org-draft-title"
                  id="org-draft-title"
                  onChange={mark(setTitle)}
                  readOnly={!canPublish}
                  required
                  value={title}
                />
              </Field>
              <Field label="Brief" id="org-draft-brief">
                <Textarea
                  className="min-h-72 resize-y leading-relaxed"
                  data-testid="org-draft-brief"
                  id="org-draft-brief"
                  onChange={mark(setBrief)}
                  readOnly={!canPublish}
                  required
                  rows={14}
                  value={brief}
                />
              </Field>
              <Field label="Review date" id="org-draft-due">
                <Input
                  data-testid="org-draft-due"
                  id="org-draft-due"
                  onChange={mark(setDueValue)}
                  readOnly={!canPublish}
                  required
                  type="date"
                  value={dueValue}
                />
              </Field>
              {draft.kind === "project" ? (
                <HolderField
                  disabled={!canPublish}
                  holder={holder}
                  members={members ?? []}
                  onChange={(pubkey) => {
                    setHolder(pubkey);
                    setDirty(true);
                  }}
                  orgAgentPubkey={orgAgentPubkey}
                  profiles={profiles}
                  selfProfile={selfProfile}
                />
              ) : null}
            </>
          ) : null}
          {draft.kind === "direction" || draft.kind === "revise-direction" ? (
            lined(draft.slug) ? (
              <LinedDraftFields
                body={body}
                lineTypes={
                  draft.slug === "strategy" ? strategyTypes : undefined
                }
                onChange={(next) => {
                  setDirty(true);
                  setBody(next);
                }}
                onLineTypes={
                  draft.slug === "strategy"
                    ? (next) => {
                        setDirty(true);
                        setStrategyTypes(next);
                      }
                    : undefined
                }
                readOnly={!canPublish}
                slug={draft.slug}
              />
            ) : (
              <Field label="Text" id="org-draft-body">
                <Textarea
                  data-testid="org-draft-body"
                  id="org-draft-body"
                  onChange={mark(setBody)}
                  readOnly={!canPublish}
                  required
                  rows={6}
                  value={body}
                />
              </Field>
            )
          ) : null}
          {draft.kind === "dri" ? (
            <>
              <div className="space-y-1.5">
                <p className="text-sm font-medium">Project</p>
                <Link
                  className="inline-flex rounded-sm text-sm font-medium text-foreground underline decoration-foreground/40 underline-offset-4 hover:decoration-foreground focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring"
                  data-testid="org-draft-project"
                  onClick={() => onOpenChange(false)}
                  params={{ itemId: draft.itemId }}
                  to="/org/work/$itemId"
                >
                  {itemTitle?.trim() || "Open the project"}
                </Link>
              </div>
              <HolderField
                allowEmpty={false}
                disabled={!canPublish}
                emptyLabel="Choose a person"
                holder={holder}
                label="Holder"
                linkProfile
                members={members ?? []}
                onChange={(pubkey) => {
                  setHolder(pubkey);
                  setDirty(true);
                }}
                onOpenProfile={() => onOpenChange(false)}
                orgAgentPubkey={orgAgentPubkey}
                profiles={profiles}
                selfProfile={selfProfile}
              />
            </>
          ) : null}
          {draft.kind === "remove-project" ? (
            <p className="text-sm text-foreground">{heading}</p>
          ) : null}
          {draft.kind === "shapers-add" || draft.kind === "shapers-remove" ? (
            <p className="text-sm text-foreground">
              {personName ?? "A member"}
            </p>
          ) : null}
          {draft.kind === "shapers-agent" ? (
            <Field label="Agent pubkey" id="org-draft-agent">
              <Input
                data-testid="org-draft-agent"
                id="org-draft-agent"
                onChange={mark(setAgentPubkey)}
                placeholder="Empty uses the hosted agent"
                readOnly={!canPublish}
                value={agentPubkey}
              />
            </Field>
          ) : null}
          {draft.kind === "shapers-rules" ? (
            <>
              {RULE_KINDS.map((key) => (
                <Field
                  key={key}
                  id={`org-draft-rule-${key}`}
                  label={ruleLabel(key)}
                >
                  <Input
                    data-testid={`org-draft-rule-${key}`}
                    id={`org-draft-rule-${key}`}
                    onChange={mark((value) =>
                      setRules((current) => ({ ...current, [key]: value })),
                    )}
                    readOnly={!canPublish}
                    value={rules[key]}
                  />
                </Field>
              ))}
              <Field label="Decision window (days)" id="org-draft-decision">
                <Input
                  data-testid="org-draft-decision"
                  id="org-draft-decision"
                  inputMode="numeric"
                  onChange={mark(setDecisionDays)}
                  readOnly={!canPublish}
                  value={decisionDays}
                />
              </Field>
              <Field label="Offer window (days)" id="org-draft-offer">
                <Input
                  data-testid="org-draft-offer"
                  id="org-draft-offer"
                  inputMode="numeric"
                  onChange={mark(setOfferDays)}
                  readOnly={!canPublish}
                  value={offerDays}
                />
              </Field>
            </>
          ) : null}
          {draft.kind === "codebases"
            ? codebaseItems.map((item, index) => (
                <div className="space-y-2" key={item.key}>
                  <Field
                    id={`org-draft-codebase-name-${index}`}
                    label={item.kind === "site" ? "Landing page" : "Repository"}
                  >
                    <Input
                      id={`org-draft-codebase-name-${index}`}
                      onChange={(event) => {
                        const name = event.target.value;
                        setDirty(true);
                        setCodebaseItems((current) =>
                          current.map((row, rowIndex) =>
                            rowIndex === index ? { ...row, name } : row,
                          ),
                        );
                      }}
                      readOnly={!canPublish}
                      value={item.name}
                    />
                  </Field>
                  <Field id={`org-draft-codebase-url-${index}`} label="Link">
                    <Input
                      id={`org-draft-codebase-url-${index}`}
                      onChange={(event) => {
                        const url = event.target.value;
                        setDirty(true);
                        setCodebaseItems((current) =>
                          current.map((row, rowIndex) =>
                            rowIndex === index ? { ...row, url } : row,
                          ),
                        );
                      }}
                      readOnly={!canPublish}
                      value={item.url}
                    />
                  </Field>
                  <Field
                    id={`org-draft-codebase-about-${index}`}
                    label="What it is"
                  >
                    <Input
                      id={`org-draft-codebase-about-${index}`}
                      onChange={(event) => {
                        const about = event.target.value;
                        setDirty(true);
                        setCodebaseItems((current) =>
                          current.map((row, rowIndex) =>
                            rowIndex === index ? { ...row, about } : row,
                          ),
                        );
                      }}
                      readOnly={!canPublish}
                      value={item.about}
                    />
                  </Field>
                </div>
              ))
            : null}
          {dirty && seeded.current !== draft.messageId ? (
            <p className="text-sm text-muted-foreground">
              The agent updated this draft. Your edits are still here.
            </p>
          ) : null}
          {!canPublish ? (
            <p className="text-sm text-muted-foreground">
              A Shaper publishes this draft.
            </p>
          ) : null}
          {!room && canPublish ? (
            <p className="text-sm text-muted-foreground">
              #shapers is not open yet. Publishing still puts it on My work.
            </p>
          ) : null}
          {error ? (
            <p className="text-sm text-destructive" role="alert">
              {error}
            </p>
          ) : null}
          <DialogFooter>
            {canPublish ? (
              <Button
                data-testid="org-draft-publish"
                disabled={busy || (draft.kind === "dri" && !holder.trim())}
                type="submit"
              >
                Publish
              </Button>
            ) : null}
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function storedWhy(draft: ChatDraft): string {
  if (
    draft.kind === "shapers-add" ||
    draft.kind === "shapers-remove" ||
    draft.kind === "shapers-agent"
  ) {
    return draft.why;
  }
  return "";
}

function daysOf(secs: number | null): string {
  if (secs == null || secs <= 0) return "";
  return String(Math.round(secs / 86_400));
}

function daysToSecs(days: string): number | null {
  const value = Number(days.trim());
  if (!Number.isFinite(value) || value <= 0) return null;
  return Math.floor(value) * 86_400;
}

function ruleLabel(key: RuleKind): string {
  if (key === "dri") return "DRI rule";
  return `${key.charAt(0).toUpperCase()}${key.slice(1)} rule`;
}

type HolderOption = {
  avatarUrl: string | null;
  isAgent: boolean;
  label: string;
  pubkey: string;
};

function holderOption(
  pubkey: string,
  profiles: Record<string, UserProfileSummary> | undefined,
  selfProfile: Profile | undefined,
): HolderOption {
  const key = normalizePubkey(pubkey);
  const summary = profiles?.[key] ?? profiles?.[pubkey];
  const self =
    selfProfile && normalizePubkey(selfProfile.pubkey) === key
      ? selfProfile
      : undefined;
  const label =
    summary?.displayName?.trim() ||
    summary?.name?.trim() ||
    self?.displayName?.trim() ||
    summary?.nip05Handle?.trim() ||
    self?.nip05Handle?.trim() ||
    truncateNpub(key);
  return {
    avatarUrl: summary?.avatarUrl ?? self?.avatarUrl ?? null,
    isAgent: summary?.isAgent === true,
    label,
    pubkey: key,
  };
}

function HolderFace({ option }: { option: HolderOption }) {
  return (
    <span className="flex min-w-0 flex-1 items-center gap-2">
      <span aria-hidden="true" className="shrink-0">
        <UserAvatar
          avatarUrl={option.avatarUrl}
          displayName={option.label}
          shape={option.isAgent ? "squircle" : "circle"}
          size="sm"
        />
      </span>
      <span className="truncate">{option.label}</span>
    </span>
  );
}

function HolderField({
  allowEmpty = true,
  disabled,
  emptyLabel = "Nobody yet",
  holder,
  label,
  linkProfile = false,
  members,
  onChange,
  onOpenProfile,
  orgAgentPubkey,
  profiles,
  selfProfile,
}: {
  allowEmpty?: boolean;
  disabled: boolean;
  emptyLabel?: string;
  holder: string;
  label?: string;
  linkProfile?: boolean;
  members: readonly { pubkey: string }[];
  onChange: (pubkey: string) => void;
  onOpenProfile?: () => void;
  orgAgentPubkey: string | null;
  profiles?: Record<string, UserProfileSummary>;
  selfProfile?: Profile;
}) {
  const [open, setOpen] = React.useState(false);
  const selectedKey = holder ? normalizePubkey(holder) : "";
  const options = React.useMemo(() => {
    const agent = orgAgentPubkey ? normalizePubkey(orgAgentPubkey) : "";
    const seen = new Set<string>();
    const rows: HolderOption[] = [];
    const add = (pubkey: string) => {
      const key = normalizePubkey(pubkey);
      if (!key || key === agent || seen.has(key)) return;
      seen.add(key);
      rows.push(holderOption(pubkey, profiles, selfProfile));
    };
    for (const member of members) add(member.pubkey);
    if (holder) add(holder);
    rows.sort((left, right) =>
      left.label.localeCompare(right.label, undefined, {
        sensitivity: "base",
      }),
    );
    return rows;
  }, [holder, members, orgAgentPubkey, profiles, selfProfile]);
  const selected = options.find((option) => option.pubkey === selectedKey);
  const fieldLabel = label ?? "Directly responsible individual (DRI)";
  const menu = (
    <DropdownMenuContent
      align={linkProfile ? "end" : "start"}
      className="z-[70] max-h-64 overflow-y-auto"
      onCloseAutoFocus={(event) => event.preventDefault()}
      sideOffset={4}
      style={
        linkProfile
          ? { minWidth: "16rem" }
          : { width: "var(--radix-dropdown-menu-trigger-width)" }
      }
    >
      {allowEmpty ? (
        <DropdownMenuItem
          data-testid="org-draft-holder-none"
          onSelect={() => onChange("")}
        >
          <span className="min-w-0 flex-1 truncate text-muted-foreground">
            {emptyLabel}
          </span>
          <Check
            aria-hidden="true"
            className={cn("ml-auto", selected ? "opacity-0" : "")}
          />
        </DropdownMenuItem>
      ) : null}
      {options.map((option) => (
        <DropdownMenuItem
          data-testid={`org-draft-holder-option-${option.pubkey}`}
          key={option.pubkey}
          onSelect={() => onChange(option.pubkey)}
        >
          <HolderFace option={option} />
          <Check
            aria-hidden="true"
            className={cn(
              "ml-auto",
              option.pubkey === selectedKey ? "" : "opacity-0",
            )}
          />
        </DropdownMenuItem>
      ))}
    </DropdownMenuContent>
  );

  if (linkProfile) {
    return (
      <Field id="org-draft-holder" label={fieldLabel}>
        <div className="flex h-9 w-full items-center rounded-lg border border-input/40 bg-background">
          {selected ? (
            <Link
              aria-label={`Open ${selected.label}'s profile`}
              className="flex min-w-0 flex-1 items-center gap-2 rounded-lg px-3 text-sm hover:underline focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring"
              data-testid="org-draft-holder-profile"
              onClick={onOpenProfile}
              params={{ pubkey: selected.pubkey }}
              to="/org/profile/$pubkey"
            >
              <HolderFace option={selected} />
            </Link>
          ) : (
            <span className="min-w-0 flex-1 truncate px-3 text-sm text-muted-foreground">
              {emptyLabel}
            </span>
          )}
          <DropdownMenu modal={false} onOpenChange={setOpen} open={open}>
            <DropdownMenuTrigger asChild>
              <button
                className="flex h-9 shrink-0 items-center gap-1 border-l border-input/40 px-3 text-sm focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                data-testid="org-draft-holder"
                disabled={disabled}
                id="org-draft-holder"
                type="button"
              >
                Change
                <ChevronDown className="h-4 w-4 shrink-0 text-muted-foreground" />
              </button>
            </DropdownMenuTrigger>
            {menu}
          </DropdownMenu>
        </div>
      </Field>
    );
  }

  return (
    <Field id="org-draft-holder" label={fieldLabel}>
      <DropdownMenu modal={false} onOpenChange={setOpen} open={open}>
        <DropdownMenuTrigger asChild>
          <button
            className="flex h-9 w-full items-center gap-2 rounded-lg border border-input/40 bg-background px-3 text-left text-sm transition-colors focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
            data-testid="org-draft-holder"
            disabled={disabled}
            id="org-draft-holder"
            type="button"
          >
            {selected ? (
              <HolderFace option={selected} />
            ) : (
              <span className="min-w-0 flex-1 truncate text-muted-foreground">
                {emptyLabel}
              </span>
            )}
            <ChevronDown className="h-4 w-4 shrink-0 text-muted-foreground" />
          </button>
        </DropdownMenuTrigger>
        {menu}
      </DropdownMenu>
    </Field>
  );
}

/** One field per objective or strategy line. `body` stays newline-joined. */
function draftLines(body: string): string[] {
  return body.length > 0 ? body.split("\n") : [""];
}

function LinedDraftFields({
  body,
  lineTypes,
  onChange,
  onLineTypes,
  readOnly,
  slug,
}: {
  body: string;
  lineTypes?: readonly StrategyLineType[];
  onChange: (body: string) => void;
  onLineTypes?: (types: StrategyLineType[]) => void;
  readOnly: boolean;
  slug: DirectionSlug;
}) {
  const noun = slug === "objectives" ? "objective" : "strategy";
  const title = slug === "objectives" ? "Objective" : "Strategy";
  const lines = draftLines(body);
  const setLine = (index: number, value: string) => {
    const next = [...lines];
    if (slug === "strategy") {
      const read = readStrategyLine(value);
      next[index] = read.type ? read.text : value;
      onChange(next.join("\n"));
      if (read.type && onLineTypes) {
        const nextTypes = [...(lineTypes ?? [])];
        while (nextTypes.length < next.length) nextTypes.push("bet");
        nextTypes[index] = read.type;
        onLineTypes(nextTypes.slice(0, next.length));
      }
      return;
    }
    next[index] = value;
    onChange(next.join("\n"));
  };
  const removeLine = (index: number) => {
    const next = lines.filter((_, at) => at !== index);
    const kept = next.length > 0 ? next : [""];
    onChange(kept.join("\n"));
    if (!onLineTypes) return;
    const nextTypes = (lineTypes ?? []).filter((_, at) => at !== index);
    onLineTypes(nextTypes.length > 0 ? nextTypes : ["bet"]);
  };

  return (
    <div className="space-y-3">
      {lines.map((line, index) => {
        const id = `org-draft-line-${index}`;
        const number = index + 1;
        return (
          <Field id={id} key={id} label={`${title} ${number}`}>
            <Textarea
              data-testid={id}
              id={id}
              onChange={(event) => setLine(index, event.target.value)}
              readOnly={readOnly}
              rows={3}
              value={line}
            />
            {readOnly || lines.length < 2 ? null : (
              <button
                aria-label={`Remove ${noun} ${number}`}
                className="text-sm text-muted-foreground underline decoration-foreground/30 underline-offset-4 hover:text-foreground focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring"
                data-testid={`org-draft-line-remove-${index}`}
                onClick={() => removeLine(index)}
                type="button"
              >
                Remove
              </button>
            )}
          </Field>
        );
      })}
      {readOnly ? null : (
        <Button
          data-testid="org-draft-line-add"
          onClick={() => {
            onChange([...lines, ""].join("\n"));
            onLineTypes?.([...(lineTypes ?? []), "bet"]);
          }}
          type="button"
          variant="outline"
        >
          {noun === "objective" ? "Add an objective" : "Add a strategy"}
        </Button>
      )}
    </div>
  );
}

function Field({
  children,
  id,
  label,
}: {
  children: React.ReactNode;
  id: string;
  label: string;
}) {
  return (
    <div className="space-y-1.5">
      <label className="text-sm font-medium" htmlFor={id}>
        {label}
      </label>
      {children}
    </div>
  );
}
