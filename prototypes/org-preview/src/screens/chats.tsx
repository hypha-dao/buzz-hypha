'use client';

import {
  Bubble,
  ChoiceChips,
  Composer,
  ScrollArea,
  TypingDots,
} from '@/components/chat';
import {
  DIR_KINDS,
  HELP,
  TICKET_FOR_ME,
  TICKET_FOR_OTHER,
  asDirectionKind,
  driPeople,
  driProposalId,
  helpIdFrom,
  isMarkDone,
  matchOpenWork,
  openWorkWithoutDri,
  ticketYouHold,
  type HelpId,
  type OpenWork,
} from '@/lib/assist-flows';
import {
  AgentMark,
  Avatar,
  Button,
  Card,
  Chip,
  Kicker,
  cn,
} from '@/components/primitives';
import { Workspace } from '@/components/workspace';
import {
  agreedPay,
  energyOrg,
  personaName,
  seedMessages,
  space,
  threads,
  unitFor,
  type DirectionKind,
  type Msg,
  type OrgId,
  type ProjectId,
} from '@/lib/data';
import {
  useStore,
  PAY_LEA_ID,
  PAY_ROGERIO_ID,
  SUB_COVERS_TITLE,
} from '@/lib/store';
import { useState } from 'react';

let n = 0;
const uid = () => `x${++n}`;

/** "150", "1,500 EURC", "200 usdc" → the number; none → undefined */
function amountIn(text: string): number | undefined {
  const m = text.match(/(\d[\d,]*)\s*(usdc|eurc)?\b/i);
  if (!m) return undefined;
  const v = Number(m[1].replace(/,/g, ''));
  return Number.isFinite(v) && v > 0 ? v : undefined;
}

/**
 * The one way money is asked for: "draft a proposal for my work — 150",
 * "…whatever we agreed", "let's pay Lea what we agreed", "pay me".
 */
const payAsk =
  /\b(pay|paid|payment|proposal for (my|the|his|her)|whatever we agreed|what we agreed|owe)\b/i;

function ticketTitle(text: string) {
  let t = text
    .replace(/^\s*(create|make|add|open)\s+(a\s+)?ticket\s*(to|for|:)?\s*/i, '')
    .replace(/^\s*we\s+(need|should)\s+(a\s+|to\s+)?/i, '')
    .replace(/[.!\s]+$/, '')
    .trim();
  if (!t) t = text.trim();
  return t.charAt(0).toUpperCase() + t.slice(1);
}

/** "how does pay work here?" — a question about the model, not a request to pay */
const payHow =
  /\b(salary|salaries|monthly|hourly|how (do|does|am|is|will|would).*(pay|paid)|when (do|does|am|will).*(pay|paid)|get paid|paid for|pay(ment)? (work|model|system))\b/i;

function payHowReply(org: OrgId, persona: string): string {
  const a = agreedPay[org];
  const unit = unitFor(org);
  const shapers = org === 'energy' ? 'the three Shapers' : 'both Shapers';
  const money = `${a.amount.toLocaleString()} ${unit}`;
  const mine =
    persona === 'lea'
      ? ` Yours: you and ${a.withWhom} agreed ${money} for ${a.work} in “${a.roomName}” on ${a.when} — I have the line. When the ticket is done, tell me “draft a proposal for my work” and name the sum, or say “whatever we agreed”.`
      : persona === 'sam'
      ? ` Under you: ${a.who} and you agreed ${money} for ${a.work} in “${a.roomName}” on ${a.when}. When it is done, either of you asks me to draft the proposal. Your own pay for holding the project is between you and the Shapers — same move, same room.`
      : persona === 'you'
      ? org === 'energy'
        ? ' Nothing is agreed for your summary ticket yet. If it should pay, say a number to Marcus in “Pilots” — I remember the line, and when the ticket is done you ask me to draft the proposal.'
        : ' Nothing is agreed for your setup ticket — Sam offered it as a favour. If you think it should pay, say a number to Sam in “Saturday stall”; I remember the line, and when the ticket is done you ask me to draft the proposal.'
      : persona === 'maya'
      ? ' As a Shaper you see every pay proposal on Decisions and on My Work — you agree or not. Sums are never on tickets; they are in the rooms and in the proposals.'
      : '';
  return (
    `No salaries, no invoices, no numbers on tickets or projects. Pay is agreed where you already talk — a ticket holder with the project DRI, or a DRI with a Shaper — and I remember the line. When the work is done, anyone involved tells me “draft a proposal for the Shapers for this work” with a sum, or “whatever we agreed”, and I draft it with the agreement and the done receipt attached. ${shapers} agree, the money moves, and it shows on the profile. I never move money myself.` +
    mine
  );
}

/** what the assistant says when someone asks for pay before the work is done */
function notYetReply(org: OrgId): string {
  const a = agreedPay[org];
  return `${a.who}’s ${a.work} ticket is not confirmed done yet. Say it is done in “${a.roomName}” or here and I draft the done for ${a.who} to confirm — then ask me again and I draft the pay proposal with the agreed line attached.`;
}

/* =========================================================
   Thread — with agent cards, receipts, and scripted replies
   ========================================================= */

type ThreadInfo = {
  id: string;
  kind: 'agent' | 'shapers' | 'room' | 'dm';
  title: string;
  sub: string;
};

