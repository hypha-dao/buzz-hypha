import {
  KIND_IO_ACCEPT,
  KIND_IO_SHAPER_ACCEPT,
  KIND_IO_VOTE,
} from "@/shared/constants/kinds";

/**
 * Commands whose success can put the signer in a room the sidebar did not
 * have yet: accepting work (project or ticket home), accepting a Shaper
 * seat, or a vote that passes and names the signer (a project DRI).
 */
const JOINS_A_ROOM = new Set<number>([
  KIND_IO_ACCEPT,
  KIND_IO_SHAPER_ACCEPT,
  KIND_IO_VOTE,
]);

export function orgCommandJoinsARoom(kind: number): boolean {
  return JOINS_A_ROOM.has(kind);
}

/** Refresh the sidebar after a command that may have joined a room. */
export function afterOrgCommandPublished(
  kind: number,
  refreshChannelList: () => void,
): void {
  if (orgCommandJoinsARoom(kind)) {
    refreshChannelList();
  }
}
