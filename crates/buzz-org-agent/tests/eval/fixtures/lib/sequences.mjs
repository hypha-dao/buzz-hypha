// Sequence fixtures (AI evaluation § Test data): multi-step briefs whose
// order matters, written down in `why_gold`, each shipped as a series of
// snapshots — before the gate, the gate accepted and held, gate done with
// outcome A, gate done with outcome B — so the "next wave" and "outcome
// changes the plan" cases replay from real state, not from a described one.
//
// A snapshot is the org's seed plus the deltas `sequence.json` lists for it,
// in order; every delta event is younger than the seed. The seeds stay
// faithful to `data.ts`, so the multi-step brief is a ticket the sequence
// adds under the named root, held by the person the case names. The gate
// stage is the record of the "before" gold having happened: the agent's
// drafts (their `coverage` is the plan), the holder accepting them into
// tickets, the gate offered and taken. Each outcome is the gate holder's
// message in the project's home room and the agent's `io_done` citing it
// (Protocol §5.5), so the outcome the next wave must read is in the ledger.

import { ENERGY, RIVER } from "./constants.mjs";
import { DAY, NOW } from "./dates.mjs";
import { AGENT, personKey, slugify } from "./emit.mjs";
import { buildOrg } from "./world.mjs";

const HOUR = 3600;
const MINUTE = 60;

// ── the four sequences ───────────────────────────────────────────────────────

const WEEKDAY_HALL = {
  name: "weekday-hall",
  spec: RIVER,
  // The seed leaves the hall an open proposal; the Shapers pass it and Lea,
  // its suggested DRI, accepts. Her first ticket carries the four-piece brief.
  before(ctx) {
    passWeekday(ctx);
    holdTicket(ctx, {
      key: "weekday/open-the-hall-on-weekdays",
      parentKey: "weekday",
      title: "Open the hall on weekdays",
      brief:
        "Get the council licence for weekday use, take out the hall insurance, print a weekday rota, and plan the opening night. Nothing opens without the licence, and its conditions set the rota.",
      holder: "Lea",
    });
  },
  parentKey: "weekday/open-the-hall-on-weekdays",
  homeRoom: "home-weekday",
  gate: "licence",
  plan: [
    {
      key: "licence",
      piece: "the council licence for weekday use",
      title: "Council licence for weekday use",
      brief:
        "Apply to the council for a weekday-use licence for the hall. Ask for evenings; write down whatever conditions come back — hours, capacity, noise — because the rota and the opening night follow them.",
      requires: ["council-licensing"],
      suggested: "Rafi",
      gate: true,
      after: [],
    },
    {
      key: "insurance",
      piece: "the hall insurance",
      title: "Hall insurance",
      brief:
        "Get public-liability cover for the hall on the days we use it. Independent of the licence — the insurer needs the address and the days, nothing more.",
      requires: ["permits-and-insurance"],
      suggested: "Rafi",
      after: [],
    },
    {
      key: "rota",
      piece: "a weekday rota",
      title: "Weekday rota",
      brief: "Print the rota for the weekday hall: who opens, who closes, within the hours the licence allows.",
      requires: ["printing"],
      suggested: "Jun",
      after: ["licence"],
    },
    {
      key: "opening",
      piece: "the opening night",
      title: "Opening night",
      brief: "Plan the first weekday evening: invite the growers and the Saturday regulars, within the licence hours.",
      requires: ["hosting-events"],
      suggested: "Tom",
      after: ["licence"],
    },
  ],
  why_gold:
    "No licence, no opening — and the licence conditions (hours, capacity) may change the rota, so the rota waits for the licence too. Insurance depends on nothing but the address and the days, so it starts now. Two drafts, not four.",
  // Lea accepts both drafts; the licence goes to Rafi, the insurance stays open.
  gate_offers: { licence: "Rafi", insurance: null },
  outcomes: {
    a: {
      said: "Licence granted — weekdays only, until 22:00. Conditions attached: capacity sixty, no amplified music.",
      next_wave: [
        { key: "rota", after: ["licence"], must_mention: ["22:00"] },
        { key: "opening", after: ["licence"], must_mention: ["22:00"] },
      ],
      held: [],
      not_redrafted: ["licence", "insurance"],
      fails: ["a rota or opening night that runs past 22:00", "a second licence or insurance draft"],
      why_gold:
        "The gate answered: the hall is ours on weekdays until 22:00, so both held pieces can start, each after the licence, each inside its hours. The insurance is live and the licence is done; neither is drafted again.",
    },
    b: {
      said: "Licence refused for weekday evenings — noise complaints from the flats. The council will look at daytime, 09:00 to 17:00, on a fresh application.",
      next_wave: [
        {
          key: "licence-daytime",
          piece: "a daytime licence",
          title: "Reapply for a daytime licence (09:00–17:00)",
          requires: ["council-licensing"],
          gate: true,
          after: ["licence"],
          replaces: "licence",
        },
      ],
      held: ["rota", "opening"],
      not_redrafted: ["insurance"],
      fails: ["a rota or opening night drafted as if evenings were granted", "an opening night at all before a licence exists"],
      why_gold:
        "The refusal changes the plan, not just the timing: an evening rota and an opening night no longer follow from anything. The next piece is a re-scoped gate — the daytime application — and the old held pieces stay held behind it.",
    },
  },
};

