import * as React from "react";

import { useEphemeralChannelDisplay } from "@/features/channels/useEphemeralChannelDisplay";
import { usePresenceQuery } from "@/features/presence/hooks";
import { useUsersBatchQuery } from "@/features/profile/hooks";
import {
  isOrgAgentDm,
  ORG_AGENT_LABEL,
  orgAgentDmPresence,
} from "@/features/org/orgAgent";
import { useOrgAgentPubkey } from "@/features/org/useOrgAgent";
import { resolveUserLabel } from "@/features/profile/lib/identity";
import { resolveChannelDisplayLabel } from "@/features/sidebar/lib/channelLabels";
import type { Channel, PresenceStatus } from "@/shared/api/types";
import { normalizePubkey } from "@/shared/lib/pubkey";

export type ActiveDmHeaderParticipant = {
  pubkey: string;
  displayName: string;
  avatarUrl: string | null;
  isAgent?: boolean;
};

export function useActiveChannelHeader(
  activeChannel: Channel | null,
  currentPubkey?: string,
) {
  const activeDmParticipants = React.useMemo(() => {
    if (activeChannel?.channelType !== "dm") {
      return [];
    }

    const normalizedCurrentPubkey = currentPubkey
      ? normalizePubkey(currentPubkey)
      : null;

    return activeChannel.participantPubkeys
      .map((pubkey, index) => ({
        fallbackName: activeChannel.participants[index] ?? null,
        pubkey,
      }))
      .filter(
        (participant) =>
          normalizePubkey(participant.pubkey) !== normalizedCurrentPubkey,
      );
  }, [activeChannel, currentPubkey]);
  const orgAgentPubkey = useOrgAgentPubkey();
  const isAgentDm =
    activeChannel !== null &&
    isOrgAgentDm(activeChannel, orgAgentPubkey, currentPubkey ?? null);
  const activeDmParticipantPubkeys = React.useMemo(() => {
    if (isAgentDm && orgAgentPubkey && activeDmParticipants.length === 0) {
      return [orgAgentPubkey];
    }
    return activeDmParticipants.map((participant) => participant.pubkey);
  }, [activeDmParticipants, isAgentDm, orgAgentPubkey]);
  const activeDmPresenceQuery = usePresenceQuery(activeDmParticipantPubkeys, {
    enabled: activeDmParticipantPubkeys.length > 0,
  });
  const activeDmProfilesQuery = useUsersBatchQuery(activeDmParticipantPubkeys, {
    enabled: activeDmParticipantPubkeys.length > 0,
  });
  const activeChannelEphemeralDisplay =
    useEphemeralChannelDisplay(activeChannel);
  const relayDmPresence: PresenceStatus | null =
    activeDmParticipantPubkeys.length > 0
      ? (activeDmPresenceQuery.data?.[
          activeDmParticipantPubkeys[0]?.toLowerCase()
        ] ?? null)
      : null;
  const activeDmPresenceStatus: PresenceStatus | null = activeChannel
    ? orgAgentDmPresence(
        activeChannel,
        orgAgentPubkey,
        currentPubkey ?? null,
        relayDmPresence,
      )
    : null;
  const activeDmAvatarUrl =
    activeDmParticipantPubkeys.length > 0
      ? (activeDmProfilesQuery.data?.profiles?.[
          normalizePubkey(activeDmParticipantPubkeys[0] ?? "")
        ]?.avatarUrl ?? null)
      : null;
  const activeDmHeaderParticipants = React.useMemo(() => {
    if (isAgentDm && orgAgentPubkey) {
      const profile =
        activeDmProfilesQuery.data?.profiles?.[
          normalizePubkey(orgAgentPubkey)
        ] ?? null;
      const displayName = ORG_AGENT_LABEL;
      return [
        {
          pubkey: orgAgentPubkey,
          displayName,
          avatarUrl: profile?.avatarUrl ?? null,
          isAgent: true,
        },
      ];
    }
    return activeDmParticipants.map((participant) => {
      const profile =
        activeDmProfilesQuery.data?.profiles?.[
          normalizePubkey(participant.pubkey)
        ] ?? null;

      return {
        pubkey: participant.pubkey,
        displayName: resolveUserLabel({
          currentPubkey,
          fallbackName: participant.fallbackName,
          profiles: activeDmProfilesQuery.data?.profiles,
          pubkey: participant.pubkey,
        }),
        avatarUrl: profile?.avatarUrl ?? null,
        ...(profile?.isAgent === true ? { isAgent: true } : {}),
      };
    });
  }, [
    activeDmParticipants,
    activeDmProfilesQuery.data?.profiles,
    currentPubkey,
    isAgentDm,
    orgAgentPubkey,
  ]);

  return {
    activeChannelTitle: activeChannel
      ? isAgentDm
        ? ORG_AGENT_LABEL
        : resolveChannelDisplayLabel(
            activeChannel,
            currentPubkey,
            activeDmProfilesQuery.data?.profiles,
            orgAgentPubkey,
          )
      : "Channels",
    activeDmAvatarUrl,
    activeDmHeaderParticipants,
    activeDmPresenceStatus,
    activeChannelEphemeralDisplay,
  };
}
