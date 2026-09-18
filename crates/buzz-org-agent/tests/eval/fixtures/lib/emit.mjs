// The relay, in miniature: person-signed commands go in, relay-signed state
// comes out, with exactly the tag set Protocol §4 fixes for each kind
// (mirrors `crates/buzz-relay/src/handlers/intelligent_org/state.rs`).
//
// `Org` keeps the projection the relay would keep — items, proposals,
// direction heads, the Shaper set, draft outcomes — so every state event
// is derived from the commands before it, never written by hand. Every
// mutator returns the events it stored, in relay order.

import { Clock, DAY } from "./dates.mjs";
import { keyFor, signEvent, uuidFor } from "./nostr.mjs";

export const KIND = {
  MESSAGE: 9,
  MEMBERSHIP: 13534,
  DIRECTION: 39100,
  WORK_ITEM: 39101,
  PROPOSAL: 39102,
  SHAPERS: 39103,
  OUTCOME: 39104,
  PROFILE: 39105,
  CHANNEL_META: 39000,
  CHANNEL_MEMBERS: 39002,
  DM_OPEN: 41010,
  SHAPERS_PROPOSE: 50001,
  DIRECTION_PROPOSE: 50002,
  VOTE: 50003,
  PROJECT_PROPOSE: 50004,
  TICKET_CREATE: 50005,
  OFFER: 50006,
  ACCEPT: 50007,
  DECLINE: 50008,
  DONE: 50009,
  SET_DUE: 50011,
  DRAFT_DECIDE: 50012,
  SHAPER_ACCEPT: 50019,
  PROFILE_SET: 50021,
  DRAFT: 50100,
  HEALTH: 50101,
  PROGRESS: 50102,
};

export const DECISION_WINDOW = 7 * DAY;
export const OFFER_WINDOW = 3 * DAY;
export const RELAY = "relay";
export const AGENT = "agent";
export const READER = "reader";

const DEFAULT_RULES = {
  direction: "majority",
  project: "majority",
  dri: "majority",
  shapers: "majority",
  money: "majority",
  join: "majority",
};

export function neededFor(rule, eligible) {
  if (eligible === 0) return 0;
  if (rule === "majority") return Math.floor(eligible / 2) + 1;
  if (rule === "all") return eligible;
  return Math.min(Math.max(rule, 1), eligible);
}