export function Thread() {
  const s = useStore();
  const custom = s.customChats.find((c) => c.id === s.threadId);
  const t: ThreadInfo =
    (threads.find((x) => x.id === s.threadId) as ThreadInfo | undefined) ??
    (energyOrg.threads.find((x) => x.id === s.threadId) as
      | ThreadInfo
      | undefined) ??
    (custom
      ? {
          id: custom.id,
          kind: 'room',
          title: custom.title,
          sub: 'room you started · feeds the org like any other',
        }
      : (threads[0] as ThreadInfo));
  const [agentTyping, setAgentTyping] = useState(false);
  const [awaiting, setAwaiting] = useState<
    | null
    | 'direction'
    | 'ticket'
    | 'ticket-who'
    | 'dri-work'
    | 'dri-who'
    | 'dri-name'
  >(null);
  const [driWork, setDriWork] = useState<OpenWork | null>(null);
  // the assistant is one thread in the sidebar but keeps a history per org
  const key = t.kind === 'agent' ? s.agentKey : t.id;
  const msgs: Msg[] = [
    ...(seedMessages[key] ?? []),
    ...(s.extraMsgs[key] ?? []),
  ];

  function agentReply(text: string, extra?: Partial<Msg>) {
    setAgentTyping(true);
    setTimeout(() => {
      setAgentTyping(false);
      s.sendMsg(key, { id: uid(), from: 'agent', text, ...extra });
    }, 950);
  }

  function later(fn: () => void) {
    setAgentTyping(true);
    setTimeout(() => {
      setAgentTyping(false);
      fn();
    }, 1100);
  }

  function runHelp(id: HelpId) {
    if (id === 'direction') {
      setAwaiting('direction');
      agentReply(
        'Which of the four — mission, vision, objectives, or strategy?',
      );
      return;
    }
    if (id === 'project') {
      if (s.assistProject?.org === s.org && s.assistProject.opened) {
        agentReply(
          'That project proposal is already open — the Shapers are deciding. It is on Decisions.',
        );
        return;
      }
      if (s.assistProject?.org === s.org) {
        agentReply(
          'The draft is above — open it as a proposal when you are ready.',
        );
        return;
      }
      later(() => s.draftAssistProject());
      return;
    }
    if (id === 'money') {
      const livePayOpen = (
        s.org === 'energy' ? s.eProposals : s.proposals
      ).some(
        (p) => p.id === (s.org === 'energy' ? PAY_ROGERIO_ID : PAY_LEA_ID),
      );
      const workDone =
        s.org === 'energy' ? s.eMuni === 'done' : s.covers === 'done';
      if (livePayOpen) {
        agentReply(
          'A pay proposal is already open — the Shapers are deciding. It is on Decisions.',
        );
        return;
      }
      if (s.org === 'energy' ? s.ePayDraft : s.payDraft) {
        agentReply(
          'The draft is above — open it as a proposal when you are ready. I never move money.',
        );
        return;
      }
      if (workDone && (s.persona === 'lea' || s.persona === 'sam')) {
        later(() => s.draftPayment());
        return;
      }
      if (s.assistPay?.org === s.org && s.assistPay.opened) {
        agentReply(
          'That pay proposal is already open — the Shapers are deciding. It is on Decisions.',
        );
        return;
      }
      if (s.assistPay?.org === s.org) {
        agentReply(
          'The draft is above — open it as a proposal when you are ready.',
        );
        return;
      }
      later(() => s.draftAssistPay());
      return;
    }
    if (id === 'done') {
      if (s.org === 'energy') {
        if (s.persona === 'lea' && s.eMuni === 'doing') {
          later(() => s.triggerMuniDone('Mark my ticket done', 'e-agent'));
          return;
        }
        if (s.persona === 'lea' && s.eMuni === 'done') {
          agentReply('That ticket is already done — the receipt is on it.');
          return;
        }
        if (s.persona === 'you' && s.eSummary === 'accepted') {
          agentReply(
            'Your Ameland summary ticket. You hold it, so you confirm — the project DRI does not need to agree.',
            { card: 'you-done-draft' },
          );
          return;
        }
        if (s.persona === 'you' && s.eSummary === 'done') {
          agentReply('Your summary ticket is already done.');
          return;
        }
      } else {
        if (s.persona === 'lea' && s.covers === 'accepted') {
          later(() => s.triggerDoneDraft('Mark my ticket done', 'agent'));
          return;
        }
        if (s.persona === 'lea' && s.covers === 'done') {
          agentReply('That ticket is already done — the receipt is on it.');
          return;
        }
        if (s.persona === 'you' && s.setup === 'accepted') {
          agentReply(
            'Your setup ticket. You hold it, so you confirm — the project DRI does not need to agree.',
            { card: 'you-done-draft' },
          );
          return;
        }
        if (s.persona === 'you' && s.setup === 'done') {
          agentReply('Your setup ticket is already done.');
          return;
        }
      }
      agentReply(
        'I can only mark done a ticket you hold. You do not hold one that is open. I can draft a ticket for you instead.',
      );
      return;
    }
    if (id === 'ticket') {
      setAwaiting('ticket');
      agentReply('For you, or for someone else?');
      return;
    }
    if (id === 'dri') {
      const works = openWorkWithoutDri(s.org, s);
      if (works.length === 0) {
        agentReply('Everything that is open already has a DRI.');
        return;
      }
      const list = s.org === 'energy' ? s.eProposals : s.proposals;
      if (list.some((p) => p.id.startsWith('dri-') && p.state === 'open')) {
        agentReply(
          'A DRI proposal is already open — the Shapers are deciding. It is on Decisions.',
        );
        return;
      }
      if (s.assistDri?.org === s.org && !s.assistDri.opened) {
        agentReply(
          'The draft is above — open it as a proposal when you are ready.',
        );
        return;
      }
      if (works.length === 1) {
        setDriWork(works[0]);
        setAwaiting('dri-who');
        agentReply(`${works[0].work} has no DRI. Yourself, or someone else?`);
        return;
      }
      setAwaiting('dri-work');
      agentReply('Which of these has no DRI should this name someone for?');
      return;
    }
    // ask
    if (s.org === 'energy') {
      agentReply(
        'Yes. Ameland ran the tokenised credits through a full sandbox cycle — Marcus posted the report an hour ago and the ticket is done. The EECF second call opens in spring; Pedro wants six communities ready, and the 6,000 EURC to support their applications is an open proposal with the Shapers. Receipts below.',
        {
          receipts: [
            {
              label: 'Marcus — “Ameland report is live”',
              go: 'thread',
              id: 'e-pilots',
            },
            {
              label: 'Proposal — EECF round 2, open',
              go: 'proposal',
              id: 'e-eecf',
            },
            { label: 'Project — Island grids', go: 'project', id: 'islands' },
          ],
        },
      );
      return;
    }
    agentReply(
      s.weekday === 'held'
        ? 'Yes — twice. The council licence came up in “Saturday stall”; nobody held it, so a project was drafted and Rafi holds Weekday hall now. And the strategy says no brand money. Receipts below.'
        : 'Yes — twice. Jun raised the council licence in “Saturday stall”; nobody held it, so I drafted the Weekday hall project — it is with the Shapers now. And the strategy says no brand money. Receipts below.',
      {
        receipts: [
          {
            label: 'Jun — “Who signs the weekday hall licence?”',
            go: 'thread',
            id: 'saturday',
          },
          {
            label: 'Strategy — no brand money',
            go: 'proposal',
            id: 'dir-strategy-v4',
          },
          {
            label: 'Project — Weekday hall',
            go: 'project',
            id: 'weekday',
          },
        ],
      },
    );
  }

  function handleDriAwaiting(text: string): boolean {
    if (awaiting === 'dri-work') {
      const works = openWorkWithoutDri(s.org, s);
      const hit = matchOpenWork(text, works);
      if (hit) {
        setDriWork(hit);
        setAwaiting('dri-who');
        agentReply(`${hit.work} has no DRI. Yourself, or someone else?`);
        return true;
      }
      agentReply(`Say one of: ${works.map((w) => w.work).join(', ')}.`);
      return true;
    }
    if (awaiting === 'dri-who') {
      const work = driWork ?? openWorkWithoutDri(s.org, s)[0];
      if (!work) {
        setAwaiting(null);
        agentReply('Everything that is open already has a DRI.');
        return true;
      }
      const me = personaName(s.org, s.persona);
      const names = driPeople(s.org, me).filter((n) => n !== me);
      const named = names.find((n) =>
        text.toLowerCase().includes(n.toLowerCase()),
      );
      if (/\bmyself\b|^me$|\bfor me\b|\bi (will|can) sit\b/i.test(text)) {
        setAwaiting(null);
        later(() => s.draftAssistDri(work.work, work.workKind, me));
        return true;
      }
      if (named) {
        setAwaiting(null);
        later(() => s.draftAssistDri(work.work, work.workKind, named));
        return true;
      }
      if (/someone else|somebody else|another/i.test(text)) {
        setAwaiting('dri-name');
        agentReply('Who should hold it?');
        return true;
      }
      agentReply('Myself, or someone else?');
      return true;
    }
    if (awaiting === 'dri-name') {
      const work = driWork ?? openWorkWithoutDri(s.org, s)[0];
      const me = personaName(s.org, s.persona);
      const names = driPeople(s.org, me).filter((n) => n !== me);
      const who =
        names.find((n) => text.toLowerCase().includes(n.toLowerCase())) ??
        names.find((n) => n.toLowerCase() === text.trim().toLowerCase());
      if (who && work) {
        setAwaiting(null);
        later(() => s.draftAssistDri(work.work, work.workKind, who));
        return true;
      }
      agentReply(`Say one of: ${names.join(', ')}.`);
      return true;
    }
    return false;
  }

  function send(text: string) {
    s.sendMsg(key, { id: uid(), from: 'you', text });
    if (t.kind === 'shapers') {
      const help = helpIdFrom(text);
      if (help === 'dri') {
        setAwaiting(null);
        runHelp(help);
        return;
      }
      handleDriAwaiting(text);
      return;
    }
    if (t.kind === 'agent') {
      const help = helpIdFrom(text);
      if (help) {
        setAwaiting(null);
        runHelp(help);
        return;
      }
      if (handleDriAwaiting(text)) return;
      if (awaiting === 'direction') {
        const kind = asDirectionKind(text);
        setAwaiting(null);
        if (kind) {
          later(() => s.draftAssistDirection(kind));
          return;
        }
        agentReply(
          'Say mission, vision, objectives, or strategy and I will draft that proposal.',
        );
        setAwaiting('direction');
        return;
      }
      if (awaiting === 'ticket') {
        const other =
          /\bsomeone else\b|for (jun|lea|tom|priya|rowan|rogerio|suzana|inês|ines)\b/i.test(
            text,
          );
        const mine = /\bfor (me|myself|you)\b|^for me$/i.test(text);
        if (mine || (!other && /^for me/i.test(text))) {
          setAwaiting(null);
          const under = ticketYouHold(s.org, s.persona, s);
          later(() =>
            s.draftChatTicket(
              TICKET_FOR_ME[s.org],
              undefined,
              under ?? undefined,
            ),
          );
          return;
        }
        if (other || /^for someone else/i.test(text)) {
          setAwaiting('ticket-who');
          agentReply('Who should it be for?');
          return;
        }
        setAwaiting(null);
        later(() => s.draftChatTicket(ticketTitle(text)));
        return;
      }
      if (awaiting === 'ticket-who') {
        const names = Object.keys(TICKET_FOR_OTHER[s.org]);
        const who =
          names.find((n) => text.toLowerCase().includes(n.toLowerCase())) ??
          names.find((n) => n.toLowerCase() === text.trim().toLowerCase());
        setAwaiting(null);
        if (who) {
          const under = ticketYouHold(s.org, s.persona, s);
          later(() =>
            s.draftChatTicket(
              TICKET_FOR_OTHER[s.org][who],
              who,
              under ?? undefined,
            ),
          );
          return;
        }
        agentReply(`Say one of: ${names.join(', ')}.`);
        setAwaiting('ticket-who');
        return;
      }
    }
    if (s.org === 'energy') return sendEnergy(text);

    const splitish = /\b(print|rota|could you|can you take|piece)\b/i.test(
      text,
    );

    // ---- rooms: the org listens ----
    if (t.id === 'saturday') {
      // Lea asks Jun for a piece of her ticket — heard, drafted under hers
      if (splitish && s.persona === 'lea' && s.covers === 'accepted') {
        if (s.subCovers === 'none') later(() => s.draftSubTicket('saturday'));
        return;
      }
      if (/\b(done|finished|found|covered|got (both|two))\b/i.test(text)) {
        if (s.covers === 'accepted') {
          setAgentTyping(true);
          setTimeout(() => {
            setAgentTyping(false);
            s.triggerDoneDraft(text);
          }, 1100);
        }
      }
      return;
    }
    if (t.kind !== 'agent') return;

    // ---- the assistant thread ----
    if (payHow.test(text)) {
      agentReply(payHowReply('river', s.persona), {
        receipts: [
          {
            label: 'Where Lea and Sam agreed the covers pay — “Saturday stall”',
            go: 'thread',
            id: 'saturday',
          },
          {
            label: 'Proposal — Lea, four Saturdays hosted',
            go: 'proposal',
            id: 'lea-saturdays',
          },
          {
            label: 'Proposal — Priya, voucher design',
            go: 'proposal',
            id: 'pay-priya',
          },
        ],
      });
      return;
    }
    const payish = payAsk.test(text);
    const askish =
      /\b(council|licen[cs]e|hall|before|dealt|history|happened|sponsor)\b/i.test(
        text,
      );
    const doneish = /\b(done|finished|found|covered)\b/i.test(text);
    const worky =
      /\b(need|should|fix|broken|help|ticket|project)\b/i.test(text) &&
      !isMarkDone(text);

    if (isMarkDone(text)) {
      runHelp('done');
      return;
    }

    // move your own work: Lea says her ticket is done, right here
    if (doneish && s.persona === 'lea' && s.covers === 'accepted') {
      setAgentTyping(true);
      setTimeout(() => {
        setAgentTyping(false);
        s.triggerDoneDraft(text, 'agent');
      }, 1100);
      return;
    }

    if (payish) {
      if (s.proposals.some((p) => p.id === PAY_LEA_ID)) {
        agentReply(
          'That proposal is already open — the Shapers are deciding. It is on the Decisions page.',
        );
      } else if (s.payDraft) {
        agentReply(
          'The draft is above — open it as a proposal when you are ready. I never move money.',
        );
      } else if (s.persona === 'you') {
        agentReply(
          'Nothing is agreed for your setup ticket — Sam offered it as a favour. If you think it should pay, say a number to Sam in “Saturday stall”; I remember the line, and when the ticket is done you ask me to draft the proposal.',
        );
      } else if (s.covers === 'done') {
        const amount = amountIn(text);
        later(() => s.draftPayment(amount));
      } else {
        agentReply(notYetReply('river'));
      }
      return;
    }

    // split your own work: Lea holds covers, so a need she voices lands
    // under her ticket — the nearest thing she holds — not with Sam
    if ((splitish || worky) && s.persona === 'lea' && s.covers !== 'done') {
      if (s.subCovers === 'none') {
        later(() => s.draftSubTicket());
      } else {
        agentReply(
          s.subCovers === 'drafted'
            ? 'The draft is above — offer it to Jun when you are ready.'
            : s.subCovers === 'offered'
            ? 'Offered to Jun — his yes or no, nobody else’s.'
            : s.subCovers === 'accepted'
            ? 'Jun holds the rota, under your covers ticket. Yours cannot close until his piece does — done moves up the tree, never down.'
            : 'Jun printed the rota — done, receipt in “Saturday stall”. Your covers ticket can close now.',
        );
      }
      return;
    }

    if (askish) {
      agentReply(
        s.weekday === 'held'
          ? 'Yes — twice. The council licence came up in “Saturday stall”; nobody held it, so a project was drafted and Rafi holds Weekday hall now. And the strategy says no brand money. Receipts below.'
          : 'Yes — twice. Jun raised the council licence in “Saturday stall”; nobody held it, so I drafted the Weekday hall project — it is with the Shapers now. And the strategy says no brand money. Receipts below.',
        {
          receipts: [
            {
              label: 'Jun — “Who signs the weekday hall licence?”',
              go: 'thread',
              id: 'saturday',
            },
            {
              label: 'Strategy — no brand money',
              go: 'proposal',
              id: 'dir-strategy-v4',
            },
            {
              label: 'Project — Weekday hall',
              go: 'project',
              id: 'weekday',
            },
          ],
        },
      );
      return;
    }

    if (worky) {
      if (s.chatTicket?.org === 'river') {
        agentReply(
          s.chatTicket.state === 'created'
            ? 'That ticket exists — open, under Saturday stall. It is on Projects.'
            : s.chatTicket.state === 'routed'
            ? 'The draft is with Sam — it exists when he confirms.'
            : 'The draft is above — create it, or send it to Sam.',
        );
        return;
      }
      later(() => s.draftChatTicket(ticketTitle(text)));
      return;
    }

    agentReply(
      'Noted — it is in the record. If it ever needs work or a decision, I will draft the card and the right person says yes or no.',
    );
  }

  /* ---- Hypha Energy: same assistant, different rooms and receipts ---- */
  function sendEnergy(text: string) {
    const doneish = /\b(done|finished|signed|onboarded|both)\b/i.test(text);

    if (t.id === 'e-pilots') {
      if (doneish && s.persona === 'lea' && s.eMuni === 'doing')
        later(() => s.triggerMuniDone(text));
      return;
    }
    if (t.kind !== 'agent') return;

    if (payHow.test(text)) {
      agentReply(payHowReply('energy', s.persona), {
        receipts: [
          {
            label:
              'Where Rogerio and Pedro agreed the municipalities pay — “Pilots”',
            go: 'thread',
            id: 'e-pilots',
          },
          {
            label: 'Proposal — Pedro, held Iberia through May',
            go: 'proposal',
            id: 'e-pedro-stipend',
          },
          {
            label: 'Proposal — Rowan, battery optimisation v2',
            go: 'proposal',
            id: 'e-pay-rowan',
          },
        ],
      });
      return;
    }

    const payish = payAsk.test(text);
    const askish =
      /\b(ameland|sandbox|eecf|grant|nordpool|portug|coop[eé]rnico|carbon|credits|who holds|hubs|white ?paper|research)\b/i.test(
        text,
      );
    const worky =
      /\b(need|should|fix|broken|help|ticket|project)\b/i.test(text) &&
      !isMarkDone(text);

    if (isMarkDone(text)) {
      runHelp('done');
      return;
    }

    // Rogerio moves his own work from here
    if (doneish && s.persona === 'lea' && s.eMuni === 'doing') {
      later(() => s.triggerMuniDone(text, 'e-agent'));
      return;
    }

    if (payish) {
      if (s.eProposals.some((p) => p.id === PAY_ROGERIO_ID)) {
        agentReply(
          'That proposal is already open — the three Shapers are deciding. It is on the Decisions page.',
        );
      } else if (s.ePayDraft) {
        agentReply(
          'The draft is above — open it as a proposal when you are ready. I never move money.',
        );
      } else if (s.persona === 'you') {
        agentReply(
          'Nothing is agreed for your summary ticket yet. If it should pay, say a number to Marcus in “Pilots” — I remember the line, and when the ticket is done you ask me to draft the proposal.',
        );
      } else if (s.eMuni === 'done') {
        const amount = amountIn(text);
        later(() => s.draftPayment(amount));
      } else {
        agentReply(notYetReply('energy'));
      }
      return;
    }

    if (askish) {
      agentReply(
        'Yes. Ameland ran the tokenised credits through a full sandbox cycle — Marcus posted the report an hour ago and the ticket is done. The EECF second call opens in spring; Pedro wants six communities ready, and the 6,000 EURC to support their applications is an open proposal with the Shapers. Receipts below.',
        {
          receipts: [
            {
              label: 'Marcus — “Ameland report is live”',
              go: 'thread',
              id: 'e-pilots',
            },
            {
              label: 'Proposal — EECF round 2, open',
              go: 'proposal',
              id: 'e-eecf',
            },
            { label: 'Project — Island grids', go: 'project', id: 'islands' },
          ],
        },
      );
      return;
    }

    if (worky) {
      if (s.chatTicket?.org === 'energy') {
        agentReply(
          s.chatTicket.state === 'created'
            ? 'That ticket exists — open, under Iberia pilots. It is on Projects.'
            : s.chatTicket.state === 'routed'
            ? 'The draft is with Pedro — it exists when he confirms.'
            : 'The draft is above — create it, or send it to Pedro.',
        );
        return;
      }
      later(() => s.draftChatTicket(ticketTitle(text)));
      return;
    }

    agentReply(
      'Noted — it is in the record. If it ever needs work or a decision, I will draft the card and the right person says yes or no.',
    );
  }

  const suggestions: string[] =
    s.org === 'energy'
      ? t.kind === 'agent'
        ? [
            ...(s.persona === 'lea' && s.eMuni === 'doing'
              ? ['Both municipalities signed — done.']
              : []),
            ...(s.persona === 'lea' && s.eMuni === 'done' && !s.ePayDraft
              ? [
                  'Draft a proposal for the Shapers for my municipalities work — whatever we agreed.',
                ]
              : []),
            ...(s.persona === 'sam' && s.eMuni === 'done' && !s.ePayDraft
              ? [
                  'Rogerio finished the municipalities — draft the pay proposal, what we agreed.',
                ]
              : []),
          ]
        : t.id === 'e-pilots' && s.persona === 'lea' && s.eMuni === 'doing'
        ? ['Both municipalities signed — done.']
        : []
      : t.kind === 'agent'
      ? [
          ...(s.persona === 'lea' && s.covers === 'accepted'
            ? [
                ...(s.subCovers === 'none'
                  ? ['Jun, could you print the Saturday cover rota?']
                  : []),
                'Found both covers — Priya and Tom. Done.',
              ]
            : []),
          ...(s.persona === 'lea' && s.covers === 'done' && !s.payDraft
            ? [
                'Draft a proposal for the Shapers for my covers work — whatever we agreed.',
                'Draft a proposal for my covers work — 200 USDC.',
              ]
            : []),
          ...(s.persona === 'sam' && s.covers === 'done' && !s.payDraft
            ? [
                'Lea finished the covers — draft the pay proposal, what we agreed.',
              ]
            : []),
        ]
      : t.id === 'saturday' && s.persona === 'lea' && s.covers === 'accepted'
      ? [
          ...(s.subCovers === 'none'
            ? ['Jun, could you print the Saturday cover rota?']
            : []),
          'Found both covers — Priya and Tom. Done.',
        ]
      : [];

  return (
    <Workspace>
      <header className="flex shrink-0 items-center gap-3 border-b border-hair px-4 py-3 md:px-6">
        {t.kind === 'agent' ? (
          <AgentMark size={14} />
        ) : (
          <Avatar name={t.title} size="sm" />
        )}
        <div>
          <p className="text-[14px] font-semibold tracking-[-0.01em]">
            {t.kind === 'agent'
              ? `Personal Assistant (${
                  s.org === 'energy' ? energyOrg.space.name : space.name
                })`
              : t.title}
          </p>
          {t.kind !== 'agent' && t.kind !== 'shapers' && (
            <p className="text-[12px] text-faint">{t.sub}</p>
          )}
        </div>
      </header>

      <ScrollArea
        deps={[
          msgs.length,
          agentTyping,
          s.covers,
          s.subCovers,
          s.strategyPending,
          s.eStrategyPending,
          s.eMuni,
          awaiting,
          s.assistDir,
          s.assistProject,
          s.assistPay,
          s.assistDri,
        ]}
      >
        {msgs.map((m) => (
          <MsgBlock
            key={m.id}
            msg={m}
            agentName={
              t.kind === 'agent'
                ? `Personal Assistant (${
                    s.org === 'energy' ? energyOrg.space.name : space.name
                  })`
                : s.org === 'energy'
                ? 'Hypha Energy'
                : 'River Commons'
            }
            agentRole={t.kind === 'agent' ? '' : 'agent'}
            onHelp={send}
          />
        ))}
        {agentTyping && <TypingDots />}
        {t.kind === 'agent' && !agentTyping && awaiting === 'direction' && (
          <ChoiceChips
            options={DIR_KINDS.map((k) =>
              k === 'objectives'
                ? 'Objectives'
                : k[0].toUpperCase() + k.slice(1),
            )}
            onPick={(v) => send(v)}
          />
        )}
        {t.kind === 'agent' && !agentTyping && awaiting === 'ticket' && (
          <ChoiceChips
            options={['For me', 'For someone else']}
            onPick={(v) => send(v)}
          />
        )}
        {t.kind === 'agent' && !agentTyping && awaiting === 'ticket-who' && (
          <ChoiceChips
            options={Object.keys(TICKET_FOR_OTHER[s.org])}
            onPick={(v) => send(v)}
          />
        )}
        {(t.kind === 'agent' || t.kind === 'shapers') &&
          !agentTyping &&
          awaiting === 'dri-work' && (
            <ChoiceChips
              options={openWorkWithoutDri(s.org, s).map((w) => w.work)}
              onPick={(v) => send(v)}
            />
          )}
        {(t.kind === 'agent' || t.kind === 'shapers') &&
          !agentTyping &&
          awaiting === 'dri-who' && (
            <ChoiceChips
              options={['Myself', 'Someone else']}
              onPick={(v) => send(v)}
            />
          )}
        {(t.kind === 'agent' || t.kind === 'shapers') &&
          !agentTyping &&
          awaiting === 'dri-name' && (
            <ChoiceChips
              options={driPeople(s.org, personaName(s.org, s.persona)).filter(
                (n) => n !== personaName(s.org, s.persona),
              )}
              onPick={(v) => send(v)}
            />
          )}
      </ScrollArea>

      {suggestions.length > 0 && (
        <div className="flex flex-wrap gap-2 px-4 pb-1 md:px-6">
          {suggestions.map((sg) => (
            <button
              key={sg}
              type="button"
              onClick={() => send(sg)}
              className="rounded-full border border-hair bg-paper px-3.5 py-1.5 text-[13px] text-sub transition-colors hover:border-ink hover:text-ink"
            >
              {sg}
            </button>
          ))}
        </div>
      )}

      <Composer
        onSend={send}
        placeholder={
          t.kind === 'agent'
            ? 'Ask, or say what should happen…'
            : `Message ${t.title}…`
        }
      />
    </Workspace>
  );
}

