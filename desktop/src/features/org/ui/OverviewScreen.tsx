import * as React from "react";

import { useAppNavigation } from "@/app/navigation/useAppNavigation";
import { ChatHeader } from "@/features/chat/ui/ChatHeader";
import { useIdentityQuery } from "@/shared/api/hooks";
import { useUsersBatchQuery } from "@/features/profile/hooks";
import { TopChromeInsetHeader } from "@/shared/layout/TopChromeInsetHeader";

import {
  useLiveDoorEvents,
  useOverviewEvents,
  viewerIsShaper,
} from "@/features/org/hooks";
import { useOrgCommandE2eBridge } from "@/features/org/useOrgCommands";

import { DirectionCards } from "./overview/DirectionCards";
import { tallyFilters } from "./overview/overviewExtraFilters";
import {
  collectOverviewPubkeys,
  directionSlots,
  parseShapersState,
  parseTallyNote,
  projectHolds,
} from "./overview/parseOverview";
import { ShapersCard } from "./overview/ShapersCard";
import { TallyCard } from "./overview/TallyCard";
import { WhoHoldsWhat } from "./overview/WhoHoldsWhat";

/** Overview door — Phase 0 § Overview; Prototype map § Overview. */
export function OverviewScreen() {
  useOrgCommandE2eBridge();
  const { events } = useOverviewEvents();
  const pubkey = useIdentityQuery().data?.pubkey ?? null;
  const isShaper = pubkey ? viewerIsShaper(events, pubkey) : false;
  const tallyReq = useLiveDoorEvents(isShaper ? tallyFilters() : []);
  const navigation = useAppNavigation();

  const slots = React.useMemo(() => directionSlots(events), [events]);
  const shapers = React.useMemo(() => parseShapersState(events), [events]);
  const holds = React.useMemo(() => projectHolds(events), [events]);
  const tally = React.useMemo(
    () => parseTallyNote(tallyReq.events),
    [tallyReq.events],
  );
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
      <div className="min-h-0 flex-1 overflow-y-auto px-6 py-4">
        <div className="mx-auto flex max-w-4xl flex-col gap-6">
          <DirectionCards
            isShaper={isShaper}
            onOpenDirection={(slug) => {
              void navigation.goOrgDirection(slug);
            }}
            profiles={profiles}
            slots={slots}
          />
          <div className="grid gap-3 md:grid-cols-2">
            <ShapersCard
              isShaper={isShaper}
              profiles={profiles}
              shapers={shapers}
            />
            <WhoHoldsWhat
              holds={holds}
              onOpenItem={(itemId) => {
                void navigation.goOrgWorkItem(itemId);
              }}
              profiles={profiles}
            />
          </div>
          {isShaper ? <TallyCard tally={tally} /> : null}
        </div>
      </div>
    </div>
  );
}
