import { X } from "lucide-react";
import * as React from "react";
import { toast } from "sonner";

import { useOrgProfile } from "@/features/org/hooks";
import {
  addSkillLabel,
  buildValidatedProfileSet,
  PROFILE_ABOUT_MAX_CHARS,
  PROFILE_MAX_SKILLS,
  PROFILE_OPEN_LIMIT_MAX,
  PROFILE_OPEN_LIMIT_MIN,
  PROFILE_SKILL_LABEL_MAX_CHARS,
} from "@/features/org/profile";
import { useOrgCommands } from "@/features/org/useOrgCommands";
import { useFeatureEnabled } from "@/shared/features/useFeatureEnabled";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { PanelSectionGroup } from "@/shared/ui/PanelSectionGroup";
import { Textarea } from "@/shared/ui/textarea";

export function OrgAboutSkillsSection({
  isSelf,
  pubkey,
}: {
  isSelf: boolean;
  pubkey: string | null;
}) {
  const orgEnabled = useFeatureEnabled("org");
  if (!orgEnabled || !pubkey) return null;
  return <OrgAboutSkillsBody isSelf={isSelf} pubkey={pubkey} />;
}

function OrgAboutSkillsBody({
  isSelf,
  pubkey,
}: {
  isSelf: boolean;
  pubkey: string;
}) {
  const headingId = React.useId();
  const aboutId = React.useId();
  const skillId = React.useId();
  const limitId = React.useId();
  const errorId = React.useId();
  const { profile } = useOrgProfile(pubkey);
  const commands = useOrgCommands();

  const [about, setAbout] = React.useState(profile.about);
  const [skills, setSkills] = React.useState(
    profile.skills.map((skill) => skill.label),
  );
  const [skillDraft, setSkillDraft] = React.useState("");
  const [openLimit, setOpenLimit] = React.useState(
    profile.openLimit === null ? "" : String(profile.openLimit),
  );
  const [error, setError] = React.useState<string | null>(null);
  const [saving, setSaving] = React.useState(false);

  React.useEffect(() => {
    setAbout(profile.about);
    setSkills(profile.skills.map((skill) => skill.label));
    setOpenLimit(profile.openLimit === null ? "" : String(profile.openLimit));
  }, [profile]);

  const addSkill = React.useCallback(() => {
    const result = addSkillLabel(skills, skillDraft);
    if (!result.ok) {
      setError(result.error);
      return;
    }
    setSkills(result.skills);
    setSkillDraft("");
    setError(null);
  }, [skillDraft, skills]);

  const removeSkill = React.useCallback((label: string) => {
    setSkills((current) => current.filter((skill) => skill !== label));
    setError(null);
  }, []);

  const onSubmit = React.useCallback(
    (event: React.FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      const built = buildValidatedProfileSet({ about, openLimit, skills });
      if (!built.ok) {
        setError(built.error);
        return;
      }
      setError(null);
      setSaving(true);
      void commands
        .publish(built.command)
        .catch((publishError: unknown) => {
          const message =
            publishError instanceof Error
              ? publishError.message
              : "Failed to save About & skills.";
          setError(message);
          toast.error(message);
        })
        .finally(() => {
          setSaving(false);
        });
    },
    [about, commands, openLimit, skills],
  );

  return (
    <section aria-labelledby={headingId} data-testid="org-about-skills">
      <PanelSectionGroup
        testId="org-about-skills-section"
        title={<span id={headingId}>About & skills</span>}
      >
        {isSelf ? (
          <form
            aria-describedby={error ? errorId : undefined}
            className="divide-y divide-border/55"
            data-testid="org-about-skills-form"
            onSubmit={onSubmit}
          >
            <div className="space-y-2 px-4 py-3">
              <label
                className="block text-sm font-medium text-foreground"
                htmlFor={aboutId}
              >
                About
              </label>
              <Textarea
                aria-describedby={`${aboutId}-hint`}
                data-testid="org-about-skills-about"
                id={aboutId}
                maxLength={PROFILE_ABOUT_MAX_CHARS}
                onChange={(event) => {
                  setAbout(event.target.value);
                  setError(null);
                }}
                rows={4}
                value={about}
              />
              <p
                className="text-2xs text-muted-foreground"
                id={`${aboutId}-hint`}
              >
                {about.length}/{PROFILE_ABOUT_MAX_CHARS}
              </p>
            </div>
            <div className="space-y-2 px-4 py-3">
              <label
                className="block text-sm font-medium text-foreground"
                htmlFor={skillId}
              >
                Skills
              </label>
              <SkillChips labels={skills} onRemove={removeSkill} removable />
              <div className="flex items-center gap-2">
                <Input
                  aria-describedby={`${skillId}-hint`}
                  autoComplete="off"
                  data-testid="org-about-skills-skill-input"
                  disabled={skills.length >= PROFILE_MAX_SKILLS}
                  id={skillId}
                  maxLength={PROFILE_SKILL_LABEL_MAX_CHARS}
                  onChange={(event) => {
                    setSkillDraft(event.target.value);
                    setError(null);
                  }}
                  onKeyDown={(event) => {
                    if (event.key !== "Enter") return;
                    event.preventDefault();
                    addSkill();
                  }}
                  value={skillDraft}
                />
                <Button
                  aria-label="Add skill"
                  data-testid="org-about-skills-skill-add"
                  disabled={skills.length >= PROFILE_MAX_SKILLS}
                  onClick={addSkill}
                  type="button"
                  variant="outline"
                >
                  Add
                </Button>
              </div>
              <p
                className="text-2xs text-muted-foreground"
                id={`${skillId}-hint`}
              >
                Up to {PROFILE_MAX_SKILLS} skills,{" "}
                {PROFILE_SKILL_LABEL_MAX_CHARS} characters each. Enter adds a
                chip.
              </p>
            </div>
            <div className="space-y-2 px-4 py-3">
              <label
                className="block text-sm font-medium text-foreground"
                htmlFor={limitId}
              >
                Open limit
              </label>
              <Input
                aria-describedby={`${limitId}-hint`}
                data-testid="org-about-skills-open-limit"
                id={limitId}
                inputMode="numeric"
                max={PROFILE_OPEN_LIMIT_MAX}
                min={PROFILE_OPEN_LIMIT_MIN}
                onChange={(event) => {
                  setOpenLimit(event.target.value);
                  setError(null);
                }}
                type="number"
                value={openLimit}
              />
              <p
                className="text-2xs text-muted-foreground"
                id={`${limitId}-hint`}
              >
                How many open pieces at once ({PROFILE_OPEN_LIMIT_MIN}–
                {PROFILE_OPEN_LIMIT_MAX}). Leave empty for no limit.
              </p>
            </div>
            {error ? (
              <p
                className="px-4 py-3 text-sm text-destructive"
                data-testid="org-about-skills-error"
                id={errorId}
                role="alert"
              >
                {error}
              </p>
            ) : null}
            <div className="flex justify-end px-4 py-3">
              <Button
                aria-label="Save About & skills"
                data-testid="org-about-skills-save"
                disabled={saving}
                type="submit"
              >
                Save
              </Button>
            </div>
          </form>
        ) : (
          <OrgAboutSkillsReadOnly
            about={profile.about}
            openLimit={profile.openLimit}
            skills={profile.skills.map((skill) => skill.label)}
          />
        )}
      </PanelSectionGroup>
    </section>
  );
}

