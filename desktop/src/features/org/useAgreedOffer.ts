import * as React from "react";

import { useChannelListRefresh } from "@/features/channels/useChannelListRefresh";
import { useIdentityQuery } from "@/shared/api/hooks";
import type { RelayEvent } from "@/shared/api/types";

import { agreedOfferItemIds, claimAgreedOffer } from "./agreedOffer";
import { toOrgEvent } from "./cards";
import { afterOrgCommandPublished } from "./channelListRefresh";
import { buildIoAccept, publishOrgCommand } from "./commands";

/**
 * Record `io_accept` for a project the viewer already agreed to.
 * The card stays on Needs your answer until that accept lands, so a
 * rejected publish still leaves the Accept button.
 */
export function useAcceptAgreedProjectOffers(
  events: readonly RelayEvent[],
): void {
  const viewer = useIdentityQuery().data?.pubkey ?? null;
  const refreshChannelList = useChannelListRefresh();
  const ids = React.useMemo(() => {
    if (!viewer) return [];
    return agreedOfferItemIds(events.map(toOrgEvent), viewer);
  }, [events, viewer]);

  React.useEffect(() => {
    for (const itemId of ids) {
      if (!claimAgreedOffer(itemId)) continue;
      const command = buildIoAccept(itemId);
      void publishOrgCommand(command)
        .then(() => {
          afterOrgCommandPublished(command.kind, refreshChannelList);
        })
        .catch((error: unknown) => {
          console.error(
            "Could not record the project you already agreed to",
            error,
          );
        });
    }
  }, [ids, refreshChannelList]);
}