const HALL_ELECTRICS = {
  name: "hall-electrics",
  spec: RIVER,
  before(ctx) {
    passWeekday(ctx);
    holdTicket(ctx, {
      key: "weekday/hall-electrics",
      parentKey: "weekday",
      title: "Hall electrics",
      brief:
        "The hall's wiring was last certified in 2019. Get the electrics certified first — the electrician's report says how much rewiring the hall needs — then do the rewiring, then book the council's inspection of the finished work before we open.",
      holder: "Noor",
    });
  },
  parentKey: "weekday/hall-electrics",
  homeRoom: "home-weekday",
  gate: "certification",
  plan: [
    {
      key: "certification",
      piece: "get the electrics certified",
      title: "Electrical certification",
      brief: "Book a certified electrician to test the hall's wiring and issue the report. The report decides the rewiring scope.",
      requires: ["electrical-certification"],
      suggested: null,
      unfilled: "nobody here holds an electrical certification",
      gate: true,
      after: [],
    },
    {
      key: "rewiring",
      piece: "do the rewiring",
      title: "Rewiring",
      brief: "Rewire what the report names.",
      requires: ["electrical-work"],
      suggested: null,
      unfilled: "nobody here is an electrician",
      after: ["certification"],
    },
    {
      key: "inspection",
      piece: "the council's inspection of the finished work",
      title: "Council inspection",
      brief: "Book the council's inspection of the finished work and be there for it.",
      requires: ["council-licensing"],
      suggested: "Rafi",
      after: ["rewiring"],
    },
  ],
  why_gold:
    "The report is the scope: until an electrician has certified — or failed — the wiring, nobody knows whether the rewiring is one circuit or the whole hall, and the council inspects finished work, not plans. One draft, and it names what River lacks: nobody here holds the certification.",
  // Noor takes the certification to Rafi anyway — he knows a contractor.
  gate_offers: { certification: "Rafi" },
  outcomes: {
    a: {
      said: "Electrician's report is in: the hall passes except the kitchen circuit, which has to be rewired before the council will sign it off.",
      next_wave: [{ key: "rewiring", after: ["certification"], must_mention: ["kitchen"] }],
      held: ["inspection"],
      not_redrafted: ["certification"],
      fails: ["a whole-hall rewire", "an inspection drafted before the rewiring exists"],
      why_gold:
        "The report scoped the rewiring to one circuit, so the next piece is that circuit and says so; the inspection still waits on finished work. Nobody here is an electrician, and the draft says that rather than naming a handy member.",
    },
    b: {
      said: "Report is in and it is bad: the whole hall fails, and the electrician will not touch the rewire until a structural engineer has checked the ceiling void.",
      next_wave: [
        {
          key: "structural-check",
          piece: "a structural check of the ceiling void",
          title: "Structural check of the ceiling void",
          requires: ["structural-engineering"],
          gate: true,
          after: ["certification"],
        },
      ],
      held: ["rewiring", "inspection"],
      not_redrafted: ["certification"],
      fails: ["a kitchen-only rewire", "any rewiring draft before the structural check"],
      why_gold:
        "The outcome added a step the brief never named: a structural check now gates the rewire. Drafting the rewiring — of any size — ignores what the report said; the next piece is the new gate, and the rest of the plan moves behind it.",
    },
  },
};

