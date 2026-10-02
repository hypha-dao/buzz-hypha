import * as React from "react";
import { createFileRoute } from "@tanstack/react-router";

import { usePreviewFeatureWarning } from "@/shared/features";
import { ViewLoadingFallback } from "@/shared/ui/ViewLoadingFallback";

const MemberProfileScreen = React.lazy(async () => {
  const module = await import("@/features/org/ui/MemberProfileScreen");
  return { default: module.MemberProfileScreen };
});

export const Route = createFileRoute("/org/profile/$pubkey")({
  component: OrgMemberProfileRouteComponent,
});

function OrgMemberProfileRouteComponent() {
  usePreviewFeatureWarning("org");
  const { pubkey } = Route.useParams();
  return (
    <React.Suspense fallback={<ViewLoadingFallback includeHeader kind="org" />}>
      <MemberProfileScreen key={pubkey} mine={false} pubkey={pubkey} />
    </React.Suspense>
  );
}
