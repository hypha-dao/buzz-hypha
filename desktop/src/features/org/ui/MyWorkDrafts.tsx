import { Link } from "@tanstack/react-router";

import { draftPageModel } from "@/features/org/draftPage";
import { draftItemId, type OwnDraft } from "@/features/org/myDrafts";
import { formatWorkDate } from "@/features/org/work/model";

const MAX_CARD_LINES = 3;

/** Drafts you asked the agent for. Each opens its draft page. */
export function MyWorkDrafts({
  drafts,
  itemTitles,
}: {
  drafts: readonly OwnDraft[];
  itemTitles: ReadonlyMap<string, string>;
}) {
  return (
    <section aria-label="Your drafts" data-testid="org-my-drafts-list">
      <ul className="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-4">
        {drafts.map(({ draft }) => {
          const itemId = draftItemId(draft);
          const page = draftPageModel(
            draft,
            itemId ? itemTitles.get(itemId) : null,
          );
          const shown = page.lines.slice(0, MAX_CARD_LINES);
          const more = page.lines.length - shown.length;
          const dueAt =
            draft.kind === "project" || draft.kind === "revise-project"
              ? draft.dueAt
              : null;
          return (
            <li className="min-w-0" key={draft.messageId}>
              <Link
                className="block h-full rounded-xl border border-dashed border-border bg-card p-5 text-left shadow-xs hover:bg-muted/40 focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring"
                data-testid={`org-my-draft-${draft.messageId}`}
                params={{ messageId: draft.messageId }}
                to="/org/draft/$messageId"
              >
                <span className="block text-2xs font-medium uppercase tracking-wider text-muted-foreground">
                  {page.eyebrow}
                </span>
                {shown.length > 0 ? (
                  <span className="mt-3 block space-y-1.5 text-sm leading-relaxed">
                    {shown.map((line) => (
                      <span
                        className="block rounded-md border border-border/60 bg-background/40 px-3 py-1.5"
                        key={line.id}
                      >
                        {line.text}
                      </span>
                    ))}
                    {more > 0 ? (
                      <span className="block text-xs text-muted-foreground">
                        {more === 1 ? "1 more" : `${more} more`}
                      </span>
                    ) : null}
                  </span>
                ) : (
                  <span className="mt-3 line-clamp-3 block text-message font-semibold leading-snug">
                    {page.title}
                  </span>
                )}
                <span className="mt-3 block text-xs text-muted-foreground">
                  {dueAt !== null ? (
                    <>
                      Review{" "}
                      <time dateTime={new Date(dueAt * 1000).toISOString()}>
                        {formatWorkDate(dueAt)}
                      </time>
                      {" · "}
                    </>
                  ) : null}
                  Drafted{" "}
                  <time
                    dateTime={new Date(draft.createdAt * 1000).toISOString()}
                  >
                    {formatWorkDate(draft.createdAt)}
                  </time>
                </span>
              </Link>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
