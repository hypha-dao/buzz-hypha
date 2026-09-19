import * as React from "react";
import { createFileRoute } from "@tanstack/react-router";

import { usePreviewFeatureWarning } from "@/shared/features";
import { ViewLoadingFallback } from "@/shared/ui/ViewLoadingFallback";

const WorkScreen = React.lazy(async () => {
  const module = await import("@/features/org/ui/WorkScreen");
  return { default: module.WorkScreen };
});

export const Route = createFileRoute("/org/work")({
  component: OrgWorkRouteComponent,
});

function OrgWorkRouteComponent() {
  usePreviewFeatureWarning("org");
  return (
    <React.Suspense fallback={<ViewLoadingFallback includeHeader kind="org" />}>
      <WorkScreen />
    </React.Suspense>
  );
}
