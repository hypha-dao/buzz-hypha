import {
  ClipboardList,
  FilePen,
  LayoutDashboard,
  ListTree,
  UserRound,
} from "lucide-react";
import { useLocation } from "@tanstack/react-router";

import { useAppNavigation } from "@/app/navigation/useAppNavigation";
import { SidebarMenuButton, SidebarMenuItem } from "@/shared/ui/sidebar";
import { SidebarMenuLabel } from "@/shared/ui/sidebar-menu-label";

const ORG_ITEMS = [
  {
    icon: LayoutDashboard,
    label: "Overview",
    match: (pathname: string) =>
      pathname === "/org" || pathname.startsWith("/org/direction"),
    onSelect: "goOrg" as const,
    testId: "sidebar-org-overview",
  },
  {
    icon: ListTree,
    label: "Work",
    match: (pathname: string) =>
      pathname === "/org/work" || pathname.startsWith("/org/work/"),
    onSelect: "goOrgWork" as const,
    testId: "sidebar-org-work",
  },
];

const PERSONAL_ITEMS = [
  {
    icon: ClipboardList,
    label: "My work",
    match: (pathname: string) => pathname === "/org/my-work",
    onSelect: "goOrgMyWork" as const,
    testId: "sidebar-org-my-work",
  },
  {
    icon: FilePen,
    label: "My drafts",
    match: (pathname: string) =>
      pathname === "/org/my-drafts" || pathname.startsWith("/org/draft/"),
    onSelect: "goOrgMyDrafts" as const,
    testId: "sidebar-org-my-drafts",
  },
  {
    icon: UserRound,
    label: "My profile",
    match: (pathname: string) => pathname === "/org/profile",
    onSelect: "goOrgProfile" as const,
    testId: "sidebar-org-my-profile",
  },
];

/**
 * Primary-menu groups for the org doors. Hidden when the `org` preview
 * feature is off (the FeatureGate wraps this). Items are `li`s so they can
 * sit inside the existing `SidebarMenu` list.
 */
export function OrgSidebarGroup() {
  const navigation = useAppNavigation();
  const pathname = useLocation({ select: (location) => location.pathname });

  return (
    <>
      <SidebarMenuItem data-testid="sidebar-org-group">
        <span
          className="px-2 py-1 text-2xs font-medium uppercase tracking-wider text-sidebar-foreground/50"
          data-testid="sidebar-org-group-label"
        >
          Org
        </span>
      </SidebarMenuItem>
      {ORG_ITEMS.map((item) => (
        <SidebarItem
          item={item}
          key={item.testId}
          navigation={navigation}
          pathname={pathname}
        />
      ))}
      <SidebarMenuItem data-testid="sidebar-personal-group">
        <span
          className="px-2 py-1 text-2xs font-medium uppercase tracking-wider text-sidebar-foreground/50"
          data-testid="sidebar-personal-group-label"
        >
          Personal
        </span>
      </SidebarMenuItem>
      {PERSONAL_ITEMS.map((item) => (
        <SidebarItem
          item={item}
          key={item.testId}
          navigation={navigation}
          pathname={pathname}
        />
      ))}
    </>
  );
}

function openSidebarItem(
  navigation: ReturnType<typeof useAppNavigation>,
  onSelect:
    | (typeof ORG_ITEMS)[number]["onSelect"]
    | (typeof PERSONAL_ITEMS)[number]["onSelect"],
) {
  switch (onSelect) {
    case "goOrg":
      return navigation.goOrg();
    case "goOrgWork":
      return navigation.goOrgWork();
    case "goOrgMyWork":
      return navigation.goOrgMyWork();
    case "goOrgMyDrafts":
      return navigation.goOrgMyDrafts();
    case "goOrgProfile":
      return navigation.goOrgProfile();
  }
}

function SidebarItem({
  item,
  navigation,
  pathname,
}: {
  item: (typeof ORG_ITEMS)[number] | (typeof PERSONAL_ITEMS)[number];
  navigation: ReturnType<typeof useAppNavigation>;
  pathname: string;
}) {
  const Icon = item.icon;
  return (
    <SidebarMenuItem>
      <SidebarMenuButton
        className="data-[active=true]:font-normal"
        data-testid={item.testId}
        isActive={item.match(pathname)}
        onClick={() => {
          void openSidebarItem(navigation, item.onSelect);
        }}
        tooltip={item.label}
        type="button"
      >
        <Icon className="h-4 w-4" />
        <SidebarMenuLabel>{item.label}</SidebarMenuLabel>
      </SidebarMenuButton>
    </SidebarMenuItem>
  );
}