const IBERIA_PILOT = {
  name: "iberia-pilot",
  spec: ENERGY,
  // Iberia is live with Pedro as DRI; he takes the second Spanish pilot himself.
  before(ctx) {
    holdTicket(ctx, {
      key: "iberia/pilot-two-andalusia",
      parentKey: "iberia",
      title: "Pilot two — Andalusia",
      brief:
        "Pick the pilot site from the three candidate roofs in Alcalá la Real, sign the landowner, order the inverters — the count depends on the roof — install, and commission with the DSO.",
      holder: "Pedro",
    });
  },
  parentKey: "iberia/pilot-two-andalusia",
  homeRoom: "home-iberia",
  gate: "site",
  plan: [
    {
      key: "site",
      piece: "pick the pilot site",
      title: "Pick the pilot site",
      brief:
        "Survey the three candidate roofs and pick one. Everything after this depends on which roof: who owns it, how many inverters it takes, which feeder the DSO connects.",
      requires: ["municipal-relations"],
      suggested: "Pedro",
      gate: true,
      after: [],
    },
    {
      key: "landowner",
      piece: "sign the landowner",
      title: "Landowner agreement",
      brief: "Draft and sign the roof agreement with whoever owns the chosen roof.",
      requires: ["spanish-energy-law"],
      suggested: "Diego",
      after: ["site"],
    },
    {
      key: "inverters",
      piece: "order the inverters",
      title: "Order the inverters",
      brief: "Order the inverters the chosen roof needs.",
      requires: ["grid-protocols"],
      suggested: "Tomas",
      after: ["landowner"],
    },
    {
      key: "install",
      piece: "install",
      title: "Installation",
      brief: "Install panels and inverters on the signed roof.",
      requires: ["electrical-certification"],
      suggested: "Tomas",
      after: ["inverters"],
    },
    {
      key: "commission",
      piece: "commission with the DSO",
      title: "Commissioning with the DSO",
      brief: "Commission the installation with the DSO and get the meter live.",
      requires: ["grid-protocols"],
      suggested: "Tomas",
      after: ["install"],
    },
  ],
  why_gold:
    "The site is the gate: the landowner is whoever owns the chosen roof, the inverter count is the roof's size, and the DSO connection is the roof's feeder. One draft now; a five-piece batch orders inverters for a roof nobody has picked.",
  gate_offers: { site: "Pedro" },
  outcomes: {
    a: {
      said: "Site picked: the school roof, 38 kW usable. The landowner is the municipality, so the agreement goes through the council, not a private owner.",
      next_wave: [{ key: "landowner", after: ["site"], must_mention: ["municipality"] }],
      held: ["inverters", "install", "commission"],
      not_redrafted: ["site"],
      fails: ["an inverter order before the roof is signed", "a private-landowner agreement"],
      why_gold:
        "The site answered who the landowner is — the municipality — so the agreement is the next piece and is written for a council. The inverter count is now knowable but the roof is not yet ours; inverters, install, and commissioning stay held in order.",
    },
    b: {
      said: "None of the three roofs passed the structural survey. The only option left is a ground-mount on municipal land by the depot — different permits, different inverter count, and we start the site search again.",
      next_wave: [
        {
          key: "ground-site",
          piece: "municipal land for a ground-mount",
          title: "Secure municipal land for a ground-mount",
          requires: ["municipal-relations"],
          gate: true,
          after: ["site"],
          replaces: "site",
        },
      ],
      held: ["landowner", "inverters", "install", "commission"],
      not_redrafted: [],
      fails: ["a landowner agreement for a roof", "any inverter order"],
      why_gold:
        "The gate came back with no site, so nothing downstream can start; the plan restarts with a re-scoped gate — municipal land for a ground-mount — and every later piece stays held behind it.",
    },
  },
};

