import * as React from "react";

import type { UserProfileSummary } from "@/shared/api/types";
import { Button } from "@/shared/ui/button";
import { Card } from "@/shared/ui/card";

import type { DirectionSlug } from "../../commands";

import { DirectionFormDialog } from "./DirectionFormDialog";
import { DIRECTION_LABEL, NOT_SET_YET } from "./overviewCopy";
import { OrgPersonName } from "./OrgPersonName";
import type { DirectionSlot } from "./parseOverview";

type DirectionCardsProps = {
  slots: readonly DirectionSlot[];
  isShaper: boolean;
  profiles: Record<string, UserProfileSummary>;
  onOpenDirection: (slug: DirectionSlug) => void;
};

export function DirectionCards({
  slots,
  isShaper,
  profiles,
  onOpenDirection,
}: DirectionCardsProps) {
  const [openSlug, setOpenSlug] = React.useState<DirectionSlug | null>(null);

  return (
    <section aria-labelledby="org-direction-heading">
      <h2
        className="text-2xs font-medium uppercase tracking-wider text-muted-foreground"
        id="org-direction-heading"
      >
        Direction
      </h2>
      <div className="mt-2 grid gap-3 md:grid-cols-2">
        {slots.map((slot) => {
          const label = DIRECTION_LABEL[slot.slug];
          const head = slot.head;
          return (
            <Card
              className="flex flex-col gap-3 p-4"
              data-testid={`org-direction-card-${slot.slug}`}
              key={slot.slug}
            >
              <div>
                <h3 className="text-sm font-semibold">
                  {label.title}
                  <span className="font-normal text-muted-foreground">
                    {" "}
                    — {label.question}
                  </span>
                </h3>
                {head === null ? (
                  <p
                    className="mt-2 text-sm text-muted-foreground"
                    data-testid={`org-direction-empty-${slot.slug}`}
                  >
                    {NOT_SET_YET}
                  </p>
                ) : (
                  <>
                    <p
                      className="mt-1 text-2xs text-muted-foreground"
                      data-testid={`org-direction-meta-${slot.slug}`}
                    >
                      v{head.version}
                      {head.confirmedBy ? (
                        <>
                          {" "}
                          · confirmed by{" "}
                          <OrgPersonName
                            profiles={profiles}
                            pubkey={head.confirmedBy}
                          />
                        </>
                      ) : null}
                    </p>
                    <p className="mt-2 text-message leading-relaxed">
                      {head.lines.length > 0
                        ? head.lines.map((line) => line.text).join(" · ")
                        : head.body}
                    </p>
                  </>
                )}
              </div>
              <div className="mt-auto flex flex-wrap gap-2">
                <Button
                  data-testid={`org-direction-open-${slot.slug}`}
                  onClick={() => onOpenDirection(slot.slug)}
                  size="sm"
                  type="button"
                  variant="outline"
                >
                  Full text and versions
                </Button>
                {isShaper ? (
                  <Button
                    data-testid={`org-direction-propose-${slot.slug}`}
                    onClick={() => setOpenSlug(slot.slug)}
                    size="sm"
                    type="button"
                  >
                    Propose a version
                  </Button>
                ) : null}
              </div>
            </Card>
          );
        })}
      </div>
      {openSlug ? (
        <DirectionFormDialog
          base={
            slots.find((slot) => slot.slug === openSlug)?.head?.version ?? 0
          }
          onOpenChange={(open) => {
            if (!open) setOpenSlug(null);
          }}
          open
          slug={openSlug}
        />
      ) : null}
    </section>
  );
}