/* ---------- message + attached card / receipts ---------- */

function MsgBlock({
  msg,
  agentName,
  agentRole,
  onHelp,
}: {
  msg: Msg;
  agentName: string;
  agentRole: string;
  onHelp: (text: string) => void;
}) {
  const s = useStore();
  return (
    <div className="flex flex-col gap-2.5">
      <Bubble msg={msg} agentName={agentName} agentRole={agentRole} />
      {msg.receipts && (
        <div className="flex flex-wrap gap-2 pl-11">
          {msg.receipts.map((r) => (
            <button
              key={r.label}
              type="button"
              onClick={() => {
                if (r.go === 'thread') s.openThread(r.id);
                else if (r.go === 'proposal') s.openProposal(r.id);
                else s.openProject(r.id as ProjectId);
              }}
              className="rounded-full border border-hair bg-paper px-3 py-1.5 text-[12px] font-medium text-sub transition-colors hover:border-agent hover:text-agent"
            >
              {r.label} →
            </button>
          ))}
        </div>
      )}
      {msg.card === 'assistant-help' && <AssistantHelpCard onPick={onHelp} />}
      {msg.card === 'payment-draft' && <PaymentDraftCard />}
      {msg.card === 'assist-pay' && <AssistPayCard />}
      {msg.card === 'done-draft' && <DoneDraftCard />}
      {msg.card === 'e-done-draft' && <EnergyDoneDraftCard />}
      {msg.card === 'you-done-draft' && <YouDoneDraftCard />}
      {msg.card === 'direction-draft' && <DirectionDraftCard msg={msg} />}
      {msg.card === 'assist-direction' && <AssistDirectionCard />}
      {msg.card === 'project-draft' && <ProjectDraftCard />}
      {msg.card === 'ticket-draft' && <TicketDraftCard />}
      {msg.card === 'sub-ticket-draft' && <SubTicketDraftCard />}
      {msg.card === 'dri-draft' && <DriDraftCard msg={msg} />}
    </div>
  );
}

