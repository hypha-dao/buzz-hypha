import * as React from "react";

import {
  buildIoAccept,
  buildIoDecline,
  buildIoDirectionPropose,
  buildIoDone,
  buildIoDraftDecide,
  buildIoDriPropose,
  buildIoHealthRate,
  buildIoJoinPropose,
  buildIoMoneyPropose,
  buildIoMoneyReleased,
  buildIoOffer,
  buildIoProfileSet,
  buildIoProjectPropose,
  buildIoRelease,
  buildIoReopen,
  buildIoSetDue,
  buildIoShaperAccept,
  buildIoShaperStepDown,
  buildIoShapersPropose,
  buildIoTicketCreate,
  buildIoVote,
  publishOrgCommand,
  type UnsignedOrgCommand,
} from "./commands";

export type OrgCommands = {
  publish: (
    command: UnsignedOrgCommand,
  ) => ReturnType<typeof publishOrgCommand>;
  buildIoShapersPropose: typeof buildIoShapersPropose;
  buildIoDirectionPropose: typeof buildIoDirectionPropose;
  buildIoVote: typeof buildIoVote;
  buildIoShaperAccept: typeof buildIoShaperAccept;
  buildIoShaperStepDown: typeof buildIoShaperStepDown;
  buildIoProjectPropose: typeof buildIoProjectPropose;
  buildIoTicketCreate: typeof buildIoTicketCreate;
  buildIoOffer: typeof buildIoOffer;
  buildIoAccept: typeof buildIoAccept;
  buildIoDecline: typeof buildIoDecline;
  buildIoDone: typeof buildIoDone;
  buildIoRelease: typeof buildIoRelease;
  buildIoSetDue: typeof buildIoSetDue;
  buildIoReopen: typeof buildIoReopen;
  buildIoDraftDecide: typeof buildIoDraftDecide;
  buildIoDriPropose: typeof buildIoDriPropose;
  buildIoHealthRate: typeof buildIoHealthRate;
  buildIoJoinPropose: typeof buildIoJoinPropose;
  buildIoMoneyPropose: typeof buildIoMoneyPropose;
  buildIoMoneyReleased: typeof buildIoMoneyReleased;
  buildIoProfileSet: typeof buildIoProfileSet;
};

const COMMANDS: OrgCommands = {
  publish: publishOrgCommand,
  buildIoShapersPropose,
  buildIoDirectionPropose,
  buildIoVote,
  buildIoShaperAccept,
  buildIoShaperStepDown,
  buildIoProjectPropose,
  buildIoTicketCreate,
  buildIoOffer,
  buildIoAccept,
  buildIoDecline,
  buildIoDone,
  buildIoRelease,
  buildIoSetDue,
  buildIoReopen,
  buildIoDraftDecide,
  buildIoDriPropose,
  buildIoHealthRate,
  buildIoJoinPropose,
  buildIoMoneyPropose,
  buildIoMoneyReleased,
  buildIoProfileSet,
};

/** Stable command publishers — build, then `sign_event` + EVENT. */
export function useOrgCommands(): OrgCommands {
  return COMMANDS;
}

declare global {
  interface Window {
    __BUZZ_E2E_ORG_COMMANDS__?: OrgCommands;
  }
}

/**
 * Expose the production command hook on the mock-bridge window so a
 * Playwright spec can assert kind and tags on the captured `sign_event`.
 * No-op outside `e2e` builds.
 */
export function useOrgCommandE2eBridge(): void {
  const commands = useOrgCommands();

  React.useEffect(() => {
    if (import.meta.env.MODE !== "e2e") return;
    window.__BUZZ_E2E_ORG_COMMANDS__ = commands;
    return () => {
      delete window.__BUZZ_E2E_ORG_COMMANDS__;
    };
  }, [commands]);
}
