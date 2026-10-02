import * as React from "react";
import { createFileRoute } from "@tanstack/react-router";

import { usePreviewFeatureWarning } from "@/shared/features";
import { ViewLoadingFallback } from "@/shared/ui/ViewLoadingFallback";

const ProposalScreen = React.lazy(async () => {
  const module = await import("@/features/org/ui/ProposalScreen");
  return { default: module.ProposalScreen };
});

export const Route = createFileRoute("/org/proposal/$proposalId")({
  component: OrgProposalRouteComponent,
});

function OrgProposalRouteComponent() {
  usePreviewFeatureWarning("org");
  const { proposalId } = Route.useParams();
  return (
    <React.Suspense fallback={<ViewLoadingFallback includeHeader kind="org" />}>
      <ProposalScreen proposalId={proposalId} />
    </React.Suspense>
  );
}
