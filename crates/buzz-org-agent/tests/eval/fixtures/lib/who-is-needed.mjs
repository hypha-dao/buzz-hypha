// Who-is-needed fixtures (AI evaluation § Test data): for each requirement
// the constants name, who in the seed carries the skill and how many open
// pieces they hold — so `requires`, `matched`, and `unfilled` each have a
// case where they are the only correct answer. The expectations are
// checked against the seed here, not asserted by hand.

import { personKey } from "./emit.mjs";

const OPEN_STATES = new Set(["accepted", "in_review"]);

export function whoIsNeeded(spec, { org }) {
  const requirements = [];
  for (const [requires, expect] of Object.entries(spec.whoIsNeeded)) {
    const candidates = [];
    for (const [pubkey, profile] of org.profiles) {
      if (!profile.skills.some((s) => s.slug === requires)) continue;
      const open = [...org.items.values()].filter((i) => i.dri === pubkey && OPEN_STATES.has(i.state));
      const name = spec.members.find((n) => personKey(n).pubkey === pubkey);
      candidates.push({
        name,
        pubkey,
        open_limit: profile.open_limit,
        open_items: open.map((i) => i.id),
        at_limit: profile.open_limit !== null && open.length >= profile.open_limit,
      });
    }
    candidates.sort((a, b) => a.name.localeCompare(b.name));
    const want = { one: 1, two: 2, nobody: 0 }[expect];
    if (candidates.length !== want) {
      throw new Error(`${spec.id}: ${requires} expected ${expect}, found ${candidates.map((c) => c.name).join(", ") || "nobody"}`);
    }
    if (expect === "two" && candidates.filter((c) => c.at_limit).length !== 1) {
      throw new Error(`${spec.id}: ${requires} needs exactly one candidate at open_limit`);
    }
    const free = candidates.filter((c) => !c.at_limit);
    requirements.push({
      requires,
      expect,
      candidates,
      // What a correct ticket draft says for a piece that `requires` this.
      gold:
        want === 0
          ? { suggested_holder: null, unfilled: `nobody here has ${requires.replaceAll("-", " ")}` }
          : { suggested_holder: free[0]?.pubkey ?? null, matched_skills: [requires] },
    });
  }
  return { org: spec.id, requirements };
}
