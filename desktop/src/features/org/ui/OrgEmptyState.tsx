export const ORG_EMPTY_NOTHING_NEEDS_YOU = "Nothing needs you.";
export const ORG_EMPTY_NOT_SET_YET = "Not set yet.";

type OrgEmptyStateProps = {
  message: typeof ORG_EMPTY_NOTHING_NEEDS_YOU | typeof ORG_EMPTY_NOT_SET_YET;
  testId: string;
};

export function OrgEmptyState({ message, testId }: OrgEmptyStateProps) {
  return (
    <div
      className="flex flex-1 flex-col items-center justify-center px-6 py-16 text-center"
      data-testid={testId}
    >
      <p className="text-sm text-muted-foreground">{message}</p>
    </div>
  );
}
