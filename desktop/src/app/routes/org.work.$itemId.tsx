import * as React from "react";
import { createFileRoute } from "@tanstack/react-router";

import { usePreviewFeatureWarning } from "@/shared/features";
import { ViewLoadingFallback } from "@/shared/ui/ViewLoadingFallback";

const WorkItemScreen = React.lazy(async () => {
  const module = await import("@/features/org/ui/WorkItemScreen");
  return { default: module.WorkItemScreen };
});

export const Route = createFileRoute("/org/work/$itemId")({
  component: OrgWorkItemRouteComponent,
});

function OrgWorkItemRouteComponent() {
  usePreviewFeatureWarning("org");
  const { itemId } = Route.useParams();
  return (
    <React.Suspense fallback={<ViewLoadingFallback includeHeader kind="org" />}>
      <WorkItemScreen itemId={itemId} />
    </React.Suspense>
  );
}
