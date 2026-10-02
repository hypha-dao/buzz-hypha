import * as React from "react";

import { useAppNavigation } from "@/app/navigation/useAppNavigation";
import { useUserProfileQuery } from "@/features/profile/hooks";
import { ProfileAvatar } from "@/features/profile/ui/ProfileAvatar";
import { normalizePubkey, truncatePubkey } from "@/shared/lib/pubkey";

import { useLiveDoorEvents, useOrgProfile, useWorkEvents } from "../hooks";
import { memberActivityFilters } from "../hooks/filters";
import {
  formatProfileWhen,
  PROFILE_ACTIVITY_CAP,
  PROFILE_ACTIVITY_PREVIEW,
  profileHeldWork,
  recentMemberActivity,
  visibleProfileActivity,
  voteSubjects,
  type MemberActivity,
} from "../memberProfile";
import { latestWorkItems, parseWorkItem, type WorkItem } from "../work/model";
import { OrgDoorScreen } from "./OrgDoorScreen";
import { motionDelay } from "./OrgMotionFrame";
import { OverviewCardShell } from "./overview/OverviewCardShell";
import { ProfileSocialLinks } from "./ProfileSocialLinks";
import {
  overviewLineDelayMs,
  overviewRuleDelayMs,
  overviewStampDelayMs,
} from "./overview/overviewMotion";

type MemberProfileScreenProps = {
  mine: boolean;
  pubkey: string | null;
};

function stateLabel(state: WorkItem["state"]): string {
  if (state === "accepted") return "In progress";
  if (state === "in_review") return "In review";
  if (state === "done") return "Done";
  return state;
}

function projectName(
  item: WorkItem,
  titles: ReadonlyMap<string, string>,
): string | null {
  if (item.type !== "ticket") return null;
  const parentId = item.parent ?? (item.root !== item.id ? item.root : null);
  if (!parentId) return null;
  return titles.get(parentId) ?? null;
}

function ProfileSection({
  children,
  count,
  enterIndex,
  testId,
  title,
}: {
  children: React.ReactNode;
  count: number;
  enterIndex: number;
  testId: string;
  title: string;
}) {
  return (
    <OverviewCardShell enterIndex={enterIndex} testId={testId}>
      <h2 className="text-base font-semibold leading-snug">
        {title} ({count})
      </h2>
      <span
        aria-hidden="true"
        className="org-dir-rule mt-3 block h-px w-10 bg-foreground/45"
        style={motionDelay(overviewRuleDelayMs(enterIndex))}
      />
      <div className="mt-2">{children}</div>
    </OverviewCardShell>
  );
}

function WorkRows({
  empty,
  enterIndex,
  items,
  onOpen,
  projectOf,
}: {
  empty: string;
  enterIndex: number;
  items: readonly WorkItem[];
  onOpen: (itemId: string) => void;
  projectOf: (item: WorkItem) => string | null;
}) {
  if (items.length === 0) {
    return (
      <p
        className="org-dir-line px-2 py-3 text-sm text-muted-foreground"
        style={motionDelay(overviewLineDelayMs(enterIndex, 0))}
      >
        {empty}
      </p>
    );
  }
  return (
    <ul>
      {items.map((item, index) => {
        const project = projectOf(item);
        const detail =
          item.type === "project"
            ? item.brief.trim() || null
            : project
              ? `In ${project}`
              : item.brief.trim() || null;
        return (
          <li key={item.id}>
            <button
              className="org-dir-line flex w-full items-start justify-between gap-3 rounded-lg px-2 py-3 text-left hover:bg-accent focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
              data-testid={`org-profile-work-${item.id}`}
              onClick={() => onOpen(item.id)}
              style={motionDelay(overviewLineDelayMs(enterIndex, index))}
              type="button"
            >
              <span className="min-w-0">
                <span className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
                  {item.type === "project" ? "Project" : "Ticket"}
                </span>
                <span className="mt-1 block truncate text-base font-medium">
                  {item.title}
                </span>
                {detail ? (
                  <span className="mt-0.5 block truncate text-sm text-muted-foreground">
                    {detail}
                  </span>
                ) : null}
              </span>
              <span className="shrink-0 pt-5 text-sm text-muted-foreground">
                {stateLabel(item.state)}
              </span>
            </button>
          </li>
        );
      })}
    </ul>
  );
}

