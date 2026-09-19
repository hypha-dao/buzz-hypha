import { ClipboardList, LayoutDashboard, ListTree } from "lucide-react";
import { useLocation } from "@tanstack/react-router";

import { useAppNavigation } from "@/app/navigation/useAppNavigation";
import { SidebarMenuButton, SidebarMenuItem } from "@/shared/ui/sidebar";
import { SidebarMenuLabel } from "@/shared/ui/sidebar-menu-label";

const ITEMS = [
  {
    icon: LayoutDashboard,
    label: "Overview",
    match: (pathname: string) => pathname === "/org",
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
  {
    icon: ClipboardList,
    label: "My Work",
    match: (pathname: string) => pathname === "/org/my-work",
    onSelect: "goOrgMyWork" as const,
    testId: "sidebar-org-my-work",
  },
];

/**
 * Primary-menu group for the three Phase 0 doors. Hidden when the `org`
 * preview feature is off (the FeatureGate wraps this). Items are `li`s so
 * they can sit inside the existing `SidebarMenu` list.
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
      {ITEMS.map((item) => {
        const Icon = item.icon;
        return (
          <SidebarMenuItem key={item.testId}>
            <SidebarMenuButton
              className="data-[active=true]:font-normal"
              data-testid={item.testId}
              isActive={item.match(pathname)}
              onClick={() => {
                void navigation[item.onSelect]();
              }}
              tooltip={item.label}
              type="button"
            >
              <Icon className="h-4 w-4" />
              <SidebarMenuLabel>{item.label}</SidebarMenuLabel>
            </SidebarMenuButton>
          </SidebarMenuItem>
        );
      })}
    </>
  );
}