export function slugify(text) {
  return text
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

export function bandFor(pct) {
  return pct <= 33 ? "struggling" : pct <= 66 ? "wobbly" : "healthy";
}

// A person's key by fixture name; `You` is the harness's reader.
export function personKey(name) {
  return keyFor(name === "You" || name === "you" ? READER : name);
}

export class Org {
  constructor(id, { members = [] } = {}) {
    this.id = id;
    this.clock = new Clock();
    this.events = [];
    this.byId = new Map();
    this.relay = keyFor(RELAY).pubkey;
    this.agent = keyFor(AGENT).pubkey;
    this.members = new Set(members.map((n) => personKey(n).pubkey));
    this.items = new Map();
    this.itemEvents = new Map(); // item uuid → latest 39101 event
    this.proposals = new Map();
    this.direction = new Map(); // slug → { artifact, eventId }
    this.shapers = null;
    this.outcomes = new Map();
    this.drafts = new Map();
    this.profiles = new Map();
    this.rooms = new Map(); // room uuid → { name, kind, members:Set, topic, private, dm }
    this.roomEvents = new Map();
    this.messages = new Map(); // "<room-key>/<msg-id>" → event
  }

  // ── plumbing ─────────────────────────────────────────────────────────────

  uuid(...parts) {
    return uuidFor(this.id, ...parts);
  }

  at(anchor, ...after) {
    return this.clock.next(anchor, ...after);
  }

  // A relay stores an event once. A parent refreshed twice in the same
  // second with the same counts is the same event — same id — so the
  // second write is the duplicate the relay would drop.
  store(ev) {
    const seen = this.byId.get(ev.id);
    if (seen) return seen;
    this.byId.set(ev.id, ev);
    this.events.push(ev);
    return ev;
  }

  command(signer, kind, at, tags, content = {}) {
    return this.store(signEvent({ signer: personKey(signer).name, kind, createdAt: at, tags, content }));
  }

  state(kind, d, at, tags, content) {
    return this.store(
      signEvent({ signer: RELAY, kind, createdAt: at, tags: [["d", d], ...tags], content }),
    );
  }

  shaperPubkeys() {
    return this.shapers ? [...this.shapers.shapers] : [];
  }

  isShaper(name) {
    return this.shaperPubkeys().includes(personKey(name).pubkey);
  }

  // ── membership (NIP-43) and rooms (NIP-29) ───────────────────────────────

  membership(at, roles = {}) {
    const founder = this.shapers?.founder;
    const tags = [["-"]];
    for (const pk of [...this.members].sort()) {
      const role = roles[pk] ?? (pk === founder ? "owner" : "member");
      tags.push(["member", pk, role]);
    }
    tags.push(["member", this.agent, "member"]);
    return this.store(signEvent({ signer: RELAY, kind: KIND.MEMBERSHIP, createdAt: at, tags, content: "" }));
  }

  // A room: `#shapers`, a project home, a community room, or a DM. The org
  // agent is a member of every one (Protocol §6.8); a DM's identity
  // excludes it, so it is absent from the 39000 `p` tags.
  room(key, { name, topic, privateRoom = false, dm = null, members = [], at }) {
    const id = this.uuid("room", key);
    const memberSet = new Set(members.map((n) => personKey(n).pubkey));
    const room = { id, key, name, topic, private: privateRoom, dm, members: memberSet, at };
    this.rooms.set(key, room);
    this.emitRoom(room, at);
    return room;
  }

  emitRoom(room, at) {
    const meta = [["name", room.name]];
    if (room.private) meta.push(["private"]);
    else meta.push(["public"]);
    if (room.dm) {
      meta.push(["hidden"]);
      for (const pk of [...room.members].sort()) meta.push(["p", pk]);
    }
    meta.push(["closed"]);
    meta.push(["t", room.dm ? "dm" : "stream"]);
    if (room.topic) meta.push(["topic", room.topic]);
    const roster = [["p", this.agent]];
    for (const pk of [...room.members].sort()) roster.push(["p", pk]);
    const metaEv = this.state(KIND.CHANNEL_META, room.id, at, meta, "");
    const membersEv = this.state(KIND.CHANNEL_MEMBERS, room.id, at, roster, "");
    this.roomEvents.set(room.key, { meta: metaEv, members: membersEv });
  }

  // A 1:1 (or small) DM: the opener's `41010` names the others; the relay
  // creates the room. `dm: true` marks the identity as participants only.
  dm(key, opener, others, at) {
    const openEv = this.command(
      opener,
      KIND.DM_OPEN,
      at,
      others.map((n) => ["p", personKey(n).pubkey]),
      "",
    );
    const participants = [opener, ...others];
    const room = this.room(key, {
      name: participants.map((n) => (n === AGENT ? "Org agent" : n)).join(", "),
      dm: true,
      members: participants.filter((n) => n !== AGENT),
      at,
    });
    room.open = openEv;
    return room;
  }

  joinRoom(key, name, at) {
    const room = this.rooms.get(key);
    const pk = personKey(name).pubkey;
    if (room.members.has(pk) || name === AGENT) return;
    room.members.add(pk);
    this.emitRoom(room, at);
  }

  message(roomKey, msgId, from, text, at, extraTags = []) {
    const room = this.rooms.get(roomKey);
    if (!room) throw new Error(`no room ${roomKey}`);
    const signer = from === AGENT || from === "agent" ? AGENT : from;
    if (signer !== AGENT && !room.members.has(personKey(signer).pubkey)) {
      this.joinRoom(roomKey, signer, at);
    }
    const ev = this.store(
      signEvent({
        signer: signer === AGENT ? AGENT : personKey(signer).name,
        kind: KIND.MESSAGE,
        createdAt: at,
        tags: [["h", room.id], ...extraTags],
        content: text,
      }),
    );
    this.messages.set(`${roomKey}/${msgId}`, ev);
    return ev;
  }

  msg(roomKey, msgId) {
    const ev = this.messages.get(`${roomKey}/${msgId}`);
    if (!ev) throw new Error(`no message ${roomKey}/${msgId}`);
    return ev;
  }

  // ── §4.5 Shapers ─────────────────────────────────────────────────────────

  emitShapers(at, receipt) {
    this.shapers.updated_at = at;
    this.shapers.receipt = receipt;
    return this.state(
      KIND.SHAPERS,
      "shapers",
      at,
      this.shapers.shapers.map((p) => ["p", p]),
      this.shapers,
    );
  }

  // §6.4 bootstrap: the owner adds themself; the relay makes `#shapers`,
  // fills the hosted agent, and the proposal passes at once.
  bootstrap(founder, at, { shapersRoomKey = "shapers" }) {
    const pk = personKey(founder).pubkey;
    this.members.add(pk);
    const cmd = this.command(
      founder,
      KIND.SHAPERS_PROPOSE,
      at,
      [["op", "add"], ["p", pk], ["vote", "agree"]],
      { why: "founding" },
    );
    const room = this.room(shapersRoomKey, {
      name: "shapers",
      topic: "Shapers",
      privateRoom: true,
      members: [founder],
      at,
    });
    const proposal = this.openProposal({
      kind: "shapers",
      openedBy: founder,
      at,
      receipt: cmd,
      payload: { why: "founding" },
      subject: pk,
      eligible: [pk],
      rule: "majority",
      key: `shapers-add-${founder}`,
    });
    this.shapers = {
      founder: pk,
      shapers: [pk],
      offered: [],
      room: room.id,
      agent: this.agent,
      agent_hosted: true,
      rules: { ...DEFAULT_RULES },
      decision_window_secs: DECISION_WINDOW,
      offer_window_secs: OFFER_WINDOW,
      updated_at: at,
      receipt: cmd.id,
    };
    this.recordVote(proposal, founder, "agree", at, cmd, { executed: { kind: "shapers", id: pk } });
    this.emitShapers(at, cmd.id);
    this.membership(at);
    return cmd;
  }

  // A `shapers/add` for `name`, voted through by the current set, then the
  // seat accepted (`50019`) — the only path onto the Shaper set (§4.5).
  addShaper(proposer, name, at) {
    const pk = personKey(name).pubkey;
    this.members.add(pk);
    const eligible = this.shaperPubkeys().filter((p) => p !== pk);
    const cmd = this.command(
      proposer,
      KIND.SHAPERS_PROPOSE,
      at,
      [["op", "add"], ["p", pk], ["vote", "agree"]],
      {},
    );
    const proposal = this.openProposal({
      kind: "shapers",
      openedBy: proposer,
      at,
      receipt: cmd,
      payload: {},
      subject: pk,
      eligible,
      rule: this.shapers.rules.shapers,
      key: `shapers-add-${name}`,
    });
    let t = at;
    this.recordVote(proposal, proposer, "agree", t, cmd, { executedOnPass: () => ({ kind: "shapers", id: pk }) });
    for (const voter of eligible) {
      if (proposal.status !== "open") break;
      if (voter === personKey(proposer).pubkey) continue;
      t = this.at(null, t);
      this.vote(keyName(voter), proposal.id, "agree", t);
    }
    // Passing appends the seat to `offered`; the person's accept makes it live.
    this.shapers.offered.push({ p: pk, proposal: proposal.id, at: proposal.decided_at });
    this.emitShapers(proposal.decided_at, proposal.lastReceipt);
    const acceptAt = this.at(null, proposal.decided_at);
    const accept = this.command(name, KIND.SHAPER_ACCEPT, acceptAt, [["e", proposal.id]], {});
    this.shapers.offered = this.shapers.offered.filter((o) => o.p !== pk);
    this.shapers.shapers.push(pk);
    this.emitShapers(acceptAt, accept.id);
    const shapersRoom = [...this.rooms.values()].find((r) => r.id === this.shapers.room);
    this.joinRoom(shapersRoom.key, name, acceptAt);
    this.membership(acceptAt);
    return accept;
  }

  // ── §4.4 proposals ───────────────────────────────────────────────────────

  openProposal({ kind, openedBy, at, receipt, payload, subject = null, item = null, eligible, rule, key, draft = null }) {
    const id = this.uuid("proposal", key);
    const proposal = {
      id,
      kind,
      status: "open",
      opened_by: personKey(openedBy).pubkey,
      opened_at: at,
      expires_at: at + (this.shapers?.decision_window_secs ?? DECISION_WINDOW),
      draft,
      payload,
      rule,
      needed: neededFor(rule, eligible.length),
      eligible,
      votes: [],
      decided_at: null,
      executed: null,
      settlement: null,
    };
    Object.defineProperty(proposal, "_meta", {
      enumerable: false,
      value: { subject, item, key, receipt: receipt.id },
    });
    this.proposals.set(id, proposal);
    this.emitProposal(proposal, at, receipt.id);
    return proposal;
  }

  emitProposal(p, at, receipt) {
    const tags = [
      ["t", p.kind],
      ["s", p.status],
      ["p", p.opened_by],
    ];
    if (p._meta.subject) tags.push(["p", p._meta.subject, "", "subject"]);
    for (const e of p.eligible) tags.push(["p", e, "", "eligible"]);
    if (p._meta.item) tags.push(["i", p._meta.item]);
    // The `receipt` tag is the opening command (§4.4); the vote that moved
    // it is `lastReceipt`, which the 39103 a passed `shapers` op writes cites.
    tags.push(["receipt", p._meta.receipt]);
    Object.defineProperty(p, "lastReceipt", { enumerable: false, writable: true, value: receipt });
    const { _meta, lastReceipt, ...content } = p;
    return this.state(KIND.PROPOSAL, p.id, at, tags, content);
  }

  // Record a vote (from the opening command's `["vote","agree"]` or an
  // `io_vote`), evaluate the rule, and execute on pass (§5.3).
  recordVote(proposal, voter, choice, at, receipt, { executed = null, executedOnPass = null, onPass = null } = {}) {
    const pk = personKey(voter).pubkey;
    if (!proposal.eligible.includes(pk)) return null;
    proposal.votes = proposal.votes.filter((v) => v.p !== pk);
    proposal.votes.push({ p: pk, vote: choice, at, receipt: receipt.id });
    const agrees = proposal.votes.filter((v) => v.vote === "agree").length;
    const declines = proposal.votes.filter((v) => v.vote === "decline").length;
    if (agrees >= proposal.needed) {
      proposal.status = "passed";
      proposal.decided_at = at;
      proposal.executed = executed ?? (executedOnPass ? executedOnPass() : null);
      if (onPass) proposal.executed = onPass(proposal, at, receipt) ?? proposal.executed;
    } else if (declines > proposal.eligible.length - proposal.needed) {
      proposal.status = "rejected";
      proposal.decided_at = at;
    }
    return this.emitProposal(proposal, at, receipt.id);
  }

  vote(voter, proposalId, choice, at, { reason = null, onPass = null } = {}) {
    const proposal = this.proposals.get(proposalId);
    const content = reason ? { reason } : {};
    const cmd = this.command(voter, KIND.VOTE, at, [["e", proposalId], ["vote", choice]], content);
    this.recordVote(proposal, voter, choice, at, cmd, { onPass: onPass ?? proposal._meta.onPass ?? null });
    return cmd;
  }

  proposalByKey(key) {
    const hit = [...this.proposals.values()].find((p) => p._meta.key === key);
    if (!hit) throw new Error(`no proposal ${key}`);
    return hit;
  }

  // Cast the votes a proposal needs, in Shaper order, from `agreedBy` /
  // `rejectedBy` or the first `yes` eligible Shapers (Prototype map §3).
  decide(proposal, { yes, no = 0, agreedBy = [], rejectedBy = [], at, onPass = null }) {
    const opener = proposal.opened_by;
    const eligibleNames = proposal.eligible.map((pk) => keyName(pk));
    const agree = [...agreedBy];
    for (const n of eligibleNames) if (agree.length < yes && !agree.includes(n) && !rejectedBy.includes(n)) agree.push(n);
    const decline = [...rejectedBy];
    for (const n of eligibleNames) if (decline.length < no && !decline.includes(n) && !agree.includes(n)) decline.push(n);
    let t = at;
    const order = [];
    for (const n of agree) order.push([n, "agree"]);
    for (const n of decline) order.push([n, "decline"]);
    // The opener votes first when they are among the voters.
    order.sort((a, b) => (personKey(b[0]).pubkey === opener) - (personKey(a[0]).pubkey === opener));
    for (const [name, choice] of order) {
      if (proposal.status !== "open") break;
      this.vote(name, proposal.id, choice, t, { onPass });
      t = this.at(null, t);
    }
    return proposal;
  }

  // ── §4.1 direction ───────────────────────────────────────────────────────

  // `io_direction_propose` for `slug`; passes under the current rule with
  // the votes given, and the relay writes the new `39100` head (§5.2).
  directionVersion({ slug, version, proposer, body, lines = null, why = null, at, decidedAt, yes, agreedBy, key, draft = null, full = null }) {
    const eligible = this.shaperPubkeys();
    const content = { body };
    if (lines) content.lines = lines.map((l) => (l.id ? { id: l.id, text: l.text, date: l.date ?? null } : { text: l.text, date: l.date ?? null }));
    if (why) content.why = why;
    const tags = [["d", slug], ["base", String(version - 1)]];
    if (draft) tags.push(["e", draft.id, "", "draft"]);
    if (this.isShaper(proposer)) tags.push(["vote", "agree"]);
    const cmd = this.command(proposer, KIND.DIRECTION_PROPOSE, at, tags, content);
    if (draft) this.settleDraft(draft, cmd, at, "accepted");
    const proposal = this.openProposal({
      kind: "direction",
      openedBy: proposer,
      at,
      receipt: cmd,
      payload: content,
      eligible,
      rule: this.shapers.rules.direction,
      key: key ?? `direction-${slug}-v${version}`,
      draft: draft?.id ?? null,
    });
    const onPass = (p, passedAt) => {
      const prev = this.direction.get(slug);
      const artifact = {
        slug,
        version,
        body: full?.body ?? body,
        ...(full?.lines?.length ? { lines: full.lines } : {}),
        confirmed_by: p.votes.filter((v) => v.vote === "agree").at(-1).p,
        confirmed_at: passedAt,
        proposed_by: p.opened_by,
        proposal: p.id,
        ...(prev ? { prev: prev.eventId } : {}),
      };
      const ev = this.state(
        KIND.DIRECTION,
        slug,
        passedAt,
        [["version", String(version)], ["p", artifact.confirmed_by], ["receipt", p.id]],
        artifact,
      );
      this.direction.set(slug, { artifact, eventId: ev.id });
      return { kind: "direction", id: `${slug}@${version}` };
    };
    this.recordVote(proposal, proposer, "agree", at, cmd, { onPass });
    if (proposal.status === "open") {
      this.decide(proposal, { yes, agreedBy, at: this.at(decidedAt ?? at, at), onPass });
    }
    return proposal;
  }

  // ── §4.2 work items ──────────────────────────────────────────────────────

  // Write an item's head. A child's change moves its parent's `children`
  // counts, so the relay writes the parent's head in the same transaction.
  emitItem(item, at, receipt, { cascade = true } = {}) {
    const tags = [["s", item.state], ["root", item.root]];
    if (item.parent) tags.push(["u", item.parent]);
    if (item.dri) tags.push(["p", item.dri]);
    if (item.offered_to) tags.push(["p", item.offered_to, "", "offered"]);
    tags.push(["due", String(item.due_at)]);
    tags.push(["t", item.parent ? "ticket" : "project"]);
    if (item.objective_ref) tags.push(["ref", item.objective_ref]);
    tags.push(["receipt", receipt]);
    const ev = this.state(KIND.WORK_ITEM, item.id, at, tags, item);
    this.itemEvents.set(item.id, ev);
    if (cascade && item.parent) {
      const parent = this.items.get(item.parent);
      const before = JSON.stringify(parent.children);
      this.recount(parent);
      if (JSON.stringify(parent.children) !== before) this.emitItem(parent, at, receipt, { cascade: false });
    }
    return ev;
  }

  childrenOf(id) {
    return [...this.items.values()].filter((i) => i.parent === id);
  }

  recount(item) {
    const counts = { open: 0, offered: 0, accepted: 0, done: 0 };
    for (const c of this.childrenOf(item.id)) {
      counts[c.state === "in_review" ? "accepted" : c.state] += 1;
    }
    item.children = counts;
  }

  // Re-emit a parent's `39101` once its subtree is seeded: the relay writes
  // one per child change; the fixture collapses those into the final head.
  refreshItem(id, at) {
    const item = this.items.get(id);
    this.recount(item);
    return this.emitItem(item, at, this.itemEvents.get(id).tags.find((t) => t[0] === "receipt")[1]);
  }

  // `io_project_propose`; on pass the root opens (`offered` when the
  // payload names a DRI) with its home room (§5.3, §6.7).
  // `slug` is the room name; it is an identifier, so a locale build passes the
  // `en` title's slug and shares it (Prototype map § Locales).
  proposeProject({ key, proposer, title, brief, dueAt, objectiveRef = null, suggestedDri = null, at, draft = null, decide = null, homeMembers = [], homeKey = null, slug = slugify(title) }) {
    const payload = { title, brief, due_at: dueAt };
    if (objectiveRef) payload.objective_ref = objectiveRef;
    if (suggestedDri) payload.suggested_dri = personKey(suggestedDri).pubkey;
    const tags = [];
    if (draft) tags.push(["e", draft.id, "", "draft"]);
    if (decide && this.isShaper(proposer)) tags.push(["vote", "agree"]);
    const cmd = this.command(proposer, KIND.PROJECT_PROPOSE, at, tags, payload);
    if (draft) this.settleDraft(draft, cmd, at, "accepted");
    const proposal = this.openProposal({
      kind: "project",
      openedBy: proposer,
      at,
      receipt: cmd,
      payload,
      eligible: this.shaperPubkeys(),
      rule: this.shapers.rules.project,
      key: `project-${key}`,
      draft: draft?.id ?? null,
    });
    const id = this.uuid("item", key);
    const onPass = (p, passedAt, receipt) => {
      const room = this.room(homeKey ?? `home-${key}`, {
        name: slug,
        topic: title,
        members: homeMembers,
        at: passedAt,
      });
      const item = {
        id,
        parent: null,
        root: id,
        depth: 0,
        path: [],
        title,
        brief,
        state: suggestedDri ? "offered" : "open",
        dri: null,
        offered_to: suggestedDri ? personKey(suggestedDri).pubkey : null,
        offered_by: suggestedDri ? p.opened_by : null,
        offered_at: suggestedDri ? passedAt : null,
        due_at: dueAt,
        approved_at: passedAt,
        objective_ref: objectiveRef,
        created_from: cmd.id,
        draft: draft?.id ?? null,
        done_receipt: null,
        closed_by: null,
        children: { open: 0, offered: 0, accepted: 0, done: 0 },
        home: { channel: room.id },
      };
      this.items.set(id, item);
      this.emitItem(item, passedAt, receipt.id);
      return { kind: "work_item", id };
    };
    // A proposal left open executes the same way when a later vote passes it.
    proposal._meta.onPass = onPass;
    if (decide) {
      this.recordVote(proposal, proposer, "agree", at, cmd, { onPass });
      if (proposal.status === "open") {
        this.decide(proposal, { ...decide, at: this.at(decide.at ?? at, at), onPass });
      }
    }
    return { proposal, cmd, itemId: id };
  }

  // `io_ticket_create` from the parent's holder; `offerTo` makes it `offered`.
  // `slug` names the convention branch; a locale build passes the `en` title's.
  createTicket({ key, parentId, title, brief = "", dueAt, offerTo = null, after = [], at, draft = null, creator = null, slug = slugify(title) }) {
    const parent = this.items.get(parentId);
    const holder = creator ?? keyName(parent.dri);
    const content = { title, brief, due_at: dueAt };
    if (after.length) content.after = after;
    const tags = [["u", parentId]];
    if (offerTo) tags.push(["p", personKey(offerTo).pubkey]);
    if (draft) tags.push(["e", draft.id, "", "draft"]);
    const cmd = this.command(holder, KIND.TICKET_CREATE, at, tags, content);
    if (draft) this.settleDraft(draft, cmd, at, "accepted");
    const id = this.uuid("item", key);
    const item = {
      id,
      parent: parentId,
      root: parent.root,
      depth: parent.depth + 1,
      path: [...parent.path, parentId],
      title,
      brief,
      state: offerTo ? "offered" : "open",
      dri: null,
      offered_to: offerTo ? personKey(offerTo).pubkey : null,
      offered_by: offerTo ? personKey(holder).pubkey : null,
      offered_at: offerTo ? at : null,
      due_at: dueAt,
      objective_ref: null,
      created_from: cmd.id,
      draft: draft?.id ?? null,
      done_receipt: null,
      closed_by: null,
      children: { open: 0, offered: 0, accepted: 0, done: 0 },
      branch: `io/${id.slice(0, 4)}-${slug.split("-").slice(0, 4).join("-")}`,
      ...(after.length ? { after } : {}),
    };
    this.items.set(id, item);
    this.emitItem(item, at, cmd.id);
    return { cmd, item };
  }

  offer(itemId, to, at, { by = null, draft = null } = {}) {
    const item = this.items.get(itemId);
    const offerer = by ?? (item.parent ? keyName(this.items.get(item.parent).dri) : keyName(this.shapers.shapers[0]));
    const tags = [["i", itemId], ["p", personKey(to).pubkey]];
    if (draft) tags.push(["e", draft.id, "", "draft"]);
    const cmd = this.command(offerer, KIND.OFFER, at, tags, {});
    if (draft) this.settleDraft(draft, cmd, at, "accepted");
    item.state = "offered";
    item.offered_to = personKey(to).pubkey;
    item.offered_by = draft ? "agent" : personKey(offerer).pubkey;
    item.offered_at = at;
    this.emitItem(item, at, cmd.id);
    return cmd;
  }

  accept(itemId, at) {
    const item = this.items.get(itemId);
    const who = keyName(item.offered_to);
    const cmd = this.command(who, KIND.ACCEPT, at, [["i", itemId]], {});
    item.state = "accepted";
    item.dri = item.offered_to;
    item.offered_to = null;
    item.offered_by = null;
    item.offered_at = null;
    this.emitItem(item, at, cmd.id);
    this.syncHome(item, who, at);
    return cmd;
  }

  // §6.7 membership sync: holders join the root's home room.
  syncHome(item, who, at) {
    const root = this.items.get(item.root);
    const roomKey = [...this.rooms.values()].find((r) => r.id === root.home?.channel)?.key;
    if (roomKey) this.joinRoom(roomKey, who, at);
  }

  decline(itemId, at) {
    const item = this.items.get(itemId);
    const who = keyName(item.offered_to);
    const cmd = this.command(who, KIND.DECLINE, at, [["i", itemId]], {});
    item.state = "open";
    item.offered_to = null;
    item.offered_by = null;
    item.offered_at = null;
    this.emitItem(item, at, cmd.id);
    return cmd;
  }

  // `io_done` from the holder — or, citing the holder's own message, from the
  // org agent (§5.5): `done_receipt` is then the message, not the command.
  done(itemId, at, { receipt = null, signer = null } = {}) {
    const item = this.items.get(itemId);
    const tags = [["i", itemId]];
    if (receipt) tags.push(["e", receipt.id, "", "receipt"]);
    const cmd = this.command(signer ?? keyName(item.dri), KIND.DONE, at, tags, {});
    item.state = "done";
    item.done_receipt = receipt ? receipt.id : cmd.id;
    item.closed_by = "dri";
    this.emitItem(item, at, cmd.id);
    return cmd;
  }

  // The §5.1 date rule: a root in its last fifth enters `in_review`.
  review(itemId, at) {
    const item = this.items.get(itemId);
    item.state = "in_review";
    return this.emitItem(item, at, this.itemEvents.get(itemId).tags.find((t) => t[0] === "receipt")[1]);
  }

  setDue(itemId, by, dueAt, at, why = null) {
    const item = this.items.get(itemId);
    const cmd = this.command(by, KIND.SET_DUE, at, [["i", itemId], ["due", String(dueAt)]], why ? { why } : {});
    item.due_at = dueAt;
    this.emitItem(item, at, cmd.id);
    return cmd;
  }

  // A `50102` from the holder's Work sync agent; the item's `last_progress` moves.
  progress(itemId, { summary, hint, commits, head, from, to, at, filesChanged = 0 }) {
    const item = this.items.get(itemId);
    const ref = `refs/heads/${item.branch}`;
    const tags = [["i", itemId], ["p", item.dri], ["ref", ref]];
    for (const c of commits) tags.push(["commit", c.sha]);
    tags.push(["hint", hint]);
    const content = {
      item: itemId,
      dri: item.dri,
      from,
      to,
      summary,
      hint,
      ref,
      head,
      commits,
      files_changed: filesChanged,
      uncommitted: null,
      merged_into: null,
    };
    const note = this.store(signEvent({ signer: personKey(keyName(item.dri)).name, kind: KIND.PROGRESS, createdAt: at, tags, content }));
    item.last_progress = note.id;
    this.emitItem(item, at, this.itemEvents.get(itemId).tags.find((t) => t[0] === "receipt")[1]);
    return note;
  }

  // ── §4.3 drafts and §4.6 outcomes ────────────────────────────────────────

  // An agent draft plus its `open` outcome at birth (§4.6).
  draft({ kind, needs, move, origin, gap, item = null, parent = null, suggested = null, receipts = [], skills = [], payload, at, signer = AGENT }) {
    const tags = [
      ["n", needs === "shaper" ? "shaper" : personKey(needs).pubkey],
      ["t", kind],
      ["move", String(move)],
      ["origin", origin],
      ["gap", gap],
    ];
    if (parent) tags.push(["u", parent]);
    if (item) tags.push(["i", item]);
    if (needs !== "shaper") tags.push(["p", personKey(needs).pubkey, "", "needs"]);
    if (suggested) tags.push(["p", personKey(suggested).pubkey, "", "suggested"]);
    for (const r of receipts) tags.push(r);
    if (suggested && this.profiles.has(personKey(suggested).pubkey)) {
      tags.push(["a", `39105:${this.relay}:${personKey(suggested).pubkey}`, "", "receipt"]);
      for (const s of skills) tags.push(["k", s]);
    }
    const ev = this.store(signEvent({ signer, kind: KIND.DRAFT, createdAt: at, tags, content: payload }));
    this.drafts.set(ev.id, { event: ev, kind, payload });
    const outcome = { draft: ev.id, status: "open", decided_by: null, decided_at: null, reason: null, result: null };
    this.outcomes.set(ev.id, outcome);
    this.state(KIND.OUTCOME, ev.id, at, [["s", "open"]], outcome);
    return ev;
  }

  settleDraft(draft, cmd, at, status) {
    const outcome = this.outcomes.get(draft.id);
    outcome.status = status;
    outcome.decided_by = cmd.pubkey;
    outcome.decided_at = at;
    outcome.result = cmd.id;
    this.state(KIND.OUTCOME, draft.id, at, [["s", status], ["p", cmd.pubkey]], outcome);
  }

  declineDraft(draft, by, reason, at) {
    const cmd = this.command(by, KIND.DRAFT_DECIDE, at, [["e", draft.id], ["outcome", "decline"], ["reason", reason]], {});
    const outcome = this.outcomes.get(draft.id);
    outcome.status = "declined";
    outcome.decided_by = cmd.pubkey;
    outcome.decided_at = at;
    outcome.reason = reason;
    outcome.result = cmd.id;
    this.state(KIND.OUTCOME, draft.id, at, [["s", "declined"], ["p", cmd.pubkey]], outcome);
    return cmd;
  }

  receiptFor(event) {
    return ["e", event.id, "", "receipt"];
  }

  roomReceipt(roomKey) {
    return ["a", `39000:${this.relay}:${this.rooms.get(roomKey).id}`, "", "receipt"];
  }

  itemReceipt(itemId) {
    return ["e", this.itemEvents.get(itemId).id, "", "receipt"];
  }

  // ── §4.7a profiles ───────────────────────────────────────────────────────

  setProfile(name, { about, skills, openLimit = null, at }) {
    const pk = personKey(name).pubkey;
    const content = { about, skills };
    if (openLimit !== null) content.open_limit = openLimit;
    const cmd = this.command(name, KIND.PROFILE_SET, at, [], content);
    const prev = this.profiles.get(pk);
    const profile = {
      pubkey: pk,
      version: (prev?.version ?? 0) + 1,
      about,
      skills: dedupeSkills(skills),
      open_limit: openLimit,
      updated_at: at,
      receipt: cmd.id,
    };
    this.profiles.set(pk, profile);
    this.state(
      KIND.PROFILE,
      pk,
      at,
      [["p", pk], ...profile.skills.map((s) => ["k", s.slug]), ["version", String(profile.version)]],
      profile,
    );
    return cmd;
  }

  // ── §4.7 health (gold, signed by the agent) ──────────────────────────────

  health({ itemId, week, pct, sentences, at }) {
    const band = bandFor(pct);
    const content = {
      item: itemId,
      week,
      pct: Math.round(pct) / 100,
      band,
      factors: [],
      sentences: sentences.map((text) => ({ text, rows: [] })),
      formula: "health-weights@1",
    };
    return signEvent({
      signer: AGENT,
      kind: KIND.HEALTH,
      createdAt: at,
      tags: [["i", itemId], ["week", week], ["band", band]],
      content,
    });
  }

  // Relay order: by `created_at`, ties in emission order (same transaction).
  sorted() {
    return this.events
      .map((ev, i) => [ev, i])
      .sort((a, b) => a[0].created_at - b[0].created_at || a[1] - b[1])
      .map(([ev]) => ev);
  }
}

function dedupeSkills(labels) {
  const seen = new Set();
  const out = [];
  for (const label of labels) {
    const slug = slugify(label);
    if (seen.has(slug)) continue;
    seen.add(slug);
    out.push({ slug, label });
  }
  return out;
}

const NAMES = new Map();
export function keyName(pubkey) {
  const hit = NAMES.get(pubkey);
  if (hit) return hit;
  throw new Error(`unknown pubkey ${pubkey}`);
}
export function registerNames(names) {
  for (const n of names) NAMES.set(personKey(n).pubkey, n === "You" ? "You" : n);
  NAMES.set(keyFor(READER).pubkey, "You");
  NAMES.set(keyFor(AGENT).pubkey, AGENT);
}