function OrgAboutSkillsReadOnly({
  about,
  openLimit,
  skills,
}: {
  about: string;
  openLimit: number | null;
  skills: string[];
}) {
  return (
    <div
      className="divide-y divide-border/55"
      data-testid="org-about-skills-readonly"
    >
      <div className="space-y-1 px-4 py-3">
        <h3 className="text-sm font-medium text-foreground">About</h3>
        <p
          className="whitespace-pre-wrap text-sm text-muted-foreground"
          data-testid="org-about-skills-about-text"
        >
          {about.length > 0 ? about : "Not set yet."}
        </p>
      </div>
      <div className="space-y-2 px-4 py-3">
        <h3 className="text-sm font-medium text-foreground">Skills</h3>
        {skills.length > 0 ? (
          <SkillChips labels={skills} />
        ) : (
          <p className="text-sm text-muted-foreground">Not set yet.</p>
        )}
      </div>
      <div className="space-y-1 px-4 py-3">
        <h3 className="text-sm font-medium text-foreground">Open limit</h3>
        <p
          className="text-sm text-muted-foreground"
          data-testid="org-about-skills-open-limit-text"
        >
          {openLimit === null ? "No limit" : String(openLimit)}
        </p>
      </div>
    </div>
  );
}

function SkillChips({
  labels,
  onRemove,
  removable = false,
}: {
  labels: readonly string[];
  onRemove?: (label: string) => void;
  removable?: boolean;
}) {
  if (labels.length === 0) return null;
  return (
    <ul
      aria-label="Added skills"
      className="flex flex-wrap gap-1.5"
      data-testid="org-about-skills-chips"
    >
      {labels.map((label) => (
        <li key={label}>
          <span
            className="inline-flex items-center gap-1 rounded-full border border-border/70 bg-muted/40 px-2.5 py-1 text-xs text-foreground"
            data-testid={`org-about-skills-chip-${label}`}
          >
            {label}
            {removable && onRemove ? (
              <button
                aria-label={`Remove ${label}`}
                className="rounded-full p-0.5 text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring"
                data-testid={`org-about-skills-remove-${label}`}
                onClick={() => onRemove(label)}
                type="button"
              >
                <X aria-hidden="true" className="h-3 w-3" />
              </button>
            ) : null}
          </span>
        </li>
      ))}
    </ul>
  );
}