function AssistantHelpCard({ onPick }: { onPick: (prompt: string) => void }) {
  return (
    <Card className="ml-11 max-w-lg p-4">
      <ol className="space-y-1.5">
        {HELP.map((h, i) => (
          <li key={h.id}>
            <button
              type="button"
              onClick={() => onPick(h.prompt)}
              className="w-full rounded-xl px-2 py-1.5 text-left text-[14px] leading-snug text-ink transition-colors hover:bg-wash"
            >
              <span className="text-faint">{i + 1}. </span>
              {h.label}
            </button>
          </li>
        ))}
      </ol>
    </Card>
  );
}

function AssistDirectionCard() {
  const s = useStore();
  const d = s.assistDir;
  if (!d || d.org !== s.org) return null;
  return (
    <Card className={cn('ml-11 max-w-md p-4', !d.opened && 'border-agent/30')}>
      <div className="flex items-center justify-between gap-3">
        <Kicker className={d.opened ? undefined : 'text-agent'}>
          Proposal draft — {d.kind === 'objectives' ? 'objective' : d.kind}
        </Kicker>
        {d.opened && <Chip>opened</Chip>}
      </div>
      <p className="mt-2 text-[14px] font-medium leading-relaxed">
        “{d.quote}”
      </p>
      <p className="mt-1.5 text-[13px] leading-relaxed text-sub">{d.note}</p>
      <div className="mt-3 flex gap-2">
        {d.opened ? (
          <Button
            size="sm"
            variant="soft"
            onClick={() => s.openProposal(`assist-dir-${d.kind}`)}
          >
            See the vote →
          </Button>
        ) : (
          <>
            <Button size="sm" onClick={s.openAssistDirection}>
              Open as a proposal
            </Button>
            <Button
              size="sm"
              variant="ghost"
              onClick={() =>
                s.toast('Amended in place — wording. Still a draft.')
              }
            >
              Amend
            </Button>
          </>
        )}
      </div>
    </Card>
  );
}

