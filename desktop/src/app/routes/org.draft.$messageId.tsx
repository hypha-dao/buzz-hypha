import * as React from "react";
import { createFileRoute } from "@tanstack/react-router";

import { usePreviewFeatureWarning } from "@/shared/features";
import { ViewLoadingFallback } from "@/shared/ui/ViewLoadingFallback";

const DraftScreen = React.lazy(async () => {
  const module = await import("@/features/org/ui/DraftScreen");
  return { default: module.DraftScreen };
});

export const Route = createFileRoute("/org/draft/$messageId")({
  component: OrgDraftRouteComponent,
});

function OrgDraftRouteComponent() {
  usePreviewFeatureWarning("org");
  const { messageId } = Route.useParams();
  return (
    <React.Suspense fallback={<ViewLoadingFallback includeHeader kind="org" />}>
      <DraftScreen messageId={messageId} />
    </React.Suspense>
  );
}
