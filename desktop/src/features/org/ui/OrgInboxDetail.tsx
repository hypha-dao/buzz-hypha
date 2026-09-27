import * as React from "react";

import {
  classifyEvent,
  isOrgInboxKind,
  OrgEventCard,
  toOrgEvent,
} from "@/features/org/cards";
import { useOrgAgentQuery } from "@/features/org/useOrgAgent";
import { useOrgCommandE2eBridge } from "@/features/org/useOrgCommands";
import type { InboxItem } from "@/features/home/lib/inbox";
import {
  resolveUserLabel,
  type UserProfileLookup,
} from "@/features/profile/lib/identity";
import { ChatHeader } from "@/features/chat/ui/ChatHeader";
import { TopChromeInsetHeader } from "@/shared/layout/TopChromeInsetHeader";
import { Button } from "@/shared/ui/button";
import { normalizePubkey } from "@/shared/lib/pubkey";

type OrgInboxDetailProps = {
  currentPubkey?: string;
  item: InboxItem;
  onBack?: () => void;
  profiles?: UserProfileLookup;
};

export function isOrgInboxItem(item: InboxItem): boolean {
  return isOrgInboxKind(item.item.kind);
}

/** Same card as My Work, rendered from the existing needs_action feed item. */
export function OrgInboxDetail({
  currentPubkey,
  item,
  onBack,
  profiles,
}: OrgInboxDetailProps) {
  useOrgCommandE2eBridge();
  const agent = useOrgAgentQuery().data;
  const event = toOrgEvent(item.item);
  const nameOf = React.useCallback(
    (pubkey: string) => resolveUserLabel({ pubkey, currentPubkey, profiles }),
    [currentPubkey, profiles],
  );

  const model = React.useMemo(() => {
    if (!currentPubkey) return null;
    return classifyEvent(event, {
      viewer: normalizePubkey(currentPubkey),
      agentPubkey: agent?.pubkey ?? null,
      // The feed already decided this needs the viewer (R-13 / mock seed).
      isShaper: true,
      nameOf,
    });
  }, [agent?.pubkey, currentPubkey, event, nameOf]);

  return (
    <div className="relative flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
      <TopChromeInsetHeader data-tauri-drag-region flush>
        {onBack ? (
          <Button onClick={onBack} size="sm" type="button" variant="ghost">
            Back
          </Button>
        ) : null}
        <ChatHeader mode="org" title="Needs your answer" />
      </TopChromeInsetHeader>
      <div className="overflow-auto p-4" data-testid="org-inbox-card">
        {model ? (
          <OrgEventCard model={model} />
        ) : (
          <p className="text-sm text-muted-foreground">Nothing needs you.</p>
        )}
      </div>
    </div>
  );
}