function ProjectDraftCard() {
  const s = useStore();
  const d = s.assistProject;
  if (!d || d.org !== s.org) return null;
  return (
    <Card className={cn('ml-11 max-w-md p-4', !d.opened && 'border-agent/30')}>
      <div className="flex items-center justify-between gap-3">
        <Kicker className={d.opened ? undefined : 'text-agent'}>
          Proposal draft — project
        </Kicker>
        {d.opened && <Chip>opened</Chip>}
      </div>
      <p className="mt-2 text-[15px] font-semibold tracking-[-0.015em]">
        {d.title}
      </p>
      <p className="mt-1.5 text-[13px] leading-relaxed text-sub">
        {d.description}
      </p>
      <p className="mt-1 text-[12px] text-faint">Ends {d.ends}</p>
      <div className="mt-3 flex gap-2">
        {d.opened ? (
          <Button
            size="sm"
            variant="soft"
            onClick={() => s.openProposal('assist-project')}
          >
            See the vote →
          </Button>
        ) : (
          <>
            <Button size="sm" onClick={s.openAssistProject}>
              Open as a proposal
            </Button>
            <Button
              size="sm"
              variant="ghost"
              onClick={() =>
                s.toast('Amended in place — title, review. Still a draft.')
              }
            >
              Amend
            </Button>
          </>
        )}
      </div>
    </Card>
  );
}

