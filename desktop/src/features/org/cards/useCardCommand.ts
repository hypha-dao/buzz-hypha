import * as React from "react";

import { useOrgCommands } from "@/features/org/useOrgCommands";
import type { UnsignedOrgCommand } from "@/features/org/commands";

export function useCardCommand() {
  const commands = useOrgCommands();
  const [pending, setPending] = React.useState<string | null>(null);
  const [error, setError] = React.useState<string | null>(null);

  const publish = React.useCallback(
    async (label: string, command: UnsignedOrgCommand) => {
      setPending(label);
      setError(null);
      try {
        await commands.publish(command);
      } catch (cause) {
        const message =
          cause instanceof Error ? cause.message : "Failed to publish";
        setError(message);
        throw cause;
      } finally {
        setPending(null);
      }
    },
    [commands],
  );

  return { commands, error, pending, publish };
}
