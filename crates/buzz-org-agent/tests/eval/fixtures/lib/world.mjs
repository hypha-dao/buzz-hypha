// `data.ts` → one org's seed, following the mapping table in Prototype map
// §3 row by row. `T` is the locale: identity for `en`, a lookup for
// `pt`/`es`. Ids, pubkeys, timestamps, and tags never pass through `T`.

import {
  direction as riverDirection,
  energyOrg,
  pricesChildren,
  projectsData,
  seedMessages,
  seedProposals,
  strategyDraft,
  ticketsData,
} from "../../../../../../prototypes/org-preview/src/lib/data.ts";
import { DAY, NOW, isoWeek, resolveDate } from "./dates.mjs";
import { AGENT, KIND, Org, keyName, personKey, registerNames, slugify } from "./emit.mjs";
import { lineIdFor } from "./nostr.mjs";

const SLUGS = ["mission", "vision", "objectives", "strategy"];
const HOUR = 3600;

// The prototype's two worlds, keyed like `constants.mjs`.
function sourceFor(spec) {
  if (spec.id === "river") {
    return {
      direction: riverDirection,
      projects: projectsData,
      tickets: ticketsData,
      ticketChildren: { prices: pricesChildren },
      proposals: seedProposals,
      messages: seedMessages,
      strategyDraft,
    };
  }
  return {
    direction: energyOrg.direction,
    projects: energyOrg.projects,
    tickets: energyOrg.tickets,
    ticketChildren: { "e-muni": energyOrg.tickets["e-muni"].children },
    proposals: energyOrg.proposals,
    messages: seedMessages,
    strategyDraft: null,
  };
}