function ActivityList({
  activity,
  enterIndex,
  expanded,
  nowSeconds,
  onOpen,
  onToggle,
}: {
  activity: readonly MemberActivity[];
  enterIndex: number;
  expanded: boolean;
  nowSeconds: number;
  onOpen: (itemId: string) => void;
  onToggle: () => void;
}) {
  const shown = visibleProfileActivity(activity, expanded);
  const hidden = activity.length - PROFILE_ACTIVITY_PREVIEW;
  if (activity.length === 0) {
    return (
      <p
        className="org-dir-line px-2 py-3 text-sm text-muted-foreground"
        style={motionDelay(overviewLineDelayMs(enterIndex, 0))}
      >
        No recent activity.
      </p>
    );
  }
  return (
    <>
      <ul id="org-profile-activity-list">
        {shown.map((entry, index) => {
          const when = formatProfileWhen(entry.createdAt, nowSeconds);
          const body = (
            <>
              <span className="min-w-0">
                <span className="block text-sm">{entry.label}</span>
                {entry.detail ? (
                  <span className="mt-0.5 block truncate text-sm text-muted-foreground">
                    {entry.detail}
                  </span>
                ) : null}
              </span>
              <time
                className="shrink-0 text-sm text-muted-foreground"
                dateTime={new Date(entry.createdAt * 1000).toISOString()}
              >
                {when}
              </time>
            </>
          );
          const rowClass =
            "org-dir-line flex w-full items-baseline justify-between gap-3 rounded-lg px-2 py-3 text-left";
          const rowStyle = motionDelay(overviewLineDelayMs(enterIndex, index));
          if (!entry.itemId) {
            return (
              <li className={rowClass} key={entry.id} style={rowStyle}>
                {body}
              </li>
            );
          }
          const itemId = entry.itemId;
          return (
            <li key={entry.id}>
              <button
                className={`${rowClass} hover:bg-accent focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring`}
                onClick={() => onOpen(itemId)}
                style={rowStyle}
                type="button"
              >
                {body}
              </button>
            </li>
          );
        })}
      </ul>
      {hidden > 0 ? (
        <div
          className="org-dir-stamp pt-3"
          style={motionDelay(overviewStampDelayMs(enterIndex, shown.length))}
        >
          <button
            aria-controls="org-profile-activity-list"
            aria-expanded={expanded}
            className="rounded-lg px-2 py-2 text-sm font-medium hover:bg-accent focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
            data-testid="org-profile-activity-more"
            onClick={onToggle}
            type="button"
          >
            {expanded ? "Show less" : `Show ${hidden} more`}
          </button>
        </div>
      ) : null}
    </>
  );
}

