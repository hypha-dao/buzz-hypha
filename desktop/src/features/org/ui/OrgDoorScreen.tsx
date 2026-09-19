import type * as React from "react";

import { ChatHeader } from "@/features/chat/ui/ChatHeader";
import { TopChromeInsetHeader } from "@/shared/layout/TopChromeInsetHeader";

import {
  ORG_EMPTY_NOT_SET_YET,
  ORG_EMPTY_NOTHING_NEEDS_YOU,
  OrgEmptyState,
} from "./OrgEmptyState";

type OrgDoorScreenProps = {
  children?: React.ReactNode;
  empty?: typeof ORG_EMPTY_NOTHING_NEEDS_YOU | typeof ORG_EMPTY_NOT_SET_YET;
  testId: string;
  title: string;
};

export function OrgDoorScreen({
  children,
  empty,
  testId,
  title,
}: OrgDoorScreenProps) {
  return (
    <div
      className="relative flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden"
      data-testid={testId}
    >
      <TopChromeInsetHeader data-tauri-drag-region flush>
        <ChatHeader mode="org" title={title} />
      </TopChromeInsetHeader>
      {children ??
        (empty ? (
          <OrgEmptyState message={empty} testId={`${testId}-empty`} />
        ) : null)}
    </div>
  );
}

export { ORG_EMPTY_NOT_SET_YET, ORG_EMPTY_NOTHING_NEEDS_YOU };