const ANDALUSIA = {
  name: "andalusia",
  spec: ENERGY,
  // The depth fixture: a sixth level under the Andalusia branch of the
  // playbook tree, with one piece already covered by a live item elsewhere.
  before(ctx) {
    holdTicket(ctx, {
      key: "playbook/andalusia-annex",
      parentKey:
        "playbook/legal-templates-per-country/spain-comunidad-energetica-template/check-the-andalusia-regional-variant",
      title: "Andalusia annex for the Spain template",
      brief:
        "Once §4 is translated, get the Junta's legal desk to confirm our reading of the self-consumption clause; then draft the annex and fold it into the Spain template. The reading decides whether the annex is two paragraphs or a separate chapter.",
      holder: "Diego",
    });
  },
  parentKey: "playbook/andalusia-annex",
  homeRoom: "home-playbook",
  gate: "reading",
  plan: [
    {
      key: "translation",
      piece: "§4 is translated",
      coveredBy:
        "playbook/legal-templates-per-country/spain-comunidad-energetica-template/check-the-andalusia-regional-variant/get-the-junta-s-2026-decree-text/translate-4-self-consumption-for-the-lawyers",
      after: [],
    },
    {
      key: "reading",
      piece: "confirm our reading of the self-consumption clause",
      title: "Confirm the §4 reading with the Junta",
      brief:
        "Ask the Junta's legal desk whether §4 self-consumption applies to comunidades energéticas as written. The answer sizes the annex.",
      requires: ["spanish-energy-law"],
      suggested: "Diego",
      gate: true,
      after: ["translation"],
    },
    {
      key: "annex",
      piece: "draft the annex",
      title: "Draft the Andalusia annex",
      brief: "Write the annex at the size the Junta's answer calls for.",
      requires: ["spanish-energy-law", "legal-templates"],
      suggested: "Diego",
      after: ["reading"],
    },
    {
      key: "merge",
      piece: "fold it into the Spain template",
      title: "Merge the annex into the Spain template",
      brief: "Fold the annex into the Spain template and check the cross-references.",
      requires: ["legal-templates"],
      suggested: "Diego",
      after: ["annex"],
    },
  ],
  why_gold:
    "The translation is already live under the decree ticket, so it is covered, not drafted. The Junta's reading is the gate — it decides whether the annex is two paragraphs or a chapter — so drafting the annex now writes the wrong document. One draft, its `after` pointing at the live translation.",
  gate_offers: { reading: "Diego" },
  outcomes: {
    a: {
      said: "Junta confirms our reading: §4 applies to comunidades as written. The annex is two paragraphs on the regional registry, nothing more.",
      next_wave: [{ key: "annex", after: ["reading"], must_mention: ["two paragraphs"] }],
      held: ["merge"],
      not_redrafted: ["translation", "reading"],
      fails: ["a chapter-length annex", "a merge drafted before the annex exists"],
      why_gold: "The reading sized the annex — two paragraphs — so that is the piece; the merge still waits on an annex that exists.",
    },
    b: {
      said: "The Junta says no — Andalusia puts shared self-consumption under its own 2026 regime. The annex becomes a separate chapter, and §3 of the Spain template has to change with it.",
      next_wave: [{ key: "annex", after: ["reading"], must_mention: ["chapter"] }],
      held: ["merge"],
      not_redrafted: ["translation", "reading"],
      elsewhere: [
        {
          piece: "§3 of the Spain template",
          under: "playbook/legal-templates-per-country/spain-comunidad-energetica-template",
          why: "a change to the Spain template belongs under the Spain template, whose holder is Diego, not under the annex",
        },
      ],
      fails: ["a two-paragraph annex", "the §3 change drafted under the annex"],
      why_gold:
        "The answer changed the annex's shape — a chapter, not two paragraphs — and named a change outside this brief. The next piece is the re-scoped annex; the §3 change is a piece under a different parent and is not drafted here.",
    },
  },
};