function AssistPayCard() {
  const s = useStore();
  const d = s.assistPay;
  if (!d || d.org !== s.org) return null;
  const unit = unitFor(s.org);
  return (
    <Card className={cn('ml-11 max-w-md p-4', !d.opened && 'border-agent/30')}>
      <div className="flex items-center justify-between gap-3">
        <Kicker className={d.opened ? undefined : 'text-agent'}>
          Proposal draft — money
        </Kicker>
        {d.opened && <Chip>opened</Chip>}
      </div>
      <p className="mt-2 text-[15px] font-semibold tracking-[-0.015em]">
        Pay {d.who} {d.amount.toLocaleString()} {unit} — {d.work}
      </p>
      <p className="mt-1.5 text-[13px] leading-relaxed text-sub">
        Agreed with {d.withWhom} in “{d.room}”. I never move money.
      </p>
      <div className="mt-3 flex gap-2">
        {d.opened ? (
          <Button
            size="sm"
            variant="soft"
            onClick={() => s.openProposal('assist-pay')}
          >
            See the vote →
          </Button>
        ) : (
          <>
            <Button size="sm" onClick={s.openAssistPay}>
              Open as a proposal
            </Button>
            <Button
              size="sm"
              variant="ghost"
              onClick={() =>
                s.toast('Amended in place — amount, wording. Still a draft.')
              }
            >
              Amend
            </Button>
          </>
        )}
      </div>
    </Card>
  );
}

function DriDraftCard({ msg }: { msg: Msg }) {
  const s = useStore();
  const d = msg.dri;
  if (!d) return null;
  const id = driProposalId(d.work);
  const list = s.org === 'energy' ? s.eProposals : s.proposals;
  const existing = list.find((p) => p.id === id);
  const held =
    (d.work === 'Weekday hall' && s.weekday === 'held') ||
    (d.work === 'Carbon credits' && s.eCarbon === 'held');
  const opened = !!existing || held;

  return (
    <Card className={cn('ml-11 max-w-md p-4', !opened && 'border-agent/30')}>
      <div className="flex items-center justify-between gap-3">
        <Kicker className={opened ? undefined : 'text-agent'}>
          Proposal draft — DRI
        </Kicker>
        {opened && (
          <Chip>
            {existing?.state === 'passed' || held
              ? 'agreed'
              : existing?.state === 'rejected'
              ? 'declined'
              : 'opened'}
          </Chip>
        )}
      </div>
      <p className="mt-2 text-[15px] font-semibold tracking-[-0.015em]">
        Name {d.who} as DRI — {d.work}
      </p>
      <p className="mt-1.5 text-[13px] leading-relaxed text-sub">
        {d.workKind === 'project' ? 'Project' : 'Ticket'} with no DRI. Naming{' '}
        {d.who} is a Shaper decision.
      </p>
      <div className="mt-3 flex gap-2">
        {opened && existing ? (
          <Button size="sm" variant="soft" onClick={() => s.openProposal(id)}>
            See the vote →
          </Button>
        ) : opened && held ? (
          <Button
            size="sm"
            variant="soft"
            onClick={() =>
              s.openProject(d.work === 'Carbon credits' ? 'carbon' : 'weekday')
            }
          >
            See the project →
          </Button>
        ) : (
          <>
            <Button size="sm" onClick={() => s.openAssistDri(d)}>
              Open as a proposal
            </Button>
            <Button
              size="sm"
              variant="ghost"
              onClick={() =>
                s.toast('Amended in place — who holds it. Still a draft.')
              }
            >
              Amend
            </Button>
          </>
        )}
      </div>
    </Card>
  );
}

function YouDoneDraftCard() {
  const s = useStore();
  const energy = s.org === 'energy';
  const done = energy ? s.eSummary === 'done' : s.setup === 'done';
  return (
    <Card className={cn('ml-11 max-w-md p-4', !done && 'border-agent/30')}>
      <div className="flex items-center justify-between gap-3">
        <Kicker className={done ? undefined : 'text-agent'}>
          Done draft — {energy ? 'Ameland summary' : 'setup ticket'}
        </Kicker>
        {done && <Chip>confirmed</Chip>}
      </div>
      <p className="mt-2 text-[14px] font-medium">
        {energy
          ? 'Write the Ameland pilot summary for new communities'
          : 'Write the Saturday setup so someone else could run it'}
      </p>
      {done ? (
        <p className="mt-2 text-[13px] text-sub">
          Confirmed by you. The receipt stays on the ticket.
        </p>
      ) : (
        <div className="mt-3 flex gap-2">
          <Button size="sm" onClick={s.confirmYouDone}>
            Confirm — done
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => s.toast('Left open. Nothing changed.')}
          >
            Not done yet
          </Button>
        </div>
      )}
    </Card>
  );
}

