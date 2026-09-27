import * as React from "react";

import { Button } from "@/shared/ui/button";

import { DECLINE_REASON_LABEL, DECLINE_REASONS } from "./reasons";
import type { DeclineReason } from "./types";

type ActionButtonProps = {
  disabled?: boolean;
  label: string;
  onClick: () => void;
  testId: string;
  variant?: "default" | "outline" | "secondary" | "ghost";
};

export function ActionButton({
  disabled,
  label,
  onClick,
  testId,
  variant = "default",
}: ActionButtonProps) {
  return (
    <Button
      data-testid={testId}
      disabled={disabled}
      onClick={onClick}
      size="sm"
      type="button"
      variant={variant}
    >
      {label}
    </Button>
  );
}

type DeclineChipsProps = {
  disabled?: boolean;
  onSelect: (reason: DeclineReason) => void;
  selected: DeclineReason | null;
};

export function DeclineChips({
  disabled,
  onSelect,
  selected,
}: DeclineChipsProps) {
  const groupName = React.useId();
  return (
    <fieldset
      className="flex w-full flex-wrap gap-1.5 border-0 p-0"
      data-testid="org-card-reasons"
    >
      <legend className="sr-only">Decline reason</legend>
      {DECLINE_REASONS.map((reason) => {
        const pressed = selected === reason;
        return (
          <label
            className="inline-flex cursor-pointer items-center rounded-full border border-border/70 px-2.5 py-1 text-2xs text-muted-foreground hover:bg-muted/70 has-[:focus-visible]:ring-1 has-[:focus-visible]:ring-ring has-[:checked]:border-foreground has-[:checked]:text-foreground"
            data-testid={`org-card-reason-${reason}`}
            key={reason}
          >
            <input
              checked={pressed}
              className="sr-only"
              disabled={disabled}
              name={groupName}
              onChange={() => onSelect(reason)}
              type="radio"
              value={reason}
            />
            {DECLINE_REASON_LABEL[reason]}
          </label>
        );
      })}
    </fieldset>
  );
}
