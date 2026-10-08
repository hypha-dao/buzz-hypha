import * as React from "react";
import { Link, useNavigate } from "@tanstack/react-router";

import { useAppNavigation } from "@/app/navigation/useAppNavigation";
import { useUsersBatchQuery } from "@/features/profile/hooks";
import { resolveUserLabel } from "@/features/profile/lib/identity";
import { useIdentityQuery } from "@/shared/api/hooks";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/shared/ui/alert-dialog";
import { Button } from "@/shared/ui/button";

import { draftPageModel, type DraftFact } from "../draftPage";
import { useMyDrafts, useWorkItemTitles } from "../hooks";
import { useOverviewEvents } from "../hooks/useOverviewEvents";
import {
  draftItemId,
  draftPersonPubkey,
  draftPublishableBy,
  type OwnDraft,
} from "../myDrafts";
import { formatWorkDate } from "../work/model";
import { motionDelay } from "./OrgMotionFrame";
import { OrgDoorScreen } from "./OrgDoorScreen";
import { overviewStampDelayMs } from "./overview/overviewMotion";
import { parseShapersState } from "./overview/parseOverview";
import { ProposalArticle, ProposalFact } from "./ProposalArticle";
import { ProposalDraftDialog } from "./ProposalDraftDialog";
import { HolderName } from "./work/HolderName";

/** One of your drafts, laid out as the page it becomes once published. */
export function DraftScreen({ messageId }: { messageId: string }) {
  const navigate = useNavigate();
  const { goChannel } = useAppNavigation();
  const viewer = useIdentityQuery().data?.pubkey ?? null;
  const { drafts, isLoading, markDeleted, markPublished } = useMyDrafts();
  const overview = useOverviewEvents();
  const [leaving, setLeaving] = React.useState(false);
  const [publishing, setPublishing] = React.useState(false);
  const [confirmingDelete, setConfirmingDelete] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const lastSeen = React.useRef<OwnDraft | null>(null);
  const found =
    drafts.find((entry) => entry.draft.messageId === messageId) ?? null;
  if (found) lastSeen.current = found;
  const entry = found ?? (leaving ? lastSeen.current : null);
  const draft = entry?.draft ?? null;

  const itemId = draft ? draftItemId(draft) : null;
  const itemTitles = useWorkItemTitles(itemId ? [itemId] : []);
  const itemTitle = itemId ? (itemTitles.get(itemId) ?? null) : null;
  const personPubkey = draft ? draftPersonPubkey(draft) : null;
  const profiles = useUsersBatchQuery(personPubkey ? [personPubkey] : []).data
    ?.profiles;
  const personName = personPubkey
    ? resolveUserLabel({
        pubkey: personPubkey,
        currentPubkey: viewer ?? undefined,
        profiles,
      })
    : null;
  const shapers = parseShapersState(overview.events)?.shapers ?? null;
  const canPublish = draft ? draftPublishableBy(draft, viewer, shapers) : false;

  const leave = () => {
    setLeaving(true);
    void navigate({ to: "/org/my-drafts" });
  };

  if (!draft) {
    return (
      <OrgDoorScreen testId="org-draft" title="Draft">
        {isLoading ? null : (
          <div
            className="flex flex-1 flex-col items-center justify-center gap-3 px-6 py-16 text-center"
            data-testid="org-draft-gone"
          >
            <p className="text-sm text-muted-foreground">
              This draft was published or deleted.
            </p>
            <Link
              className="rounded-sm text-sm font-medium text-foreground underline decoration-foreground/40 underline-offset-4 hover:decoration-foreground focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring"
              to="/org/my-drafts"
            >
              Back to My drafts
            </Link>
          </div>
        )}
      </OrgDoorScreen>
    );
  }

  const page = draftPageModel(draft, itemTitle);

  return (
    <OrgDoorScreen testId="org-draft" title="Draft">
      <div className="min-h-0 flex-1 overflow-y-auto">
        <ProposalArticle
          brief={page.brief}
          briefHeading={page.briefHeading}
          crumb={page.crumb}
          eyebrow={page.eyebrow}
          facts={
            page.facts.length > 0
              ? page.facts.map((fact, index) => (
                  <ProposalFact
                    enterIndex={index + 2}
                    key={fact.label}
                    label={fact.label}
                  >
                    <FactValue fact={fact} />
                  </ProposalFact>
                ))
              : null
          }
          lines={page.lines}
          parentTo="/org/my-drafts"
          status="Draft"
          testId="org-draft-page"
          title={page.title}
        >
          <p
            className="text-sm text-muted-foreground"
            data-testid="org-draft-page-note"
          >
            {canPublish
              ? "Nothing changes until you publish it. Publishing puts it on My Work for every Shaper."
              : "Nothing changes until a Shaper publishes it."}
          </p>
          <div
            className="org-dir-stamp flex flex-wrap gap-2"
            style={motionDelay(overviewStampDelayMs(0, 2))}
          >
            {canPublish ? (
              <Button
                data-testid="org-draft-page-publish"
                disabled={leaving}
                onClick={() => {
                  setError(null);
                  setPublishing(true);
                }}
                type="button"
              >
                Edit and publish
              </Button>
            ) : null}
            {entry ? (
              <Button
                data-testid="org-draft-page-chat"
                onClick={() => {
                  void goChannel(entry.channelId);
                }}
                type="button"
                variant="outline"
              >
                Open chat
              </Button>
            ) : null}
            <Button
              data-testid="org-draft-page-delete"
              disabled={leaving}
              onClick={() => {
                setError(null);
                setConfirmingDelete(true);
              }}
              type="button"
              variant="ghost"
            >
              Delete
            </Button>
          </div>
          {error ? (
            <p
              className="text-sm text-destructive"
              data-testid="org-draft-page-error"
              role="alert"
            >
              {error}
            </p>
          ) : null}
        </ProposalArticle>
      </div>
      <ProposalDraftDialog
        canPublish={canPublish}
        draft={publishing ? draft : null}
        itemTitle={itemTitle}
        onOpenChange={(open) => {
          if (!open) setPublishing(false);
        }}
        onPublished={(publishedId) => {
          if (markPublished(publishedId)) {
            leave();
            return;
          }
          setError(
            "Published. This device could not record it, so the draft is still listed. Delete it to clear it.",
          );
        }}
        personName={personName}
      />
      <AlertDialog onOpenChange={setConfirmingDelete} open={confirmingDelete}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete this draft?</AlertDialogTitle>
            <AlertDialogDescription>
              It leaves your My Work, and its card leaves your chat. The agent's
              message stays in the chat.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel asChild>
              <Button type="button" variant="outline">
                Cancel
              </Button>
            </AlertDialogCancel>
            <AlertDialogAction asChild>
              <Button
                data-testid="org-draft-page-delete-confirm"
                onClick={() => {
                  if (markDeleted(draft.messageId)) {
                    leave();
                    return;
                  }
                  setError("Could not delete the draft. Try again.");
                }}
                type="button"
                variant="destructive"
              >
                Delete draft
              </Button>
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </OrgDoorScreen>
  );
}

function FactValue({ fact }: { fact: DraftFact }) {
  if (fact.kind === "person") {
    return <HolderName empty={fact.empty} pubkey={fact.pubkey} />;
  }
  if (fact.at === null) return <>{fact.empty}</>;
  return (
    <time dateTime={new Date(fact.at * 1000).toISOString()}>
      {formatWorkDate(fact.at)}
    </time>
  );
}
