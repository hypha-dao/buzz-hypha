import type { UserProfileSummary } from "@/shared/api/types";
import { Button } from "@/shared/ui/button";
import { Card } from "@/shared/ui/card";

import { OrgPersonName } from "./OrgPersonName";
import type { ProjectHold } from "./parseOverview";

type WhoHoldsWhatProps = {
  holds: readonly ProjectHold[];
  profiles: Record<string, UserProfileSummary>;
  onOpenItem: (itemId: string) => void;
};

export function WhoHoldsWhat({
  holds,
  profiles,
  onOpenItem,
}: WhoHoldsWhatProps) {
  return (
    <Card className="p-4" data-testid="org-who-holds">
      <h2 className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
        Who holds a project
      </h2>
      {holds.length === 0 ? (
        <p className="mt-2 text-sm text-muted-foreground">Not set yet.</p>
      ) : (
        <ul className="mt-2 space-y-1">
          {holds.map((hold) => (
            <li key={hold.id}>
              <Button
                className="h-auto w-full justify-start px-2 py-2 text-left font-normal"
                data-testid={`org-hold-${hold.id}`}
                onClick={() => onOpenItem(hold.id)}
                type="button"
                variant="ghost"
              >
                {hold.dri ? (
                  <span className="text-sm">
                    <OrgPersonName profiles={profiles} pubkey={hold.dri} />
                    {" · "}
                    {hold.title}
                  </span>
                ) : (
                  <span className="text-sm">
                    <span className="text-muted-foreground">{hold.title}</span>
                    <span className="mt-0.5 block text-2xs text-muted-foreground">
                      needs a DRI
                    </span>
                  </span>
                )}
              </Button>
            </li>
          ))}
        </ul>
      )}
    </Card>
  );
}
