import * as React from "react";
import { useQueryClient } from "@tanstack/react-query";

import { channelsQueryKey } from "@/features/channels/hooks";
import {
  CHANNEL_LIST_REFRESH_RETRY_MS,
  refreshChannelsWhenIdle,
} from "@/features/channels/refreshChannelsWhenIdle";
import {
  createTrailingDebounce,
  type TrailingDebounce,
} from "@/shared/lib/trailingDebounce";

/**
 * Refetch the sidebar channel list once `get_channels` is idle.
 *
 * A membership notice (or the accept that caused it) often lands while a
 * poll is already in flight. Invalidating into that poll keeps the pre-join
 * list and the new project room never appears. Re-arm until the query is
 * quiet, then refetch.
 */
export function useChannelListRefresh(): () => void {
  const queryClient = useQueryClient();
  const retryRef = React.useRef<TrailingDebounce | null>(null);
  if (retryRef.current === null) {
    retryRef.current = createTrailingDebounce(() => {
      refreshChannelsWhenIdle({
        isFetching: () =>
          queryClient.isFetching({ queryKey: channelsQueryKey }),
        invalidate: () => {
          void queryClient.invalidateQueries({ queryKey: channelsQueryKey });
        },
        reArm: () => retryRef.current?.trigger(),
      });
    }, CHANNEL_LIST_REFRESH_RETRY_MS);
  }

  React.useEffect(() => () => retryRef.current?.cancel(), []);

  return React.useCallback(() => {
    refreshChannelsWhenIdle({
      isFetching: () => queryClient.isFetching({ queryKey: channelsQueryKey }),
      invalidate: () => {
        void queryClient.invalidateQueries({ queryKey: channelsQueryKey });
      },
      reArm: () => retryRef.current?.trigger(),
    });
  }, [queryClient]);
}