/** A ticket under Lea's ticket — she holds the parent, so she offers it. */
function SubTicketDraftCard() {
  const s = useStore();
  const st = s.subCovers;
  if (st === 'none') return null;
  const settled = st !== 'drafted';
  return (
    <Card className={cn('ml-11 max-w-md p-4', !settled && 'border-agent/30')}>
      <div className="flex items-center justify-between gap-3">
        <Kicker className={settled ? undefined : 'text-agent'}>
          Ticket draft — under your covers ticket
        </Kicker>
        {st === 'offered' && <Chip>offered to Jun</Chip>}
        {st === 'accepted' && <Chip tone="agent">Jun holds it</Chip>}
        {st === 'done' && <Chip>done</Chip>}
      </div>
      <p className="mt-2 text-[15px] font-semibold tracking-[-0.015em]">
        {SUB_COVERS_TITLE}
      </p>
      <p className="mt-1 text-[12px] text-faint">
        Saturday stall › Find two neighbours who can cover a Saturday › this
      </p>
      <p className="mt-2 text-[13px] leading-relaxed text-sub">
        {st === 'drafted'
          ? 'Jun has the printer — he said so in “Saturday stall”. You hold the covers ticket, so you can offer a piece of it yourself. Due 6 Jun. Nothing exists until Jun says yes.'
          : st === 'offered'
          ? 'Offered. His yes or no, nobody else’s. Yours stays whole above it.'
          : st === 'accepted'
          ? 'Jun is DRI of this piece only. Your covers ticket cannot close until it does — done moves up, never down.'
          : 'Printed, confirmed by Jun, receipt in “Saturday stall”. Your covers ticket can close now.'}
      </p>
      <div className="mt-3 flex flex-wrap gap-2">
        {st === 'drafted' && s.persona === 'lea' && (
          <>
            <Button size="sm" onClick={s.offerSubTicket}>
              Offer to Jun
            </Button>
            <Button
              size="sm"
              variant="ghost"
              onClick={() =>
                s.toast(
                  'Amended in place — title, due, wording. Still a draft.',
                )
              }
            >
              Amend
            </Button>
          </>
        )}
        {st === 'drafted' && s.persona !== 'lea' && (
          <p className="text-[13px] text-sub">
            Only Lea can offer this — she holds the ticket above it.
          </p>
        )}
        {st === 'offered' && (
          <p className="text-[13px] text-sub">Waiting on Jun…</p>
        )}
        {(st === 'accepted' || st === 'done') && (
          <Button
            size="sm"
            variant="soft"
            onClick={() =>
              s.persona === 'lea'
                ? s.openTicket('covers')
                : s.openProject('stall')
            }
          >
            See it under the covers ticket →
          </Button>
        )}
      </div>
    </Card>
  );
}

function PaymentDraftCard() {
  const s = useStore();
  const energy = s.org === 'energy';
  const id = energy ? PAY_ROGERIO_ID : PAY_LEA_ID;
  const list = energy ? s.eProposals : s.proposals;
  const draft = energy ? s.ePayDraft : s.payDraft;
  const a = agreedPay[s.org];
  const unit = unitFor(s.org);
  const opened = list.some((p) => p.id === id);
  if (!draft) return null;
  const money = (n: number) => `${n.toLocaleString()} ${unit}`;
  const differs = draft.amount !== draft.agreed;
  return (
    <Card className={cn('ml-11 max-w-md p-4', !opened && 'border-agent/30')}>
      <div className="flex items-center justify-between">
        <Kicker className="text-agent">Proposal draft — for the Shapers</Kicker>
        {opened && <Chip>opened</Chip>}
      </div>
      <p className="mt-2 text-[15px] font-semibold tracking-[-0.015em]">
        Pay {a.who} for {a.work} — {money(draft.amount)}
      </p>
      <blockquote className="mt-2 rounded-xl bg-wash px-3 py-2 text-[13px] leading-relaxed text-sub">
        “{money(a.amount)} when both are {energy ? 'signed' : 'found'}.{' '}
        {energy ? 'Yes' : 'Deal'}.” — {a.withWhom} to {a.who}, “{a.roomName}”,{' '}
        {a.when}
      </blockquote>
      <p className="mt-2 text-[13px] leading-relaxed text-sub">
        {differs
          ? `${draft.by} named ${money(
              draft.amount,
            )}; the agreed line says ${money(
              draft.agreed,
            )}. Both go on the card so the Shapers see the difference. `
          : ''}
        Evidence: the agreement above, the ticket for {a.work} (done, confirmed
        by {a.who}), and the line in “{a.roomName}” where done was said.
      </p>
      <div className="mt-3 flex gap-2">
        {opened ? (
          <Button size="sm" variant="soft" onClick={() => s.openProposal(id)}>
            See the vote →
          </Button>
        ) : (
          <>
            <Button size="sm" onClick={s.openPayProposal}>
              Open as a proposal
            </Button>
            <Button
              size="sm"
              variant="ghost"
              onClick={() =>
                s.toast('Amended in place — amount, wording. Still a draft.')
              }
            >
              Amend
            </Button>
          </>
        )}
      </div>
    </Card>
  );
}

function TicketDraftCard() {
  const s = useStore();
  const t = s.chatTicket;
  if (!t || t.org !== s.org) return null;
  const energy = t.org === 'energy';
  const project = energy ? 'Iberia pilots' : 'Saturday stall';
  const dri = personaName(t.org, 'sam');
  const settled = t.state !== 'drafted';
  const own = Boolean(t.under) && t.by === s.persona;
  const canCreate = own || s.persona === 'sam';
  return (
    <Card className={cn('ml-11 max-w-md p-4', !settled && 'border-agent/30')}>
      <div className="flex items-center justify-between">
        <Kicker className={settled ? undefined : 'text-agent'}>
          Ticket draft — under {t.under ?? project}
        </Kicker>
        {t.state === 'created' && <Chip>created</Chip>}
        {t.state === 'routed' && <Chip>with {dri}</Chip>}
      </div>
      <p className="mt-2 text-[15px] font-semibold tracking-[-0.015em]">
        {t.title}
      </p>
      <p className="mt-1 text-[13px] leading-relaxed text-sub">
        {own
          ? t.forName
            ? `You hold the ticket above this, so you can create this piece for ${t.forName}. The project DRI does not need to agree.`
            : 'You hold the ticket above this, so you can create this piece yourself. The project DRI does not need to agree.'
          : t.forName
          ? `Meant for ${t.forName}. It starts open; they accept, or it stays a draft. Only the project DRI can make it real.`
          : 'No DRI yet — it starts open, and someone accepts it. Only the project DRI can make it real.'}
      </p>
      <div className="mt-3 flex flex-wrap gap-2">
        {t.state === 'drafted' &&
          (canCreate ? (
            <>
              <Button size="sm" onClick={s.confirmChatTicket}>
                Create it
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={() =>
                  s.toast('Amended in place — title, due. Still a draft.')
                }
              >
                Amend
              </Button>
            </>
          ) : (
            <Button size="sm" onClick={s.routeChatTicket}>
              Send to {dri} — he confirms
            </Button>
          ))}
        {t.state === 'created' && (
          <Button
            size="sm"
            variant="soft"
            onClick={() =>
              t.under
                ? s.openTicket(
                    energy
                      ? s.persona === 'you'
                        ? 'e-summary'
                        : 'e-muni'
                      : s.persona === 'you'
                      ? 'setup'
                      : 'covers',
                  )
                : s.openProject(energy ? 'iberia' : 'stall')
            }
          >
            {t.under ? 'See it under your ticket →' : 'See it on the board →'}
          </Button>
        )}
        {t.state === 'routed' && (
          <p className="text-[13px] text-sub">
            Nothing exists until {dri} says yes — same rule as everything else.
          </p>
        )}
      </div>
    </Card>
  );
}

