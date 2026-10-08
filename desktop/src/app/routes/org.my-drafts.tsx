import * as React from "react";
import { createFileRoute } from "@tanstack/react-router";

import { usePreviewFeatureWarning } from "@/shared/features";
import { ViewLoadingFallback } from "@/shared/ui/ViewLoadingFallback";

const MyDraftsScreen = React.lazy(async () => {
  const module = await import("@/features/org/ui/MyDraftsScreen");
  return { default: module.MyDraftsScreen };
});

export const Route = createFileRoute("/org/my-drafts")({
  component: OrgMyDraftsRouteComponent,
});

function OrgMyDraftsRouteComponent() {
  usePreviewFeatureWarning("org");
  return (
    <React.Suspense fallback={<ViewLoadingFallback includeHeader kind="org" />}>
      <MyDraftsScreen />
    </React.Suspense>
  );
}
