import { index, rootRoute, route } from "@tanstack/virtual-file-routes";

export const routes = rootRoute("root.tsx", [
  index("index.tsx"),
  route("/agents", "agents.tsx"),
  route("/org", "org.tsx"),
  route("/org/work", "org.work.tsx"),
  route("/org/work/$itemId", "org.work.$itemId.tsx"),
  route("/org/my-work", "org.my-work.tsx"),
  route("/org/direction/$slug", "org.direction.$slug.tsx"),
  route("/pulse", "pulse.tsx"),
  route("/reminders", "reminders.tsx"),
  route("/settings", "settings.tsx"),
  route("/workflows", "workflows.tsx"),
  route("/workflows/$workflowId", "workflows.$workflowId.tsx"),
  route("/projects", "projects.tsx"),
  route("/projects/$projectId", "projects.$projectId.tsx"),
  route("/messages/new", "messages.new.tsx"),
  route("/channels/$channelId", "channels.$channelId.tsx"),
  route(
    "/channels/$channelId/posts/$postId",
    "channels.$channelId.posts.$postId.tsx",
  ),
]);
