import * as React from "react";
import { createFileRoute } from "@tanstack/react-router";

import { usePreviewFeatureWarning } from "@/shared/features";
import { ViewLoadingFallback } from "@/shared/ui/ViewLoadingFallback";

const MyWorkScreen = React.lazy(async () => {
  const module = await import("@/features/org/ui/MyWorkScreen");
  return { default: module.MyWorkScreen };
});

export const Route = createFileRoute("/org/my-work")({
  component: OrgMyWorkRouteComponent,
});

function OrgMyWorkRouteComponent() {
  usePreviewFeatureWarning("org");
  return (
    <React.Suspense fallback={<ViewLoadingFallback includeHeader kind="org" />}>
      <MyWorkScreen />
    </React.Suspense>
  );
}
