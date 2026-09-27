import * as React from "react";
import { createFileRoute } from "@tanstack/react-router";

import { usePreviewFeatureWarning } from "@/shared/features";
import { ViewLoadingFallback } from "@/shared/ui/ViewLoadingFallback";

const DirectionScreen = React.lazy(async () => {
  const module = await import("@/features/org/ui/DirectionScreen");
  return { default: module.DirectionScreen };
});

export const Route = createFileRoute("/org/direction/$slug")({
  component: OrgDirectionRouteComponent,
});

function OrgDirectionRouteComponent() {
  usePreviewFeatureWarning("org");
  const { slug } = Route.useParams();
  return (
    <React.Suspense fallback={<ViewLoadingFallback includeHeader kind="org" />}>
      <DirectionScreen slug={slug} />
    </React.Suspense>
  );
}