const SEQUENCES = [WEEKDAY_HALL, HALL_ELECTRICS, IBERIA_PILOT, ANDALUSIA];

// ── stages ───────────────────────────────────────────────────────────────────

// The seed's open `project-weekday` proposal passes (Maya, Sam) and Lea, the
// suggested DRI, accepts the root; the relay opened its home room on pass.
function passWeekday(ctx) {
  const { org } = ctx;
  if (org.items.get(ctx.byKey.get("weekday"))?.state === "accepted") return;
  const proposal = org.proposalByKey("project-weekday");
  org.decide(proposal, { yes: 2, agreedBy: ["Maya", "Sam"], at: tick(ctx) });
  ctx.t = proposal.decided_at;
  org.accept(ctx.byKey.get("weekday"), tick(ctx));
}

// The parent's holder creates the multi-step ticket and offers it to
// `holder`, who accepts — the trigger the "before" gold answers.
function holdTicket(ctx, { key, parentKey, title, brief, holder }) {
  const { org } = ctx;
  const parentId = ctx.byKey.get(parentKey);
  if (!parentId) throw new Error(`no item ${parentKey}`);
  const parent = org.items.get(parentId);
  const { item } = org.createTicket({
    key,
    parentId,
    title,
    brief,
    dueAt: parent.due_at,
    offerTo: holder,
    at: tick(ctx),
  });
  ctx.byKey.set(key, item.id);
  org.accept(item.id, tick(ctx));
}

// The "before" gold happens: the agent drafts the pieces that can start
// now, the holder accepts each into a ticket, and the gate is offered and
// taken.
function gateStage(ctx, def) {
  const { org } = ctx;
  const parentId = ctx.byKey.get(def.parentKey);
  ctx.t = NOW + 2 * DAY;
  const drafts = draftWave(ctx, def, parentId, def.plan, tick(ctx));
  ctx.drafts = {};
  ctx.gateItems = {};
  for (const [key, draft] of Object.entries(drafts)) {
    ctx.drafts[key] = draft.id;
    const piece = def.plan.find((p) => p.key === key);
    const offerTo = def.gate_offers[key] ?? null;
    const { item } = org.createTicket({
      key: `${def.parentKey}/${slugify(piece.title)}`,
      parentId,
      title: piece.title,
      brief: piece.brief,
      dueAt: org.items.get(parentId).due_at,
      offerTo,
      after: piece.after.map((k) => ctx.byKey.get(itemKeyFor(def, k))).filter(Boolean),
      at: tick(ctx),
      draft,
    });
    ctx.byKey.set(itemKeyFor(def, key), item.id);
    ctx.gateItems[key] = item.id;
    if (offerTo) org.accept(item.id, tick(ctx));
  }
}

// The gate holder says what came back, in the project's home room, and the
// agent relays the done (§5.5). Nothing else moves: the next wave is the
// case's expected output, not fixture state.
function outcomeStage(ctx, def, outcome, label) {
  const { org } = ctx;
  const gateId = ctx.gateItems[def.gate];
  const gate = org.items.get(gateId);
  const holder = nameOf(ctx, gate.dri);
  ctx.t = NOW + 9 * DAY;
  const msg = org.message(def.homeRoom, `${def.name}-${label}`, holder, outcome.said, tick(ctx));
  org.done(gateId, tick(ctx), { receipt: msg, signer: AGENT });
  ctx.outcomeReceipt = msg.id;
}

// ── the agent's drafts, as the relay would store them ────────────────────────

// Which item key a plan piece's ticket has (or the live item it is covered by).
function itemKeyFor(def, key) {
  const piece = def.plan.find((p) => p.key === key);
  if (piece?.coveredBy) return piece.coveredBy;
  return `${def.parentKey}/${slugify(piece.title)}`;
}

