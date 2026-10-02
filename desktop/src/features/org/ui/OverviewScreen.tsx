import * as React from "react";

import { useAppNavigation } from "@/app/navigation/useAppNavigation";
import { ChatHeader } from "@/features/chat/ui/ChatHeader";
import { useIdentityQuery } from "@/shared/api/hooks";
import { useUsersBatchQuery } from "@/features/profile/hooks";
import { normalizePubkey } from "@/shared/lib/pubkey";
import { TopChromeInsetHeader } from "@/shared/layout/TopChromeInsetHeader";

import { useOverviewEvents } from "@/features/org/hooks";
import { useOrgCommandE2eBridge } from "@/features/org/useOrgCommands";

import { DirectionCards } from "./overview/DirectionCards";
import {
  collectOverviewPubkeys,
  directionSlots,
  parseShapersState,
  projectHolds,
} from "./overview/parseOverview";
import { ShapersCard } from "./overview/ShapersCard";
import { WhoHoldsWhat } from "./overview/WhoHoldsWhat";

/** Overview door — Phase 0 § Overview; Prototype map § Overview. */
export function OverviewScreen() {
  useOrgCommandE2eBridge();
  const { events } = useOverviewEvents();
  const pubkey = useIdentityQuery().data?.pubkey ?? null;
  const navigation = useAppNavigation();

  const slots = React.useMemo(() => directionSlots(events), [events]);
  const shapers = React.useMemo(() => parseShapersState(events), [events]);
  const holds = React.useMemo(() => projectHolds(events), [events]);
  const pubkeys = React.useMemo(
    () => collectOverviewPubkeys(slots, shapers, holds),
    [holds, shapers, slots],
  );
  const profiles = useUsersBatchQuery(pubkeys).data?.profiles ?? {};

  return (
    <div
      className="relative flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden"
      data-testid="org-overview"
    >
      <TopChromeInsetHeader data-tauri-drag-region flush>
        <ChatHeader mode="org" title="Overview" />
      </TopChromeInsetHeader>
      <div className="min-h-0 flex-1 overflow-y-auto px-6 py-6">
        <div className="mx-auto grid w-full max-w-6xl grid-cols-1 gap-4 md:grid-cols-2">
          <DirectionCards
            className="md:col-span-2"
            onOpenDirection={(slug) => {
              void navigation.goOrgDirection(slug);
            }}
            slots={slots}
          />
          <ShapersCard
            enterIndex={slots.length}
            onOpenProfile={(member) => {
              if (
                pubkey &&
                normalizePubkey(member) === normalizePubkey(pubkey)
              ) {
                void navigation.goOrgProfile();
                return;
              }
              void navigation.goOrgProfile(normalizePubkey(member));
            }}
            profiles={profiles}
            shapers={shapers}
          />
          <WhoHoldsWhat
            enterIndex={slots.length + 1}
            holds={holds}
            onOpenItem={(itemId) => {
              void navigation.goOrgWorkItem(itemId);
            }}
            profiles={profiles}
          />
        </div>
      </div>
    </div>
  );
}
