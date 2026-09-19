import type { DirectionSlug } from "@/features/org/commands";

import { asNumber, asString, parseJsonObject } from "./tags";
import type { OrgCardModel } from "./types";

const DIRECTION_SLUGS = new Set<DirectionSlug>([
  "mission",
  "vision",
  "objectives",
  "strategy",
]);

export function projectProposeInput(model: OrgCardModel, title?: string) {
  const content = parseJsonObject(model.event.content);
  return {
    title: title ?? asString(content?.title) ?? model.claim,
    brief: asString(content?.brief) ?? "",
    dueAt: asNumber(content?.due_at) ?? 0,
    objectiveRef: asString(content?.objective_ref) ?? undefined,
    suggestedDri: asString(content?.suggested_dri) ?? undefined,
    draftId: model.event.id,
    voteAgree: true,
  };
}

export function ticketCreateInput(model: OrgCardModel, title?: string) {
  const content = parseJsonObject(model.event.content);
  return {
    parent: asString(content?.parent) ?? model.itemId ?? "",
    title: title ?? asString(content?.title) ?? model.claim,
    brief: asString(content?.brief) ?? "",
    dueAt: asNumber(content?.due_at) ?? 0,
    offerTo: asString(content?.suggested_holder) ?? undefined,
    after: Array.isArray(content?.after)
      ? content.after.filter(
          (value): value is string => typeof value === "string",
        )
      : undefined,
    draftId: model.event.id,
  };
}

export function directionProposeInput(model: OrgCardModel) {
  const content = parseJsonObject(model.event.content);
  const slugRaw = asString(content?.slug);
  const slug: DirectionSlug = DIRECTION_SLUGS.has(slugRaw as DirectionSlug)
    ? (slugRaw as DirectionSlug)
    : "objectives";
  const lines = Array.isArray(content?.lines) ? content.lines : undefined;
  return {
    slug,
    base: asNumber(content?.base_version) ?? 0,
    body: asString(content?.body) ?? "",
    lines,
    why: asString(content?.why) ?? undefined,
    draftId: model.event.id,
    voteAgree: true,
  };
}

export function profileSetInput(model: OrgCardModel) {
  const content = parseJsonObject(model.event.content);
  const skills = Array.isArray(content?.skills)
    ? content.skills.filter(
        (value): value is string => typeof value === "string",
      )
    : [];
  return {
    about: asString(content?.about) ?? "",
    skills,
    openLimit: asNumber(content?.open_limit) ?? undefined,
    draftId: model.event.id,
  };
}

export function reviewProjectInput(model: OrgCardModel) {
  const content = parseJsonObject(model.event.content);
  const recommendation =
    content?.recommendation && typeof content.recommendation === "object"
      ? (content.recommendation as Record<string, unknown>)
      : null;
  const project =
    recommendation?.project && typeof recommendation.project === "object"
      ? (recommendation.project as Record<string, unknown>)
      : null;
  return {
    title: asString(project?.title) ?? model.claim,
    brief: asString(project?.brief) ?? "",
    dueAt: asNumber(project?.due_at) ?? 0,
    objectiveRef: asString(project?.objective_ref) ?? undefined,
    suggestedDri: asString(project?.suggested_dri) ?? undefined,
    draftId: model.event.id,
    voteAgree: true,
  };
}