function coverage(ctx, def, plan) {
  return plan.map((p, i) => ({
    piece: p.piece,
    covered_by: ctx.byKey.get(itemKeyFor(def, p.key)) ?? null,
    order: i + 1,
    after: p.after.map((k) => plan.find((q) => q.key === k).piece),
    held: heldBy(ctx, def, p),
  }));
}

// A piece is held when a predecessor is neither live nor done.
function heldBy(ctx, def, piece) {
  const missing = piece.after.find((k) => !ctx.byKey.has(itemKeyFor(def, k)));
  return missing ? `after ${def.plan.find((q) => q.key === missing).piece}` : null;
}

function matched(ctx, name, requires) {
  const pk = personKey(name).pubkey;
  const profile = ctx.org.profiles.get(pk);
  return {
    skills: (profile?.skills ?? []).map((s) => s.slug).filter((s) => requires.includes(s)),
    about: null,
    items: [...ctx.org.items.values()].filter((i) => i.dri === pk).map((i) => i.id),
  };
}

// One `50100 t=ticket` per piece that can start now: not covered, not held.
function draftWave(ctx, def, parentId, plan, at) {
  const { org } = ctx;
  const parent = org.items.get(parentId);
  const holder = nameOf(ctx, parent.dri);
  const list = coverage(ctx, def, plan);
  const out = {};
  let t = at;
  for (const piece of plan) {
    const row = list.find((c) => c.piece === piece.piece);
    if (row.covered_by || row.held) continue;
    const payload = {
      parent: parentId,
      title: piece.title,
      brief: piece.brief,
      due_at: parent.due_at,
      requires: piece.requires,
      suggested_holder: piece.suggested ? personKey(piece.suggested).pubkey : null,
      unfilled: piece.suggested ? null : piece.unfilled,
      covers: piece.piece,
      after: piece.after.map((k) => ctx.byKey.get(itemKeyFor(def, k))).filter(Boolean),
      gate: Boolean(piece.gate),
      coverage: list,
    };
    if (piece.suggested) payload.matched = matched(ctx, piece.suggested, piece.requires);
    out[piece.key] = org.draft({
      kind: "ticket",
      needs: holder,
      move: 2,
      origin: "gap",
      gap: `${parentId}#${slugify(piece.piece)}`,
      parent: parentId,
      suggested: piece.suggested,
      skills: payload.matched?.skills ?? [],
      receipts: [org.itemReceipt(parentId)],
      payload,
      at: t,
    });
    t += MINUTE;
  }
  ctx.t = t;
  return out;
}

// ── plumbing ─────────────────────────────────────────────────────────────────

function tick(ctx) {
  ctx.t += MINUTE;
  return ctx.t;
}

function nameOf(ctx, pubkey) {
  const hit = [...ctx.spec.members, "You"].find((n) => personKey(n).pubkey === pubkey);
  if (!hit) throw new Error(`unknown pubkey ${pubkey}`);
  return hit;
}

function sortEvents(events) {
  return events
    .map((ev, i) => [ev, i])
    .sort((a, b) => a[0].created_at - b[0].created_at || a[1] - b[1])
    .map(([ev]) => ev);
}

// Build the org fresh and run `stages` on it, cutting a delta per stage.
function run(def, stages) {
  const built = buildOrg(def.spec);
  const ctx = { ...built, spec: def.spec, byKey: built.world.byKey, t: NOW + DAY - HOUR };
  const seedMax = Math.max(...ctx.org.events.map((e) => e.created_at));
  const deltas = [];
  for (const stage of stages) {
    const from = ctx.org.events.length;
    stage(ctx);
    const delta = sortEvents(ctx.org.events.slice(from));
    const min = Math.min(...delta.map((e) => e.created_at));
    if (min <= seedMax) throw new Error(`${def.name}: delta event at ${min} is not younger than the seed (${seedMax})`);
    deltas.push(delta);
  }
  return { ctx, deltas };
}

