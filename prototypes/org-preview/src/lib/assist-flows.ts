import type { DirectionKind, OrgId, PersonaId } from './data';

export type HelpId =
  | 'direction'
  | 'project'
  | 'money'
  | 'done'
  | 'ticket'
  | 'dri'
  | 'ask';

export const HELP: { id: HelpId; label: string; prompt: string }[] = [
  {
    id: 'direction',
    label:
      'Draft and publish a direction proposal — mission, vision, objectives, or strategy',
    prompt: 'Draft and publish a direction proposal',
  },
  {
    id: 'project',
    label: 'Draft and publish a project proposal',
    prompt: 'Draft and publish a project proposal',
  },
  {
    id: 'money',
    label: 'Draft and publish a money-movement proposal',
    prompt: 'Draft and publish a money-movement proposal',
  },
  {
    id: 'done',
    label: 'Mark your ticket done',
    prompt: 'Mark my ticket done',
  },
  {
    id: 'ticket',
    label: 'Create a ticket — for you, or for someone else',
    prompt: 'Create a ticket',
  },
  {
    id: 'dri',
    label: 'Name a DRI for work that has none — yourself, or someone else',
    prompt: 'Name a DRI for work that has no one',
  },
  {
    id: 'ask',
    label: 'Answer anything — I have an overview of the whole org',
    prompt: 'What should I know about this org right now?',
  },
];

export const DIR_KINDS: DirectionKind[] = [
  'mission',
  'vision',
  'objectives',
  'strategy',
];

export const DIR_COPY: Record<
  OrgId,
  Record<DirectionKind, { quote: string; note: string }>
> = {
  river: {
    mission: {
      quote:
        'A street food hub from people we know, not a supermarket — and not a brand’s community programme.',
      note: 'One clause added to the mission. The rest stands.',
    },
    vision: {
      quote:
        'A neighbourhood that feeds itself two days a week — a Saturday stall and a weekday people can walk to after work.',
      note: 'The vision already said two days. This names the weekday.',
    },
    objectives: {
      quote: 'Price signs on the stall every Saturday by the end of June.',
      note: 'A new objective. Saturday is holding; neighbours still ask what things cost.',
    },
    strategy: {
      quote: 'We do not take the brand sponsorship. Not this year.',
      note: 'From Tuesday’s call · consistent with the vote on 2 May.',
    },
  },
  energy: {
    mission: {
      quote:
        'Energy should create value where it is produced — the credits belong to the members, not to us.',
      note: 'The mission tightened: ownership is named, not implied.',
    },
    vision: {
      quote:
        '10,000 energy hubs by 2030 — and a second island running the Ameland model by December.',
      note: 'The far target stays. One nearer line added from the pilots room.',
    },
    objectives: {
      quote: 'A Spanish-language onboarding guide is live before September.',
      note: 'Pedro asked for it under Iberia. An objective makes the date real.',
    },
    strategy: {
      quote: 'Prove it in real towns first. Conferences wait.',
      note: 'Same bet as the rejected booth — written so the next draft cannot forget it.',
    },
  },
};

export const PROJECT_COPY: Record<
  OrgId,
  { title: string; description: string; ends: string }
> = {
  river: {
    title: 'Weekday hall',
    description:
      'Find a weekday hall and hold it: the licence, the deposit, the keys. The vision asked for two days; Saturday is the only one that exists.',
    ends: '1 Aug 2026',
  },
  energy: {
    title: 'Carbon credits',
    description:
      'Stand up the carbon-credit line so communities can issue, not only consume. Needs someone who knows the accounting.',
    ends: '30 Nov 2026',
  },
};

export const PAY_COPY: Record<
  OrgId,
  { who: string; amount: number; work: string; withWhom: string; room: string }
> = {
  river: {
    who: 'Tom',
    amount: 40,
    work: 'held the cash box last Saturday',
    withWhom: 'Sam',
    room: 'Saturday stall',
  },
  energy: {
    who: 'Suzana',
    amount: 400,
    work: 'playbook chapter 2',
    withWhom: 'Alex',
    room: 'Community onboarding',
  },
};

export const TICKET_FOR_ME: Record<OrgId, string> = {
  river: 'Host next Saturday — open and close',
  energy: 'Write a one-page for a new Spanish community',
};