export function MemberProfileScreen({
  mine,
  pubkey,
}: MemberProfileScreenProps) {
  const navigation = useAppNavigation();
  const identity = useUserProfileQuery(pubkey ?? undefined).data;
  const orgProfile = useOrgProfile(pubkey).profile;
  const work = useWorkEvents();
  const activityReq = useLiveDoorEvents(
    pubkey ? memberActivityFilters(normalizePubkey(pubkey)) : [],
  );
  const nowSeconds = Math.floor(Date.now() / 1000);
  const [expandedActivity, setExpandedActivity] = React.useState(false);

  const held = React.useMemo(() => {
    if (!pubkey) {
      return {
        projects: [],
        tickets: [],
        earlier: [],
        titles: new Map<string, string>(),
      };
    }
    const items = [...latestWorkItems(work.events).values()]
      .map(parseWorkItem)
      .filter((item): item is WorkItem => item !== null)
      .filter((item) => item.state !== "withdrawn");
    const titles = new Map(items.map((item) => [item.id, item.title]));
    return { ...profileHeldWork(items, pubkey), titles };
  }, [pubkey, work.events]);

  const activity = React.useMemo(
    () =>
      recentMemberActivity(
        activityReq.events,
        PROFILE_ACTIVITY_CAP,
        voteSubjects(activityReq.events),
      ),
    [activityReq.events],
  );

  const name =
    identity?.displayName?.trim() ||
    (pubkey ? truncatePubkey(pubkey) : "Profile");
  const title = mine ? "My profile" : name;
  const about = orgProfile.about.trim() || identity?.about?.trim() || "";
  const interests = orgProfile.skills.map((skill) => skill.label);
  const hasOrgDetail =
    orgProfile.about.trim().length > 0 ||
    interests.length > 0 ||
    orgProfile.socials.length > 0;

  const openItem = (itemId: string) => {
    void navigation.goOrgWorkItem(itemId);
  };

  return (
    <OrgDoorScreen testId="org-member-profile" title={title}>
      <div className="min-h-0 flex-1 overflow-y-auto px-6 py-6">
        <div className="mx-auto flex max-w-3xl flex-col gap-4">
          <OverviewCardShell enterIndex={0}>
            <div className="flex items-start gap-3">
              <span aria-hidden="true">
                <ProfileAvatar
                  avatarUrl={identity?.avatarUrl ?? null}
                  className="h-12 w-12"
                  label={name}
                />
              </span>
              <div className="min-w-0 flex-1">
                <p
                  className="org-dir-line truncate text-base font-medium"
                  style={motionDelay(overviewLineDelayMs(0, 0))}
                >
                  {name}
                </p>
                {about ? (
                  <div
                    className="org-dir-line mt-3"
                    style={motionDelay(overviewLineDelayMs(0, 1))}
                  >
                    {orgProfile.about.trim() ? (
                      <p className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
                        About
                      </p>
                    ) : null}
                    <p className="mt-1 text-sm text-muted-foreground">
                      {about}
                    </p>
                  </div>
                ) : null}
                {interests.length > 0 ? (
                  <div
                    className="org-dir-line mt-3"
                    style={motionDelay(overviewLineDelayMs(0, 2))}
                  >
                    <p className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
                      Work I'm interested in
                    </p>
                    <ul className="mt-2 flex flex-wrap gap-1.5">
                      {interests.map((label) => (
                        <li
                          className="rounded-full border border-border/70 px-2.5 py-0.5 text-sm"
                          key={label}
                        >
                          {label}
                        </li>
                      ))}
                    </ul>
                  </div>
                ) : null}
                <div
                  className="org-dir-line"
                  style={motionDelay(overviewLineDelayMs(0, 3))}
                >
                  <ProfileSocialLinks socials={orgProfile.socials} />
                </div>
                {mine && !hasOrgDetail ? (
                  <p
                    className="org-dir-line mt-3 text-sm text-muted-foreground"
                    style={motionDelay(overviewLineDelayMs(0, 1))}
                  >
                    Tell the org agent what you do, the work you want, and links
                    to your socials. It saves them here, and uses them when
                    suggesting who should hold new work.
                  </p>
                ) : null}
              </div>
            </div>
          </OverviewCardShell>

          <ProfileSection
            count={held.projects.length}
            enterIndex={1}
            testId="org-profile-projects"
            title="Project DRI"
          >
            <WorkRows
              empty={
                mine
                  ? "No projects you are responsible for."
                  : "No projects they are responsible for."
              }
              enterIndex={1}
              items={held.projects}
              onOpen={openItem}
              projectOf={() => null}
            />
          </ProfileSection>

          <ProfileSection
            count={held.tickets.length}
            enterIndex={2}
            testId="org-profile-tickets"
            title="Tickets"
          >
            <WorkRows
              empty="No tickets in progress."
              enterIndex={2}
              items={held.tickets}
              onOpen={openItem}
              projectOf={(item) => projectName(item, held.titles)}
            />
          </ProfileSection>

          <ProfileSection
            count={held.earlier.length}
            enterIndex={3}
            testId="org-profile-earlier"
            title="Earlier work"
          >
            <WorkRows
              empty="No finished work yet."
              enterIndex={3}
              items={held.earlier}
              onOpen={openItem}
              projectOf={(item) => projectName(item, held.titles)}
            />
          </ProfileSection>

          <ProfileSection
            count={activity.length}
            enterIndex={4}
            testId="org-profile-activity"
            title="Recent activity"
          >
            <ActivityList
              activity={activity}
              enterIndex={4}
              expanded={expandedActivity}
              nowSeconds={nowSeconds}
              onOpen={openItem}
              onToggle={() => setExpandedActivity((current) => !current)}
            />
          </ProfileSection>
        </div>
      </div>
    </OrgDoorScreen>
  );
}
