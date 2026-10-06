import * as React from "react";

import { useMyDrafts, useWorkItemTitles } from "@/features/org/hooks";
import { draftItemId } from "@/features/org/myDrafts";
import { ChatHeader } from "@/features/chat/ui/ChatHeader";
import { TopChromeInsetHeader } from "@/shared/layout/TopChromeInsetHeader";

import { MyWorkDrafts } from "./MyWorkDrafts";

/** Drafts you asked the agent for, on their own door. */
export function MyDraftsScreen() {
  const { drafts } = useMyDrafts();
  const draftItemIds = React.useMemo(
    () => drafts.flatMap(({ draft }) => draftItemId(draft) ?? []),
    [drafts],
  );
  const draftItemTitles = useWorkItemTitles(draftItemIds);

  return (
    <div
      className="relative flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden"
      data-testid="org-my-drafts"
    >
      <TopChromeInsetHeader data-tauri-drag-region flush>
        <ChatHeader mode="org" title="My drafts" />
      </TopChromeInsetHeader>
      {drafts.length === 0 ? (
        <p
          className="flex flex-1 items-center justify-center px-6 py-16 text-center text-sm text-muted-foreground"
          data-testid="org-my-drafts-empty"
        >
          Nothing here.
        </p>
      ) : (
        <div className="min-h-0 flex-1 overflow-auto p-4">
          <MyWorkDrafts drafts={drafts} itemTitles={draftItemTitles} />
        </div>
      )}
    </div>
  );
}