/** The open ticket this persona holds — a piece under it does not need the project DRI. */
export function ticketYouHold(
  org: OrgId,
  persona: PersonaId,
  s: {
    setup: string;
    covers: string;
    eSummary: string;
    eMuni: string;
  },
): string | null {
  if (org === 'river') {
    if (persona === 'lea' && s.covers !== 'done')
      return 'Find two neighbours who can cover a Saturday';
    if (persona === 'you' && s.setup === 'accepted')
      return 'Write the Saturday setup so someone else could run it';
    return null;
  }
  if (persona === 'lea' && s.eMuni !== 'done')
    return 'Onboard two Portuguese municipalities';
  if (persona === 'you' && s.eSummary === 'accepted')
    return 'Write the Ameland pilot summary for new communities';
  return null;
}

export const TICKET_FOR_OTHER: Record<OrgId, Record<string, string>> = {
  river: {
    Jun: 'Print price signs for the stall',
    Lea: 'Teach the cash-box count to the next host',
    Tom: 'Hold the cash box next Saturday',
    Priya: 'Redraw the stall price board',
  },
  energy: {
    Rowan: 'Write the carbon intake checklist',
    Rogerio: 'Call the third Portuguese municipality',
    Suzana: 'Translate the member FAQ into Spanish',
    Inês: 'Draft the Beja welcome note',
  },
};

export function helpIdFrom(text: string): HelpId | null {
  const t = text.trim();
  const hit = HELP.find((h) => h.prompt === t);
  if (hit) return hit.id;
  if (/^draft and publish a direction/i.test(t)) return 'direction';
  if (/direction proposal|suggest direction/i.test(t)) return 'direction';
  if (/project proposal|suggest a project/i.test(t)) return 'project';
  if (/money-movement|money movement|pay proposal/i.test(t)) return 'money';
  if (isMarkDone(t)) return 'done';
  if (/^create a ticket\b/i.test(t)) return 'ticket';
  if (/name a dri|dri for work|needs a dri|no dri|assign .{0,20}dri/i.test(t))
    return 'dri';
  if (/what should i know|overview of (the |this )?org/i.test(t)) return 'ask';
  return null;
}

export function isMarkDone(text: string): boolean {
  return /\bmark\b.{0,24}\bdone\b|\bticket\b.{0,12}\bdone\b/i.test(text);
}

export function asDirectionKind(text: string): DirectionKind | null {
  const t = text.trim().toLowerCase();
  if (t.startsWith('mission')) return 'mission';
  if (t.startsWith('vision')) return 'vision';
  if (t.startsWith('objective')) return 'objectives';
  if (t.startsWith('strategy')) return 'strategy';
  return null;
}

export type OpenWork = {
  work: string;
  workKind: 'project' | 'ticket';
};

/** Work that is live and still has no DRI — the picker for a naming proposal. */
export function openWorkWithoutDri(
  org: OrgId,
  s: {
    weekday: string;
    setup: string;
    eCarbon: string;
    harvestDri?: string;
  },
): OpenWork[] {
  if (org === 'river') {
    const list: OpenWork[] = [];
    if (s.weekday !== 'held')
      list.push({ work: 'Weekday hall', workKind: 'project' });
    if (s.harvestDri !== 'held')
      list.push({ work: 'Autumn harvest fair', workKind: 'project' });
    if (s.setup !== 'accepted' && s.setup !== 'done')
      list.push({ work: 'Saturday setup', workKind: 'ticket' });
    return list;
  }
  const list: OpenWork[] = [];
  if (s.eCarbon !== 'held')
    list.push({ work: 'Carbon credits', workKind: 'project' });
  list.push({ work: 'Hardware group-buy', workKind: 'project' });
  list.push({
    work: 'Load test with three communities’ data',
    workKind: 'ticket',
  });
  return list;
}

export function driPeople(org: OrgId, you: string): string[] {
  const others =
    org === 'river'
      ? ['Lea', 'Jun', 'Rafi', 'Tom', 'Priya', 'Sam', 'Maya']
      : ['Rowan', 'Rogerio', 'Suzana', 'Pedro', 'Marcus', 'Inês', 'Alex'];
  return [you, ...others.filter((n) => n !== you)];
}

export function driProposalId(work: string): string {
  return (
    'dri-' +
    work
      .toLowerCase()
      .normalize('NFD')
      .replace(/[\u0300-\u036f]/g, '')
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-|-$/g, '')
      .slice(0, 32)
  );
}

export function matchOpenWork(
  text: string,
  works: OpenWork[],
): OpenWork | undefined {
  const t = text.trim().toLowerCase();
  return (
    works.find((w) => w.work.toLowerCase() === t) ??
    works.find((w) => t.includes(w.work.toLowerCase()))
  );
}
