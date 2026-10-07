import * as React from "react";

import { useAppNavigation } from "@/app/navigation/useAppNavigation";
import { ChatHeader } from "@/features/chat/ui/ChatHeader";
import { useIdentityQuery } from "@/shared/api/hooks";
import { useUsersBatchQuery } from "@/features/profile/hooks";
import { KIND_IO_DIRECTION } from "@/shared/constants/kinds";
import { cn } from "@/shared/lib/cn";
import { normalizePubkey } from "@/shared/lib/pubkey";
import { TopChromeInsetHeader } from "@/shared/layout/TopChromeInsetHeader";
import { Card } from "@/shared/ui/card";
import { Skeleton } from "@/shared/ui/skeleton";

import { useOverviewEvents } from "@/features/org/hooks";
import { useOrgCommandE2eBridge } from "@/features/org/useOrgCommands";

import { CodebasesCard } from "./overview/CodebasesCard";
import { DirectionCards } from "./overview/DirectionCards";
import {
  collectOverviewPubkeys,
  DIRECTION_SLUGS,
  directionSlots,
  parseCodebases,
  parseShapersState,
  projectHolds,
} from "./overview/parseOverview";
import { ShapersCard } from "./overview/ShapersCard";
import { WhoHoldsWhat } from "./overview/WhoHoldsWhat";

/** Overview door — Phase 0 § Overview; Prototype map § Overview. */
export function OverviewScreen() {
  useOrgCommandE2eBridge();
  const { events, isLoading } = useOverviewEvents();
  const pubkey = useIdentityQuery().data?.pubkey ?? null;
  const navigation = useAppNavigation();

  const slots = React.useMemo(() => directionSlots(events), [events]);
  const codebases = React.useMemo(() => parseCodebases(events), [events]);
  const shapers = React.useMemo(() => parseShapersState(events), [events]);
  const holds = React.useMemo(() => projectHolds(events), [events]);
  const pubkeys = React.useMemo(
    () => collectOverviewPubkeys(slots, shapers, holds),
    [holds, shapers, slots],
  );
  const profiles = useUsersBatchQuery(pubkeys).data?.profiles ?? {};
  const waitingForDirection =
    isLoading && !events.some((event) => event.kind === KIND_IO_DIRECTION);

  return (
    <div
      aria-busy={waitingForDirection}
      className="relative flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden"
      data-testid="org-overview"
    >
      <TopChromeInsetHeader data-tauri-drag-region flush>
        <ChatHeader mode="org" title="Overview" />
      </TopChromeInsetHeader>
      <div className="min-h-0 flex-1 overflow-y-auto px-6 py-6">
        {waitingForDirection ? (
          <OverviewLoading />
        ) : (
          <div className="mx-auto grid w-full max-w-6xl grid-cols-1 gap-4 md:grid-cols-2">
            <DirectionCards
              className="md:col-span-2"
              onOpenDirection={(slug) => {
                void navigation.goOrgDirection(slug);
              }}
              slots={slots}
            />
            <CodebasesCard enterIndex={slots.length} items={codebases} />
            <ShapersCard
              enterIndex={slots.length + 1}
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
              enterIndex={slots.length + 2}
              holds={holds}
              onOpenItem={(itemId) => {
                void navigation.goOrgWorkItem(itemId);
              }}
              profiles={profiles}
            />
          </div>
        )}
      </div>
    </div>
  );
}

function OverviewLoading() {
  return (
    <div
      aria-live="polite"
      className="mx-auto grid w-full max-w-6xl grid-cols-1 gap-4 md:grid-cols-2"
      data-testid="org-overview-loading"
      role="status"
    >
      <span className="sr-only">Loading overview</span>
      <div className="flex flex-col gap-4 md:col-span-2">
        <h2 className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
          Direction
        </h2>
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
          {DIRECTION_SLUGS.map((slug) => (
            <OverviewLoadingCard key={slug} wide={slug === "situation"} />
          ))}
        </div>
      </div>
      <OverviewLoadingCard />
      <OverviewLoadingCard />
    </div>
  );
}

function OverviewLoadingCard({ wide = false }: { wide?: boolean }) {
  return (
    <Card
      aria-hidden="true"
      className={cn(
        "flex min-h-48 flex-col rounded-2xl p-6",
        wide && "md:col-span-2",
      )}
    >
      <Skeleton className="h-5 w-2/5" />
      <Skeleton className="mt-4 h-4 w-full" />
      <Skeleton className="mt-2 h-4 w-11/12" />
      <Skeleton className="mt-2 h-4 w-4/5" />
      <Skeleton className="mt-auto h-9 w-28 rounded-md" />
    </Card>
  );
}
