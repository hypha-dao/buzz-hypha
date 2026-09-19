import * as React from "react";

import {
  classifyContextFromEvents,
  classifyMyWork,
} from "@/features/org/cards";
import { useMyWorkEvents } from "@/features/org/hooks";
import { useOrgCommandE2eBridge } from "@/features/org/useOrgCommands";
import { ChatHeader } from "@/features/chat/ui/ChatHeader";
import { resolveUserLabel } from "@/features/profile/lib/identity";
import { useUsersBatchQuery } from "@/features/profile/hooks";
import { useIdentityQuery } from "@/shared/api/hooks";
import { TopChromeInsetHeader } from "@/shared/layout/TopChromeInsetHeader";

import { MyWorkBoard } from "./MyWorkBoard";

function collectPubkeys(
  events: readonly { pubkey: string; tags: string[][]; content: string }[],
): string[] {
  const pubkeys = new Set<string>();
  for (const event of events) {
    pubkeys.add(event.pubkey);
    for (const tag of event.tags) {
      if (tag[0] === "p" && tag[1]) pubkeys.add(tag[1]);
    }
    try {
      const content = JSON.parse(event.content) as Record<string, unknown>;
      for (const key of [
        "suggested_dri",
        "suggested_holder",
        "suggested",
        "dri",
      ]) {
        const value = content[key];
        if (typeof value === "string") pubkeys.add(value);
      }
    } catch {
      // content is not the structured payload — skip
    }
  }
  return [...pubkeys];
}

/** My Work — three columns and the card set (D-2). */
export function MyWorkScreen() {
  const { events } = useMyWorkEvents();
  useOrgCommandE2eBridge();
  const viewer = useIdentityQuery().data?.pubkey ?? null;
  const pubkeys = React.useMemo(() => collectPubkeys(events), [events]);
  const profiles = useUsersBatchQuery(pubkeys).data?.profiles;

  const columns = React.useMemo(() => {
    if (!viewer) {
      return { needs_answer: [], you_hold: [], you_offered: [] };
    }
    const nameOf = (pubkey: string) =>
      resolveUserLabel({ pubkey, currentPubkey: viewer, profiles });
    return classifyMyWork(
      events,
      classifyContextFromEvents(events, viewer, nameOf),
    );
  }, [events, profiles, viewer]);

  return (
    <div
      className="relative flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden"
      data-testid="org-my-work"
    >
      <TopChromeInsetHeader data-tauri-drag-region flush>
        <ChatHeader mode="org" title="My Work" />
      </TopChromeInsetHeader>
      <MyWorkBoard columns={columns} />
    </div>
  );
}