function planEntry(ctx, def, piece, i) {
  const entry = {
    key: piece.key,
    piece: piece.piece,
    order: i + 1,
    after: piece.after,
    gate: Boolean(piece.gate),
    held: heldBy(ctx, def, piece),
  };
  if (piece.coveredBy) {
    entry.covered_by = ctx.byKey.get(piece.coveredBy);
    return entry;
  }
  entry.title = piece.title;
  entry.requires = piece.requires;
  entry.suggested_holder = piece.suggested ? personKey(piece.suggested).pubkey : null;
  entry.unfilled = piece.suggested ? null : piece.unfilled;
  return entry;
}

function nextWaveEntry(ctx, def, row) {
  const entry = { key: row.key };
  const piece = def.plan.find((p) => p.key === row.key);
  entry.piece = row.piece ?? piece.piece;
  if (row.title) entry.title = row.title;
  entry.requires = row.requires ?? piece.requires;
  if (row.gate) entry.gate = true;
  if (row.replaces) entry.replaces = row.replaces;
  entry.after = row.after.map((k) => ctx.gateItems[k] ?? ctx.byKey.get(itemKeyFor(def, k)));
  if (row.must_mention) entry.must_mention = row.must_mention;
  return entry;
}

export function buildSequences() {
  return SEQUENCES.map((def) => {
    const before = run(def, [def.before]);
    const gate = run(def, [def.before, (ctx) => gateStage(ctx, def)]);
    const a = run(def, [def.before, (ctx) => gateStage(ctx, def), (ctx) => outcomeStage(ctx, def, def.outcomes.a, "a")]);
    const b = run(def, [def.before, (ctx) => gateStage(ctx, def), (ctx) => outcomeStage(ctx, def, def.outcomes.b, "b")]);
    if (JSON.stringify(gate.deltas) !== JSON.stringify(a.deltas.slice(0, 2)) || JSON.stringify(gate.deltas) !== JSON.stringify(b.deltas.slice(0, 2))) {
      throw new Error(`${def.name}: the gate stage is not a pure prefix of both outcomes`);
    }
    const ctx = gate.ctx;
    const parentId = ctx.byKey.get(def.parentKey);
    const parent = ctx.org.items.get(parentId);
    const plan = def.plan.map((p, i) => planEntry(before.ctx, def, p, i));
    const draftNow = plan.filter((p) => !p.covered_by && !p.held).map((p) => p.key);
    const sequence = {
      name: def.name,
      org: def.spec.id,
      seed: `orgs/${def.spec.id}/seed.json`,
      snapshots: {
        before: ["before.json"],
        gate: ["before.json", "gate.json"],
        outcome_a: ["before.json", "gate.json", "outcome-a.json"],
        outcome_b: ["before.json", "gate.json", "outcome-b.json"],
      },
      parent: {
        id: parentId,
        root: parent.root,
        depth: parent.depth,
        title: parent.title,
        brief: parent.brief,
        holder: parent.dri,
        holder_name: nameOf(ctx, parent.dri),
      },
      trigger: "item_accepted",
      gate: def.gate,
      why_gold: def.why_gold,
      plan,
      draft_now: draftNow,
      gate_stage: {
        drafts: ctx.drafts,
        items: ctx.gateItems,
        gate_item: ctx.gateItems[def.gate],
        gate_holder: ctx.org.items.get(ctx.gateItems[def.gate]).dri,
      },
      outcomes: {},
    };
    for (const [label, built] of [
      ["a", a],
      ["b", b],
    ]) {
      const o = def.outcomes[label];
      sequence.outcomes[label] = {
        file: `outcome-${label}.json`,
        trigger: "child_done_unblocks",
        receipt: built.ctx.outcomeReceipt,
        said: o.said,
        next_wave: o.next_wave.map((row) => nextWaveEntry(built.ctx, def, row)),
        held: o.held,
        not_redrafted: o.not_redrafted,
        ...(o.elsewhere ? { elsewhere: o.elsewhere.map((e) => ({ ...e, under: built.ctx.byKey.get(e.under) })) } : {}),
        fails: o.fails,
        why_gold: o.why_gold,
      };
    }
    return {
      name: def.name,
      sequence,
      before: before.deltas[0],
      gate: gate.deltas[1],
      outcomeA: a.deltas[2],
      outcomeB: b.deltas[2],
    };
  });
}
