import type { DirectMessageIntro } from "@/features/channels/lib/dmParticipantDisplay";
import { cn } from "@/shared/lib/cn";
import { DirectMessageIntroAvatarStack } from "./DirectMessageIntroAvatarStack";

export function DirectMessageIntroBlock({
  className,
  intro,
}: {
  className?: string;
  intro: DirectMessageIntro;
}) {
  const opening = intro.opening;

  return (
    <div
      className={cn(
        "flex w-full flex-col items-start px-3 text-left",
        className,
      )}
      data-testid="message-dm-intro"
    >
      <DirectMessageIntroAvatarStack participants={intro.participants} />
      <p className="mt-4 max-w-full truncate text-xl font-semibold leading-7 tracking-tight text-foreground">
        {intro.displayName}
      </p>
      {opening ? (
        <>
          <p
            className="mt-1 max-w-md text-sm leading-5 text-foreground"
            data-testid="org-agent-opening"
          >
            {opening.lead}
          </p>
          {opening.items.length > 0 ? (
            <dl className="mt-4 flex max-w-md flex-col gap-3">
              {opening.items.map((item) => (
                <div key={item.title}>
                  <dt className="text-sm font-medium leading-5 text-foreground">
                    {item.title}
                  </dt>
                  <dd className="text-sm leading-5 text-muted-foreground">
                    {item.detail}
                  </dd>
                </div>
              ))}
            </dl>
          ) : null}
        </>
      ) : (
        <p className="mt-1 max-w-full truncate whitespace-nowrap text-sm leading-5 text-muted-foreground">
          This is the beginning of your direct message with{" "}
          <span className="font-medium text-foreground">
            {intro.displayName}
          </span>
          .
        </p>
      )}
    </div>
  );
}
