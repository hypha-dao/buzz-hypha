import { DecisionCard } from "./DecisionCard";
import { DoneCard } from "./DoneCard";
import { DraftCard } from "./DraftCard";
import { OfferCard } from "./OfferCard";
import { ReviewCard } from "./ReviewCard";
import type { OrgCardModel } from "./types";

export function OrgEventCard({ model }: { model: OrgCardModel }) {
  switch (model.cardType) {
    case "offer":
      return <OfferCard model={model} />;
    case "decision":
      return <DecisionCard model={model} />;
    case "done":
      return <DoneCard model={model} />;
    case "review":
      return <ReviewCard model={model} />;
    default:
      return <DraftCard model={model} />;
  }
}
