import { openUrl } from "@tauri-apps/plugin-opener";
import { Globe } from "lucide-react";

import {
  socialLabel,
  type OrgSocial,
  type SocialNetwork,
} from "@/features/org/profile";

function SocialMark({ network }: { network: SocialNetwork }) {
  if (network === "website") {
    return <Globe aria-hidden="true" className="size-4" />;
  }
  return <BrandSvg network={network} />;
}

function BrandSvg({ network }: { network: Exclude<SocialNetwork, "website"> }) {
  if (network === "github") {
    return (
      <svg
        aria-hidden="true"
        className="size-4"
        fill="currentColor"
        viewBox="0 0 24 24"
      >
        <path d="M12 3.2a8.8 8.8 0 0 0-2.8 17.1c.4.1.6-.2.6-.4v-1.5c-2.4.5-2.9-1-2.9-1-.4-.9-.9-1.2-.9-1.2-.8-.5.1-.5.1-.5.8.1 1.3.9 1.3.9.7 1.3 1.9.9 2.4.7.1-.5.3-.9.5-1.1-1.9-.2-3.9-1-3.9-4.3 0-.9.3-1.7.9-2.3-.1-.2-.4-1.1.1-2.3 0 0 .7-.2 2.4.9a8 8 0 0 1 4.4 0c1.7-1.1 2.4-.9 2.4-.9.5 1.2.2 2.1.1 2.3.6.6.9 1.4.9 2.3 0 3.3-2 4.1-3.9 4.3.3.3.6.8.6 1.6v2.3c0 .2.2.5.6.4A8.8 8.8 0 0 0 12 3.2Z" />
      </svg>
    );
  }
  if (network === "x") {
    return (
      <svg
        aria-hidden="true"
        className="size-4"
        fill="currentColor"
        viewBox="0 0 24 24"
      >
        <path d="M14.7 3.5h2.6l-5.7 6.5 6.7 8.5h-5.2l-4.1-5.2-4.7 5.2H1.7l6.1-6.9L1.4 3.5h5.3l3.7 4.8 4.3-4.8Zm-.9 13.2h1.4L6.3 4.8H4.8l9 11.9Z" />
      </svg>
    );
  }
  if (network === "linkedin") {
    return (
      <svg
        aria-hidden="true"
        className="size-4"
        fill="currentColor"
        viewBox="0 0 24 24"
      >
        <path d="M6.7 9.2H4.2V19h2.5V9.2ZM5.4 4.2A1.5 1.5 0 1 0 5.5 7a1.5 1.5 0 0 0 0-2.8ZM19.8 19h-2.5v-5.2c0-1.5-.6-2.5-1.9-2.5-1 0-1.5.7-1.8 1.3-.1.2-.1.6-.1.9V19h-2.5s.1-8.6 0-9.8h2.5v1.6c.3-.6 1.2-1.8 3-1.8 2.1 0 3.3 1.4 3.3 4.3V19Z" />
      </svg>
    );
  }
  if (network === "instagram") {
    return (
      <svg
        aria-hidden="true"
        className="size-4"
        fill="none"
        viewBox="0 0 24 24"
      >
        <rect
          height="14"
          rx="4"
          stroke="currentColor"
          strokeWidth="1.8"
          width="14"
          x="5"
          y="5"
        />
        <circle
          cx="12"
          cy="12"
          r="3.2"
          stroke="currentColor"
          strokeWidth="1.8"
        />
        <circle cx="16.4" cy="7.6" fill="currentColor" r="0.9" />
      </svg>
    );
  }
  if (network === "youtube") {
    return (
      <svg
        aria-hidden="true"
        className="size-4"
        fill="currentColor"
        viewBox="0 0 24 24"
      >
        <path d="M20.6 8.2a2.4 2.4 0 0 0-1.7-1.7C17.4 6.2 12 6.2 12 6.2s-5.4 0-6.9.3a2.4 2.4 0 0 0-1.7 1.7A25 25 0 0 0 3 12a25 25 0 0 0 .4 3.8 2.4 2.4 0 0 0 1.7 1.7c1.5.3 6.9.3 6.9.3s5.4 0 6.9-.3a2.4 2.4 0 0 0 1.7-1.7A25 25 0 0 0 21 12a25 25 0 0 0-.4-3.8ZM10.4 14.8V9.2L15.2 12l-4.8 2.8Z" />
      </svg>
    );
  }
  if (network === "telegram") {
    return (
      <svg
        aria-hidden="true"
        className="size-4"
        fill="none"
        viewBox="0 0 24 24"
      >
        <path
          d="M20.5 4.5 3.8 10.9c-1.1.4-1.1 1.1-.2 1.4l4.3 1.3 1.7 5.1c.2.6.1.8.7.8.4 0 .6-.2.8-.4l2.4-2.3 4.3 3.2c.8.4 1.3.2 1.5-.7l2.7-12.8c.3-1.1-.4-1.6-1.5-1.2Z"
          fill="currentColor"
        />
      </svg>
    );
  }
  if (network === "discord") {
    return (
      <svg
        aria-hidden="true"
        className="size-4"
        fill="currentColor"
        viewBox="0 0 24 24"
      >
        <path d="M18.6 5.2A16 16 0 0 0 14.7 4l-.4.8a14 14 0 0 1 3.5.9 12 12 0 0 0-11.6 0A14 14 0 0 1 9.7 4.8L9.3 4a16 16 0 0 0-3.9 1.2C2.7 9.1 2 13 2.2 16.8A16 16 0 0 0 7.4 19l.8-1.1a10 10 0 0 1-1.3-.6l.3-.2c2.6 1.2 5.4 1.2 8 0l.3.2c-.4.3-.8.5-1.3.6l.8 1.1a16 16 0 0 0 5.2-2.2c.6-4.4-.6-8.2-1.6-11.6ZM9.2 14.7c-.9 0-1.6-.8-1.6-1.8s.7-1.8 1.6-1.8 1.7.8 1.6 1.8-.7 1.8-1.6 1.8Zm5.6 0c-.9 0-1.6-.8-1.6-1.8s.7-1.8 1.6-1.8 1.7.8 1.6 1.8-.7 1.8-1.6 1.8Z" />
      </svg>
    );
  }
  if (network === "bluesky") {
    return (
      <svg
        aria-hidden="true"
        className="size-4"
        fill="currentColor"
        viewBox="0 0 24 24"
      >
        <path d="M12 13.4c-1.6-3.1-6-8.8-8.4-8.2-1.6.4-1.3 3.4-.6 5.2.8 2.2 3.4 3 5.2 2.6-3.2.5-5.4 2.2-3.8 4.6 1.8 2.6 5.2.4 7.6-2.2 2.4 2.6 5.8 4.8 7.6 2.2 1.6-2.4-.6-4.1-3.8-4.6 1.8.4 4.4-.4 5.2-2.6.7-1.8 1-4.8-.6-5.2C18 4.6 13.6 10.3 12 13.4Z" />
      </svg>
    );
  }
  if (network === "mastodon") {
    return (
      <svg
        aria-hidden="true"
        className="size-4"
        fill="currentColor"
        viewBox="0 0 24 24"
      >
        <path d="M16.8 14.6v-2.2c0-2.6-1.7-3.4-1.7-3.4-.9-.4-2.4-.5-3.1-.5h-.1c-.7 0-2.2.1-3.1.5 0 0-1.7.8-1.7 3.4v3.6c.1 2.4 1.9 2.6 1.9 2.6 1.8.2 3.4 0 3.4 0l.1-1.2s-1.3.1-2.7 0c-.9-.1-1.9-.2-2-.9v-2.1h4.9v.6c0 .8-.1 1.8-.1 1.8h1.5s.1-1 .1-1.8v-.6h.6ZM8.4 10.2c.5 0 .8.6.8 1.4s-.3 1.4-.8 1.4-.9-.6-.9-1.4.4-1.4.9-1.4Zm3.2 0c.5 0 .8.6.8 1.4s-.3 1.4-.8 1.4-.9-.6-.9-1.4.4-1.4.9-1.4Z" />
      </svg>
    );
  }
  return (
    <svg aria-hidden="true" className="size-4" fill="none" viewBox="0 0 24 24">
      <path
        d="M7 16.5c2.2-4.2 4.2-7.2 5-9.5.8 2.3 2.8 5.3 5 9.5"
        stroke="currentColor"
        strokeLinecap="round"
        strokeWidth="1.8"
      />
      <circle cx="12" cy="6.2" fill="currentColor" r="1.3" />
      <path
        d="M8.2 14.2h7.6"
        stroke="currentColor"
        strokeLinecap="round"
        strokeWidth="1.8"
      />
    </svg>
  );
}

function openSocial(url: string) {
  void openUrl(url).catch(() => {
    window.open(url, "_blank", "noopener,noreferrer");
  });
}

/** Logo buttons for the social links on a member's profile. */
export function ProfileSocialLinks({
  socials,
}: {
  socials: readonly OrgSocial[];
}) {
  if (socials.length === 0) return null;
  return (
    <ul className="mt-3 flex flex-wrap gap-1.5">
      {socials.map((social) => (
        <li key={`${social.network}:${social.url}`}>
          <button
            aria-label={socialLabel(social.network)}
            className="inline-flex size-8 items-center justify-center rounded-lg text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
            onClick={() => openSocial(social.url)}
            type="button"
          >
            <SocialMark network={social.network} />
          </button>
        </li>
      ))}
    </ul>
  );
}
