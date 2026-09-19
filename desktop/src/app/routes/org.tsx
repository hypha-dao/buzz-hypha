import * as React from "react";
import { createFileRoute } from "@tanstack/react-router";

import { usePreviewFeatureWarning } from "@/shared/features";
import { ViewLoadingFallback } from "@/shared/ui/ViewLoadingFallback";

const OverviewScreen = React.lazy(async () => {
  const module = await import("@/features/org/ui/OverviewScreen");
  return { default: module.OverviewScreen };
});

export const Route = createFileRoute("/org")({
  component: OrgRouteComponent,
});

function OrgRouteComponent() {
  usePreviewFeatureWarning("org");
  return (
    <React.Suspense fallback={<ViewLoadingFallback includeHeader kind="org" />}>
      <OverviewScreen />
    </React.Suspense>
  );
}
