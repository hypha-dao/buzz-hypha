/**
 * The intelligent organization's transparency notice on the invite landing
 * page (Features 6a, Protocol §6.6, Readiness D7).
 *
 * The text is the relay operator's, delivered by `GET /api/join-policy` as
 * `org.transparency_notice` only for a community that has an org agent or a
 * bootstrapped Shaper set; the page renders it verbatim and never edits or
 * hides it. It is information the newcomer must have before joining, not a
 * consent — nothing here gates the join buttons.
 */
export function InviteOrgTransparencyNotice({ notice }: { notice: string }) {
  const paragraphs = notice
    .split(/\n\s*\n/)
    .map((paragraph) => paragraph.replace(/\s*\n\s*/g, " ").trim())
    .filter((paragraph) => paragraph.length > 0);

  return (
    <section
      aria-labelledby="invite-org-transparency-heading"
      className="w-full space-y-2 rounded-xl border border-black/10 bg-black/[0.03] p-4 text-left"
      data-testid="invite-org-transparency-notice"
    >
      <h2
        className="text-xs font-semibold uppercase tracking-wide text-black/70"
        id="invite-org-transparency-heading"
      >
        Before you join
      </h2>
      {paragraphs.map((paragraph) => (
        <p className="text-xs leading-5 text-black/60" key={paragraph}>
          {paragraph}
        </p>
      ))}
    </section>
  );
}