function sentences(text) {
  return text
    .split(/(?<=[.!?])\s+(?=[A-Z“"])/)
    .map((s) => s.trim())
    .filter(Boolean);
}

export function buildOrg(spec, T = (s) => s) {
  const src = sourceFor(spec);
  registerNames([...spec.members, "You"]);
  const org = new Org(spec.id, { members: spec.members });
  const world = { spec, src, org, T, byKey: new Map(), talkDone: new Set() };

  const shapersRoomKey = Object.entries(spec.rooms).find(([, r]) => r.shapers)[0];
  org.bootstrap(spec.founder, spec.foundedAt, { shapersRoomKey });
  for (const add of spec.shaperAdds.filter((a) => a.after === "bootstrap")) {
    org.addShaper(spec.founder, add.name, org.at(null, spec.foundedAt));
  }
  communityRooms(world);
  directionHistory(world);
  profiles(world);
  communityTalk(world);
  const approved = Object.values(src.projects).filter((p) => p.approved);
  approved.sort((a, b) => resolveDate(a.approved) - resolveDate(b.approved));
  for (const p of approved) project(world, p);
  homeTalk(world);
  for (const p of Object.values(src.projects).filter((p) => !p.approved)) project(world, p);
  openDirectionDraft(world);
  for (const talk of spec.laterTalk) roomTalk(world, talk.room, talk.ids, talk.anchor);
  const health = healthGold(world);
  return { org, health, world };
}

// Cold start (Prototype map §3): one founder, four version-1 artifacts, one
// `39103`, an empty tree, no profiles. Written from constants, not `data.ts`.
export function buildCold(spec) {
  registerNames(spec.members);
  const org = new Org(spec.id, { members: spec.members });
  org.bootstrap(spec.founder, spec.foundedAt, { shapersRoomKey: "shapers" });
  let t = spec.foundedAt;
  for (const slug of SLUGS) {
    const d = spec.direction[slug];
    const rows = d.lines ?? [];
    const ls = rows.map((l, i) => ({ n: i + 1, id: lineIdFor(spec.id, slug, l.text), text: l.text, date: l.date ?? null }));
    t = org.at(null, t);
    org.directionVersion({
      slug,
      version: 1,
      proposer: spec.founder,
      body: d.body,
      lines: rows.length ? ls : null,
      at: t,
      decidedAt: t,
      yes: 1,
      agreedBy: [spec.founder],
      full: { body: d.body, lines: ls },
    });
  }
  return { org, health: [], world: { spec, org, byKey: new Map() } };
}

// ── rooms and talk ───────────────────────────────────────────────────────────

function communityRooms({ spec, org }) {
  for (const [key, r] of Object.entries(spec.rooms)) {
    if (!r.community) continue;
    org.room(key, { name: r.community.name, topic: r.community.topic, members: r.members, at: org.at(null, spec.foundedAt) });
  }
}

function ensureDm({ spec, org }, key, anchor) {
  if (org.rooms.has(key)) return;
  const [opener, ...others] = spec.rooms[key].dm;
  org.dm(key, opener, others, org.at(anchor));
}

function roomTalk(world, roomKey, ids, anchor) {
  const { spec, org, src, T } = world;
  if (spec.rooms[roomKey]?.dm) ensureDm(world, roomKey, anchor - 60);
  let t = null;
  for (const id of ids) {
    if (world.talkDone.has(`${roomKey}/${id}`)) continue;
    const msg = src.messages[roomKey].find((m) => m.id === id);
    if (!msg) throw new Error(`no message ${roomKey}/${id}`);
    t = t === null ? org.at(anchor) : org.at(null, t);
    org.message(roomKey, id, msg.from === "you" ? "You" : msg.from, T(msg.text), t);
    world.talkDone.add(`${roomKey}/${id}`);
  }
  return t;
}

function remainingTalk(world, roomKey) {
  const { spec, src } = world;
  const ids = (src.messages[roomKey] ?? []).map((m) => m.id).filter((id) => !world.talkDone.has(`${roomKey}/${id}`));
  if (ids.length) roomTalk(world, roomKey, ids, spec.rooms[roomKey].anchor);
}

function communityTalk(world) {
  for (const [key, r] of Object.entries(world.spec.rooms)) if (r.community) remainingTalk(world, key);
}

function homeTalk(world) {
  for (const [key, r] of Object.entries(world.spec.rooms)) if (r.home || r.dm) remainingTalk(world, key);
}

// ── §4.1 direction ───────────────────────────────────────────────────────────

function lines(world, slug, artifact) {
  const { spec, T } = world;
  const rows = slug === "objectives" ? artifact.items : slug === "strategy" ? artifact.lines : [];
  return rows.map((row, i) => ({
    n: i + 1,
    id: lineIdFor(spec.id, slug, row.text),
    text: T(row.text),
    date: slug === "objectives" ? spec.objectiveDates[i] : null,
  }));
}

function directionHistory(world) {
  const { spec, src, org, T } = world;
  const versions = [];
  for (const slug of SLUGS) {
    const artifact = src.direction[slug];
    for (const h of artifact.history) {
      const proposal = src.proposals.find((p) => p.kind === "direction" && p.artifact === slug && p.id.endsWith(`-v${h.version}`));
      versions.push({ slug, artifact, h, proposal, at: resolveDate(h.confirmedOn, "start") });
    }
  }
  versions.sort((a, b) => a.at - b.at || SLUGS.indexOf(a.slug) - SLUGS.indexOf(b.slug) || a.h.version - b.h.version);
  for (const v of versions) {
    const { slug, artifact, h, proposal } = v;
    const isHead = h.version === artifact.version;
    const key = `${slug}@${h.version}`;
    const talk = spec.directionTalk[key];
    let draft = null;
    let openAt = v.at;
    if (talk) {
      const last = roomTalk(world, talk.room, talk.said, v.at - HOUR);
      const card = src.messages[talk.room].find((m) => m.id === talk.card);
      const draftAt = org.at(null, last);
      const said = talk.said.map((id) => org.receiptFor(org.msg(talk.room, id)));
      const full = fullBody(world, slug, artifact);
      draft = org.draft({
        kind: "direction",
        needs: "shaper",
        move: 1,
        origin: "talk",
        gap: `${slug}@${h.version - 1}`,
        receipts: said,
        payload: {
          slug,
          base_version: h.version - 1,
          body: full.body,
          lines: full.lines,
          diff: T(h.change),
          heard: said.map((r) => r[1]),
        },
        at: draftAt,
      });
      org.message(talk.room, talk.card, AGENT, T(card.text), draftAt);
      world.talkDone.add(`${talk.room}/${talk.card}`);
      openAt = org.at(null, draftAt);
    }
    const proposer = proposal?.openedBy ?? h.confirmedBy;
    const full = fullBody(world, slug, artifact);
    org.directionVersion({
      slug,
      version: h.version,
      proposer,
      body: isHead ? full.body : T(h.change),
      lines: isHead ? full.lines : null,
      why: T(h.change),
      at: openAt,
      decidedAt: v.at,
      yes: proposal?.yes ?? org.shaperPubkeys().length,
      agreedBy: proposal?.agreedBy ?? [h.confirmedBy],
      draft,
      full: isHead ? full : { body: T(h.change), lines: [] },
    });
    for (const add of spec.shaperAdds.filter((a) => a.after === key)) {
      org.addShaper(spec.founder, add.name, org.at(null, v.at));
    }
  }
}

function fullBody(world, slug, artifact) {
  const { T } = world;
  return {
    body: [artifact.text, ...artifact.body].map(T).join("\n\n"),
    lines: lines(world, slug, artifact),
  };
}

function objectiveRef(world, index) {
  const { spec, src } = world;
  if (index === undefined || index === null) return null;
  const artifact = src.direction.objectives;
  return `objectives@${artifact.version}#${lineIdFor(spec.id, "objectives", artifact.items[index].text)}`;
}

// The open strategy line from Tuesday's call — a `50100` nobody has settled.
function openDirectionDraft(world) {
  const { spec, src, org, T } = world;
  const d = spec.openDirectionDraft;
  if (!d) return;
  const ingest = src.messages[d.room].find((m) => m.id === d.ingest);
  const t0 = org.at(d.anchor);
  org.message(d.room, d.ingest, AGENT, T(ingest.text), t0);
  world.talkDone.add(`${d.room}/${d.ingest}`);
  const last = roomTalk(world, d.room, d.said, t0 + 60);
  const head = org.direction.get(d.slug).artifact;
  const said = d.said.map((id) => org.receiptFor(org.msg(d.room, id)));
  const draftAt = org.at(null, last);
  const newLine = src.strategyDraft
    ? { n: head.lines.length + 1, id: lineIdFor(spec.id, d.slug, src.strategyDraft.added), text: T(src.strategyDraft.added), date: null }
    : { n: head.lines.length + 1, id: lineIdFor(spec.id, d.slug, "Pilots before conferences — no booth until six communities are live."), text: T("Pilots before conferences — no booth until six communities are live."), date: null };
  const draft = org.draft({
    kind: "direction",
    needs: "shaper",
    move: 1,
    origin: "talk",
    gap: `${d.slug}@${head.version}`,
    receipts: said,
    payload: {
      slug: d.slug,
      base_version: head.version,
      body: head.body,
      lines: [...head.lines, newLine],
      diff: `+ ${newLine.text}`,
      heard: said.map((r) => r[1]),
    },
    at: draftAt,
  });
  const card = src.messages[d.room].find((m) => m.id === d.card);
  org.message(d.room, d.card, AGENT, T(card.text), draftAt);
  world.talkDone.add(`${d.room}/${d.card}`);
  return draft;
}

// ── §4.7a profiles ───────────────────────────────────────────────────────────

function profiles(world) {
  const { spec, org, T } = world;
  let t = org.at(null, spec.foundedAt + DAY);
  for (const p of spec.profiles) {
    org.setProfile(p.name, { about: T(p.about), skills: p.skills, openLimit: p.openLimit, at: t });
    t = org.at(null, t);
  }
}

function heldItems(org, name) {
  const pk = personKey(name).pubkey;
  return [...org.items.values()].filter((i) => i.dri === pk).map((i) => i.id);
}

function matched(org, name) {
  const profile = org.profiles.get(personKey(name).pubkey);
  return {
    skills: profile ? profile.skills.map((s) => s.slug) : [],
    about: null,
    items: heldItems(org, name),
  };
}

// ── §4.2 work items ──────────────────────────────────────────────────────────

function project(world, proj) {
  const { spec, src, org, T } = world;
  const key = proj.id;
  const row = src.proposals.find((p) => p.id === spec.proposalOf[key]);
  const dueAt = resolveDate(proj.review, "end");
  const approvedAt = proj.approved ? resolveDate(proj.approved, "start") : null;
  const cited = spec.projectReceipt[key];
  const citedMsg = cited?.[1] ? org.msg(cited[0], cited[1]) : null;
  // An open proposal opened when its trail says; with no trail, an hour
  // after the line it was drafted from.
  const openedAt = approvedAt
    ? approvedAt - DAY
    : proj.trail[0]
      ? resolveDate(proj.trail[0].when, "start")
      : citedMsg.created_at + 2 * HOUR;
  const objRef = objectiveRef(world, spec.objectiveOf[key]);
  const suggestedDri = proj.dri ?? spec.suggested[key] ?? null;
  let draft = null;
  if (/^Drafted from/.test(proj.from)) {
    const receipt = citedMsg ? org.receiptFor(citedMsg) : org.roomReceipt(cited[0]);
    const payload = {
      title: T(proj.title),
      brief: T(proj.brief),
      objective_ref: objRef,
      due_at: dueAt,
      suggested_dri: suggestedDri ? personKey(suggestedDri).pubkey : null,
      why: T(proj.from),
      gaps: objRef ? [{ ref: objRef, served: "not", by: [] }] : [],
    };
    if (suggestedDri) payload.matched = matched(org, suggestedDri);
    draft = org.draft({
      kind: "project",
      needs: "shaper",
      move: 1,
      origin: "talk",
      gap: objRef ?? `project:${key}`,
      suggested: suggestedDri,
      skills: payload.matched?.skills ?? [],
      receipts: [receipt],
      payload,
      at: org.at(openedAt - HOUR),
    });
  }
  const proposer = row?.openedBy ?? spec.founder;
  // The prototype thread that is this project's home room (§6.7), if any.
  const homeKey = Object.entries(spec.rooms).find(([, r]) => r.home === key)?.[0] ?? null;
  const { proposal, itemId } = org.proposeProject({
    key,
    proposer,
    title: T(proj.title),
    brief: T(proj.brief),
    slug: slugify(proj.title),
    dueAt,
    objectiveRef: objRef,
    suggestedDri,
    at: org.at(openedAt, draft?.created_at),
    draft,
    homeKey,
    decide: approvedAt ? { yes: row?.yes ?? org.shaperPubkeys().length, agreedBy: row?.agreedBy ?? [], at: approvedAt } : null,
  });
  world.byKey.set(key, itemId);
  if (!approvedAt) return;
  const root = org.items.get(itemId);
  let t = proposal.decided_at;
  if (proj.dri) {
    t = org.at(null, t);
    org.accept(itemId, t);
  }
  const rows = ticketRows(world, proj);
  t = seedRows(world, key, itemId, rows, t);
  const reviewAt = dueAt - Math.max(0.2 * (dueAt - approvedAt), 2 * DAY);
  if (root.state === "accepted" && reviewAt <= NOW) org.review(itemId, org.at(reviewAt, t));
}

// A project's rows: `tickets[]` plus the live `ticketsData` rows that
// belong to it, in `data.ts` order.
function ticketRows(world, proj) {
  const { spec, src } = world;
  const rows = [...proj.tickets];
  for (const liveKey of spec.liveTickets[proj.id] ?? []) {
    const t = src.tickets[liveKey];
    rows.push({
      title: t.title,
      who: t.dri ?? "open",
      state: t.dri ? "doing" : "open",
      due: t.due ?? undefined,
      brief: t.draft ?? "",
      children: src.ticketChildren[liveKey] ?? [],
      live: liveKey,
    });
  }
  return rows;
}

function seedRows(world, projectKey, parentId, rows, t) {
  const { spec, org, T } = world;
  const parent = org.items.get(parentId);
  const seen = new Set();
  for (const row of rows) {
    const slug = slugify(row.title);
    if (seen.has(slug)) throw new Error(`duplicate ticket title under ${projectKey}: ${row.title}`);
    seen.add(slug);
    const key = `${projectKey}/${slug}`;
    const dueAt = row.due ? resolveDate(row.due, "end") : parent.due_at;
    const who = row.who === "open" ? null : row.who;
    const holder = keyName(parent.dri);
    let draft = null;
    const suggested = row.live ? spec.suggested[row.live] : null;
    if (suggested) {
      t = org.at(null, t);
      const payload = {
        parent: parentId,
        title: T(row.title),
        brief: T(row.brief ?? ""),
        due_at: dueAt,
        requires: [],
        suggested_holder: personKey(suggested).pubkey,
        unfilled: null,
        covers: T(row.title),
        after: [],
        gate: false,
        coverage: [],
        matched: matched(org, suggested),
      };
      draft = org.draft({
        kind: "ticket",
        needs: holder,
        move: 2,
        origin: "gap",
        gap: `${parentId}#${slug}`,
        parent: parentId,
        suggested,
        skills: payload.matched.skills,
        receipts: [org.itemReceipt(parentId)],
        payload,
        at: t,
      });
    }
    t = org.at(null, t);
    const { item } = org.createTicket({
      key,
      parentId,
      title: T(row.title),
      brief: T(row.brief ?? ""),
      slug,
      dueAt,
      at: t,
      draft,
    });
    world.byKey.set(key, item.id);
    // "Assigned" is offered-then-accepted (Prototype map §3): `who` gets an
    // `io_offer`; `doing`/`done` rows add the `io_accept` that said yes.
    if (who) {
      t = org.at(null, t);
      org.offer(item.id, who, t, { by: holder });
    }
    if (row.state === "doing" || row.state === "done") {
      if (!who) throw new Error(`${key}: ${row.state} without a holder`);
      t = org.at(null, t);
      org.accept(item.id, t);
    }
    if (row.children?.length) {
      t = seedRows(world, `${projectKey}/${slug}`, item.id, row.children, t);
    }
    if (row.state === "done") {
      const talk = spec.doneFromTalk[key];
      if (talk) {
        const msg = org.msg(...talk);
        t = org.at(msg.created_at + 60, t);
        org.done(item.id, t, { receipt: msg, signer: AGENT });
      } else {
        t = org.at(null, t);
        org.done(item.id, t);
      }
    }
  }
  if (rows.length) org.refreshItem(parentId, t);
  return t;
}

// ── §4.7 health — the gold for move 4, not state ─────────────────────────────

function healthGold(world) {
  const { src, org, T } = world;
  const week = isoWeek(NOW);
  const out = [];
  for (const proj of Object.values(src.projects)) {
    if (!proj.health) continue;
    out.push(
      org.health({
        itemId: world.byKey.get(proj.id),
        week,
        pct: proj.health.pct,
        sentences: sentences(proj.health.text).map(T),
        at: NOW,
      }),
    );
  }
  return out;
}

export { KIND };
