import * as React from "react";

import {
  isOrgAgentDm,
  orgAgentDmPresence,
  orgAgentSidebarFace,
} from "@/features/org/orgAgent";
import { usePresenceQuery } from "@/features/presence/hooks";
import { resolveUserLabel } from "@/features/profile/lib/identity";
import { useUsersBatchQuery } from "@/features/profile/hooks";
import { resolveChannelDisplayLabel } from "@/features/sidebar/lib/channelLabels";
import type { SidebarDmParticipant } from "@/features/sidebar/ui/SidebarSection";
import type { Channel, PresenceStatus } from "@/shared/api/types";

export function useDmSidebarMetadata({
  currentPubkey,
  directMessages,
  fallbackDisplayName,
  orgAgentPubkey,
  profileDisplayName,
  enabled = true,
}: {
  currentPubkey?: string;
  directMessages: Channel[];
  fallbackDisplayName?: string;
  orgAgentPubkey?: string | null;
  profileDisplayName?: string | null;
  enabled?: boolean;
}) {
  const selfDmLabels = React.useMemo(
    () =>
      new Set(
        [profileDisplayName, fallbackDisplayName]
          .map((value) => value?.trim().toLowerCase())
          .filter((value): value is string => Boolean(value)),
      ),
    [fallbackDisplayName, profileDisplayName],
  );
  const dmParticipantPubkeys = React.useMemo(() => {
    const pubkeys = directMessages.flatMap((channel) =>
      channel.participantPubkeys.filter((pubkey, index) => {
        const normalizedPubkey = pubkey.toLowerCase();
        if (normalizedPubkey === currentPubkey?.toLowerCase()) {
          return false;
        }

        const participantLabel =
          channel.participants[index]?.trim().toLowerCase() ?? null;
        return !participantLabel || !selfDmLabels.has(participantLabel);
      }),
    );
    if (
      orgAgentPubkey &&
      directMessages.some((channel) =>
        isOrgAgentDm(channel, orgAgentPubkey, currentPubkey ?? null),
      ) &&
      !pubkeys.some(
        (pubkey) => pubkey.toLowerCase() === orgAgentPubkey.toLowerCase(),
      )
    ) {
      pubkeys.push(orgAgentPubkey);
    }
    return pubkeys;
  }, [currentPubkey, directMessages, orgAgentPubkey, selfDmLabels]);
  const dmPresenceQuery = usePresenceQuery(dmParticipantPubkeys, {
    enabled: enabled && directMessages.length > 0,
  });
  const dmProfilesQuery = useUsersBatchQuery(dmParticipantPubkeys, {
    enabled: enabled && directMessages.length > 0,
  });
  const dmProfiles = dmProfilesQuery.data?.profiles;
  const dmPresenceByChannelId = React.useMemo(
    () =>
      Object.fromEntries(
        directMessages.map((channel) => {
          const otherParticipantPubkey = channel.participantPubkeys.find(
            (pubkey, index) => {
              const normalizedPubkey = pubkey.toLowerCase();
              if (normalizedPubkey === currentPubkey?.toLowerCase()) {
                return false;
              }

              const participantLabel =
                channel.participants[index]?.trim().toLowerCase() ?? null;
              return !participantLabel || !selfDmLabels.has(participantLabel);
            },
          );

          const relayStatus: PresenceStatus | null = otherParticipantPubkey
            ? (dmPresenceQuery.data?.[otherParticipantPubkey.toLowerCase()] ??
              "offline")
            : "offline";
          return [
            channel.id,
            orgAgentDmPresence(
              channel,
              orgAgentPubkey ?? null,
              currentPubkey ?? null,
              relayStatus,
            ) ?? "offline",
          ];
        }),
      ) satisfies Record<string, PresenceStatus>,
    [
      currentPubkey,
      directMessages,
      dmPresenceQuery.data,
      orgAgentPubkey,
      selfDmLabels,
    ],
  );
  const dmChannelLabels = React.useMemo(
    () =>
      Object.fromEntries(
        directMessages.map((channel) => [
          channel.id,
          resolveChannelDisplayLabel(
            channel,
            currentPubkey,
            dmProfilesQuery.data?.profiles,
            orgAgentPubkey,
          ),
        ]),
      ),
    [currentPubkey, directMessages, dmProfilesQuery.data, orgAgentPubkey],
  );
  const dmParticipantsByChannelId = React.useMemo(
    () =>
      Object.fromEntries(
        directMessages.map((channel) => {
          const participants = channel.participantPubkeys.map(
            (pubkey, index) => ({
              fallbackName: channel.participants[index] ?? null,
              pubkey,
            }),
          );
          const otherParticipants = participants.filter((participant) => {
            if (
              participant.pubkey.toLowerCase() === currentPubkey?.toLowerCase()
            ) {
              return false;
            }

            const participantLabel =
              participant.fallbackName?.trim().toLowerCase() ?? null;
            return !participantLabel || !selfDmLabels.has(participantLabel);
          });
          const visibleParticipants =
            otherParticipants.length > 0 ? otherParticipants : participants;
          const agentFace = orgAgentSidebarFace(
            channel,
            orgAgentPubkey ?? null,
            currentPubkey ?? null,
            dmProfiles?.[orgAgentPubkey?.toLowerCase() ?? ""]?.avatarUrl ??
              null,
          );
          if (agentFace) {
            return [channel.id, [agentFace]];
          }

          return [
            channel.id,
            visibleParticipants.map((participant) => ({
              avatarUrl:
                dmProfiles?.[participant.pubkey.toLowerCase()]?.avatarUrl ??
                null,
              label: resolveUserLabel({
                currentPubkey,
                fallbackName: participant.fallbackName,
                profiles: dmProfiles,
                pubkey: participant.pubkey,
              }),
              ...(dmProfiles?.[participant.pubkey.toLowerCase()]?.isAgent ===
              true
                ? { isAgent: true }
                : {}),
              pubkey: participant.pubkey,
            })),
          ];
        }),
      ) satisfies Record<string, SidebarDmParticipant[]>,
    [currentPubkey, directMessages, dmProfiles, orgAgentPubkey, selfDmLabels],
  );

  return {
    dmChannelLabels,
    dmParticipantsByChannelId,
    dmPresenceByChannelId,
  };
}