function DoneDraftCard() {
  const s = useStore();
  if (s.covers === 'accepted') {
    return (
      <Card className="ml-11 max-w-md p-4">
        <Kicker>Done draft — withdrawn</Kicker>
        <p className="mt-1.5 text-[13px] text-sub">
          Reopened. Nothing changed state silently.
        </p>
      </Card>
    );
  }
  const confirmed = s.covers === 'done';
  return (
    <Card className={cn('ml-11 max-w-md p-4', !confirmed && 'border-agent/30')}>
      <div className="flex items-center justify-between">
        <Kicker className={confirmed ? undefined : 'text-agent'}>
          Done draft — covers ticket
        </Kicker>
        {confirmed && <Chip>confirmed</Chip>}
      </div>
      <p className="mt-2 text-[14px] font-medium">
        Find two neighbours who can cover a Saturday
      </p>
      {s.coversQuote && (
        <blockquote className="mt-2 rounded-xl bg-wash px-3 py-2 text-[13px] leading-relaxed text-sub">
          “{s.coversQuote}” — this thread, today
        </blockquote>
      )}
      {confirmed ? (
        <p className="mt-2 text-[13px] text-sub">
          Confirmed by Lea. The receipt stays on the ticket.
        </p>
      ) : (
        <>
          <p className="mt-2 text-[13px] leading-relaxed text-sub">
            Lea confirms — or it confirms itself in 48h if nobody objects.
          </p>
          {s.persona === 'lea' && (
            <div className="mt-3 flex gap-2">
              <Button size="sm" onClick={s.confirmCoversDone}>
                Confirm — done
              </Button>
              <Button size="sm" variant="ghost" onClick={s.reopenCovers}>
                Not done yet
              </Button>
            </div>
          )}
        </>
      )}
    </Card>
  );
}

function EnergyDoneDraftCard() {
  const s = useStore();
  if (s.eMuni === 'doing') {
    return (
      <Card className="ml-11 max-w-md p-4">
        <Kicker>Done draft — withdrawn</Kicker>
        <p className="mt-1.5 text-[13px] text-sub">
          Reopened. Nothing changed state silently.
        </p>
      </Card>
    );
  }
  const confirmed = s.eMuni === 'done';
  return (
    <Card className={cn('ml-11 max-w-md p-4', !confirmed && 'border-agent/30')}>
      <div className="flex items-center justify-between">
        <Kicker className={confirmed ? undefined : 'text-agent'}>
          Done draft — municipalities ticket
        </Kicker>
        {confirmed && <Chip>confirmed</Chip>}
      </div>
      <p className="mt-2 text-[14px] font-medium">
        Onboard two Portuguese municipalities
      </p>
      {s.eMuniQuote && (
        <blockquote className="mt-2 rounded-xl bg-wash px-3 py-2 text-[13px] leading-relaxed text-sub">
          “{s.eMuniQuote}” — this thread, today
        </blockquote>
      )}
      {confirmed ? (
        <p className="mt-2 text-[13px] text-sub">
          Confirmed by Rogerio. The receipt stays on the ticket.
        </p>
      ) : (
        <>
          <p className="mt-2 text-[13px] leading-relaxed text-sub">
            Rogerio confirms — or it confirms itself in 48h if nobody objects.
          </p>
          {s.persona === 'lea' && (
            <div className="mt-3 flex gap-2">
              <Button size="sm" onClick={s.confirmMuniDone}>
                Confirm — done
              </Button>
              <Button size="sm" variant="ghost" onClick={s.reopenMuni}>
                Not done yet
              </Button>
            </div>
          )}
        </>
      )}
    </Card>
  );
}

const DIRECTION_DRAFTS: Record<
  'river' | 'energy',
  Record<DirectionKind, { quote: string; note: string; proposalId?: string }>
> = {
  river: {
    mission: {
      quote:
        'A street food hub from people we know, not a supermarket — for neighbours, and the three growers we already buy from.',
      note: 'Why we exist. From the day the space opened.',
      proposalId: 'dir-mission-v1',
    },
    vision: {
      quote:
        'A neighbourhood that feeds itself two days a week: five growers, a hall paid without a whip-round, every grower paid the week they sell.',
      note: 'Where we are going. Voted the same day as the mission.',
      proposalId: 'dir-vision-v1',
    },
    objectives: {
      quote: 'A weekday hall is open before August.',
      note: 'One new objective. Saturday was holding; the vision asked for two days.',
      proposalId: 'dir-objectives-v3',
    },
    strategy: {
      quote: 'We do not take the brand sponsorship. Not this year.',
      note: 'From Tuesday’s call · consistent with the vote on 2 May.',
      proposalId: 'dir-strategy-v5',
    },
  },
  energy: {
    mission: {
      quote:
        'Energy should create value where it is produced — ownership, income, and control stay local.',
      note: 'Why we exist. Voted after the first pilots.',
      proposalId: 'e-dir-mission-v2',
    },
    vision: {
      quote:
        '10,000 energy hubs by 2030 — members save 20–80% and co-own the assets.',
      note: 'Where we are going. The save range is measured, not promised.',
      proposalId: 'e-dir-vision-v1',
    },
    objectives: {
      quote: 'A second island runs the Ameland model by December.',
      note: 'One new objective. Ameland was done; the vision asked for more hubs.',
      proposalId: 'e-dir-objectives-v6',
    },
    strategy: {
      quote: 'Conferences wait. Six communities producing first.',
      note: 'From Tuesday’s call · consistent with the rejected booth in December.',
      proposalId: 'e-dir-strategy-v4',
    },
  },
};

function DirectionDraftCard({ msg }: { msg: Msg }) {
  const s = useStore();
  const energy = s.org === 'energy';
  const kind = msg.artifact ?? 'strategy';
  const draft = DIRECTION_DRAFTS[energy ? 'energy' : 'river'][kind];
  const live = kind === 'strategy';
  const decided = live
    ? energy
      ? !s.eStrategyPending
      : !s.strategyPending
    : true;
  const agreed = live
    ? energy
      ? s.eStrategyVersion === 4
      : s.strategyVersion === 5
    : true;
  const proposalId = draft.proposalId;
  const list = energy ? s.eProposals : s.proposals;
  const canVote = energy
    ? s.persona === 'maya'
    : s.persona === 'maya' || s.persona === 'sam';
  const canOpen =
    proposalId &&
    (!live || (decided && agreed && list.some((p) => p.id === proposalId)));

  return (
    <Card className={cn('ml-11 max-w-md p-4', !decided && 'border-agent/30')}>
      <div className="flex items-center justify-between gap-3">
        <Kicker className={decided ? undefined : 'text-agent'}>
          Proposal — {kind === 'objectives' ? 'objective' : kind}
        </Kicker>
        {decided && <Chip>{agreed ? 'agreed' : 'declined'}</Chip>}
      </div>
      <p className="mt-2 text-[14px] font-medium leading-relaxed">
        “{draft.quote}”
      </p>
      <p className="mt-1.5 text-[13px] leading-relaxed text-sub">
        {draft.note}
      </p>
      {!decided && canVote && (
        <div className="mt-3 flex gap-2">
          <Button size="sm" onClick={s.confirmStrategy}>
            Agree
          </Button>
          <Button size="sm" variant="ghost" onClick={s.rejectStrategy}>
            Decline
          </Button>
        </div>
      )}
      {!decided && !canVote && (
        <p className="mt-2 text-[12px] text-faint">
          Waiting on a Shaper to agree or decline.
        </p>
      )}
      {canOpen && (
        <div className="mt-3">
          <Button
            size="sm"
            variant="soft"
            onClick={() => s.openProposal(proposalId)}
          >
            See the vote →
          </Button>
        </div>
      )}
    </Card>
  );
}
