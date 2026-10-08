import * as React from "react";
import { createFileRoute } from "@tanstack/react-router";

import { useIdentityQuery } from "@/shared/api/hooks";
import { usePreviewFeatureWarning } from "@/shared/features";
import { ViewLoadingFallback } from "@/shared/ui/ViewLoadingFallback";

const MemberProfileScreen = React.lazy(async () => {
  const module = await import("@/features/org/ui/MemberProfileScreen");
  return { default: module.MemberProfileScreen };
});

export const Route = createFileRoute("/org/profile")({
  component: OrgMyProfileRouteComponent,
});

function OrgMyProfileRouteComponent() {
  usePreviewFeatureWarning("org");

  const pubkey = useIdentityQuery().data?.pubkey ?? null;
  return (
    <React.Suspense fallback={<ViewLoadingFallback includeHeader kind="org" />}>
      <MemberProfileScreen key={pubkey ?? "self"} mine pubkey={pubkey} />
    </React.Suspense>
  );
}
