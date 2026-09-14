'use client';

import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from 'react';
import {
  agreedPay,
  energyOrg,
  personaName,
  seedProposals,
  space,
  TICKET_SUGGESTED,
  unitFor,
  type DirectionKind,
  type Msg,
  type OrgId,
  type PersonaId,
  type Proposal,
  type ProjectId,
  type TicketId,
  type TicketView,
} from './data';
import {
  DIR_COPY,
  PAY_COPY,
  PROJECT_COPY,
  driProposalId,
} from './assist-flows';
import { ENTRIES, type Entry } from './entries';

export type Route =
  // pre-space
  | 'onboarding'
  | 'public'
  | 'request'
  // the five surfaces
  | 'my'
  | 'all'
  | 'org'
  | 'proposals'
  | 'profile'
  // detail screens
  | 'thread'
  | 'project'
  | 'ticket'
  | 'ticket-view'
  | 'offer'
  | 'proposal'
  | 'direction'
  | 'shaper'
  | 'about';

export type ShaperAskId =
  | 'review'
  | 'weekday'
  | 'harvest'
  | 'cash-teach'
  | 'rota'
  | 'rafi'
  | 'strategy'
  | 'carbon'
  | 'join';

export type YouStage = 'chat' | 'member';
export type SetupState = 'offered' | 'accepted' | 'done' | 'declined';
/** covers: accepted → draftDone (done-from-talk, waiting on Lea) → done */
export type CoversState = 'accepted' | 'draftDone' | 'done';
/**
 * A ticket under Lea's covers ticket — she holds the whole, Jun holds the
 * piece. none → drafted (by the assistant) → offered → accepted → done.
 */
export type SubCoversState =
  | 'none'
  | 'drafted'
  | 'offered'
  | 'accepted'
  | 'done';
export const SUB_COVERS_TITLE = 'Print the Saturday cover rota';
export type WeekdayState =
  | 'draft'
  | 'offering-lea'
  | 'declined-lea'
  | 'offering-rafi'
  | 'held';
export type ReviewState = 'due' | 'extended' | 'closed';

/* ---- Hypha Energy world ---- */
export type SummaryState = 'accepted' | 'done';
/** Rogerio's municipalities ticket: doing → draftDone (done-from-talk) → done */
export type MuniState = 'doing' | 'draftDone' | 'done';
export type CarbonState = 'draft' | 'offering' | 'held';

/** small extra offers to "You", one per org — accepted or sent back */
export type OfferId = 'photo' | 'keys' | 'e-faq' | 'e-charts';
export type OfferState = 'offered' | 'accepted' | 'declined';
export const OFFERS: Record<
  OfferId,
  {
    title: string;
    from: string;
    project: string;
    projectId: ProjectId;
    due: string;
    why: string;
  }
> = {
  photo: {
    title: 'Photograph each grower’s stand for the welcome sheet',
    from: 'Jun',
    project: 'Grower onboarding',
    projectId: 'growers',
    due: '21 Jun',
    why: 'You said you have a decent camera. One Saturday morning.',
  },
  keys: {
    title: 'Note where the keys live and who has a copy',
    from: 'AI',
    project: 'Saturday stall',
    projectId: 'stall',
    due: '12 Jun',
    why: 'The setup ticket needs this piece written down — or the page lives in one head.',
  },
  'e-faq': {
    title: 'Translate the member FAQ into Portuguese',
    from: 'Suzana',
    project: 'Community onboarding playbook',
    projectId: 'playbook',
    due: '31 Jul',
    why: 'You wrote the Ameland summary — same voice, other language.',
  },
  'e-charts': {
    title: 'Pull the three sandbox charts into the summary',
    from: 'AI',
    project: 'Island grids',
    projectId: 'islands',
    due: '12 Jul',
    why: 'The Ameland summary names the charts. No ticket covers them yet.',
  },
};

export type ChatTicket = {
  title: string;
  state: 'drafted' | 'created' | 'routed';
  org: OrgId;
  /** if set, the ticket is meant for this person, not the asker */
  forName?: string;
  /** parent ticket the asker holds — they can create the piece without the project DRI */
  under?: string;
  by?: PersonaId;
};

export type AssistDir = {
  kind: DirectionKind;
  quote: string;
  note: string;
  opened: boolean;
  org: OrgId;
};

export type AssistProject = {
  title: string;
  description: string;
  ends: string;
  opened: boolean;
  org: OrgId;
};

export type AssistPay = {
  who: string;
  amount: number;
  work: string;
  withWhom: string;
  room: string;
  opened: boolean;
  org: OrgId;
};

export type AssistDri = {
  work: string;
  workKind: 'project' | 'ticket';
  who: string;
  opened: boolean;
  org: OrgId;
};

/**
 * A pay proposal the assistant drafted for someone. Anyone can ask for it —
 * the person who did the work, or the person above them. The sum is what
 * they named, or what the room says was agreed.
 */
export type PayDraft = {
  /** display name of whoever asked the assistant */
  by: string;
  amount: number;
  /** the sum found in the agreement line — differs when the asker named another */
  agreed: number;
};

type Profile = { name: string; handle: string; about: string };

type Store = {
  // viewpoint
  persona: PersonaId;
  route: Route;
  threadId: string;
  projectId: ProjectId;
  ticketId: TicketId;
  proposalId: string;
  /** which direction artifact is open on its own page */
  directionKind: DirectionKind;
  // you (newcomer)
  youStage: YouStage;
  profile: Profile;
  intent: 'join' | 'create' | null;
  pinnedJob: string | null;
  // shared world
  setup: SetupState;
  covers: CoversState;
  coversQuote: string | null;
  weekday: WeekdayState;
  rafiJoined: boolean;
  strategyVersion: 4 | 5;
  strategyPending: boolean;
  review: ReviewState;
  proposals: Proposal[];
  myVotes: Record<string, 'yes' | 'no'>;
  payDraft: PayDraft | null;
  extraMsgs: Record<string, Msg[]>;
  notice: string | null;
  // navigation
  go: (route: Route) => void;
  openThread: (id: string) => void;
  openProject: (id: ProjectId) => void;
  openTicket: (id: TicketId) => void;
  /** any ticket on the board, read-only */
  ticketView: TicketView | null;
  viewTicket: (t: TicketView) => void;
  /** who a ticket is currently offered to / held by — overrides the seed */
  ticketPerson: Record<string, string>;
  setTicketPerson: (id: string, name: string) => void;
  openProposal: (id: string) => void;
  /** full text, versions and proofs of one direction artifact */
  openDirection: (kind: DirectionKind) => void;
  /** whose profile is open — null is the current persona’s own */
  viewingProfile: string | null;
  openProfile: (name: string) => void;
  openMyProfile: () => void;
  /** which offer the offer screen is showing */
  offerKey: 'setup' | OfferId;
  openOffer: (id: 'setup' | OfferId) => void;
  /** a Shaper ask opened from My Work — full brief lives here, not on the card */
  shaperAsk: ShaperAskId | null;
  openShaperAsk: (id: ShaperAskId) => void;
  switchPersona: (id: PersonaId) => void;
  // onboarding
  setProfile: (p: Partial<Profile>) => void;
  setIntent: (i: 'join' | 'create') => void;
  pinJob: (id: string) => void;
  joinSpace: () => void;
  requestJoin: () => void;
  skipToOrg: () => void;
  // setup ticket (You)
  acceptSetup: () => void;
  declineSetup: () => void;
  finishSetup: () => void;
  // weekday project (Maya)
  offerWeekday: (to: 'lea' | 'rafi') => void;
  acceptRafi: () => void;
  harvestDri: 'asking' | 'held' | 'declined';
  acceptHarvest: () => void;
  declineHarvest: () => void;
  cashTeach: 'drafted' | 'offered' | 'dismissed';
  offerCashTeach: () => void;
  dismissCashTeach: () => void;
  // strategy — the one direction change in the demo (Maya)
  confirmStrategy: () => void;
  rejectStrategy: () => void;
  eStrategyPending: boolean;
  eStrategyVersion: 3 | 4;
  // done-from-talk (covers)
  triggerDoneDraft: (quote: string, threadId?: string) => void;
  confirmCoversDone: () => void;
  reopenCovers: () => void;
  // extra offers to You (one per org)
  offers: Record<OfferId, OfferState>;
  answerOffer: (id: OfferId, yes: boolean) => void;
  // a ticket under a ticket (Lea splits covers, offers a piece to Jun)
  subCovers: SubCoversState;
  draftSubTicket: (threadId?: string) => void;
  offerSubTicket: () => void;
  dismissSubTicket: () => void;
  // ticket created through the assistant (one draft at a time, per org)
  chatTicket: ChatTicket | null;
  draftChatTicket: (title: string, forName?: string, under?: string) => void;
  confirmChatTicket: () => void;
  routeChatTicket: () => void;
  // assistant help flows — drafts the person can publish
  assistDir: AssistDir | null;
  assistProject: AssistProject | null;
  assistPay: AssistPay | null;
  assistDri: AssistDri | null;
  draftAssistDirection: (kind: DirectionKind) => void;
  openAssistDirection: () => void;
  draftAssistProject: () => void;
  openAssistProject: () => void;
  draftAssistPay: () => void;
  openAssistPay: () => void;
  draftAssistDri: (
    work: string,
    workKind: 'project' | 'ticket',
    who: string,
  ) => void;
  openAssistDri: (draft?: {
    work: string;
    workKind: 'project' | 'ticket';
    who: string;
  }) => void;
  confirmYouDone: () => void;
  // chats the user started, in whichever org was on screen
  customChats: { id: string; title: string; org: OrgId }[];
  createChat: (title: string) => void;
  // which sample org is on screen
  org: OrgId;
  switchOrg: (id: OrgId) => void;
  /** the assistant thread is per org — its messages live under this key */
  agentKey: string;
  // money via proposals — `vote` acts on whichever org is on screen
  /**
   * Draft the pay proposal for the live ticket of whichever org is on screen.
   * `amount` is what the asker named; omitted → whatever the room says was agreed.
   */
  draftPayment: (amount?: number) => void;
  openPayProposal: () => void;
  vote: (id: string, v: 'yes' | 'no') => void;
  // Hypha Energy world
  eSummary: SummaryState;
  eMuni: MuniState;
  eMuniQuote: string | null;
  eCarbon: CarbonState;
  eJoin: boolean;
  ePayDraft: PayDraft | null;
  eProposals: Proposal[];
  eVotes: Record<string, 'yes' | 'no'>;
  finishSummary: () => void;
  triggerMuniDone: (quote: string, threadId?: string) => void;
  confirmMuniDone: () => void;
  reopenMuni: () => void;
  offerCarbon: () => void;
  acceptEnergyJoin: () => void;
  // review (Maya) — projects close on their date; the question is what follows
  reviewExtend: () => void;
  reviewClose: () => void;
  reviewCloseFollowUp: () => void;
  // chat
  sendMsg: (threadId: string, msg: Msg) => void;
  toast: (text: string) => void;
  reset: () => void;
};

const StoreCtx = createContext<Store | null>(null);

const initialProfile: Profile = { name: '', handle: '', about: '' };
const initialOffers: Record<OfferId, OfferState> = {
  photo: 'offered',
  keys: 'offered',
  'e-faq': 'offered',
  'e-charts': 'offered',
};

function uid() {
  return Math.random().toString(36).slice(2, 9);
}

export const PAY_LEA_ID = 'pay-lea';
export const PAY_ROGERIO_ID = 'e-pay-rogerio';

export function StoreProvider({
  children,
  entry = ENTRIES.onboarding,
}: {
  children: ReactNode;
  entry?: Entry;
}) {
  const [persona, setPersona] = useState<PersonaId>(entry.persona);
  const [route, setRoute] = useState<Route>(entry.route);
  const [threadId, setThreadId] = useState('agent');
  const [projectId, setProjectId] = useState<ProjectId>(
    entry.org === 'energy' ? 'iberia' : 'stall',
  );
  const [ticketId, setTicketId] = useState<TicketId>(
    entry.org === 'energy' ? 'e-summary' : 'covers',
  );
  const [proposalId, setProposalId] = useState('');
  const [directionKind, setDirectionKind] = useState<DirectionKind>('mission');
  const [ticketView, setTicketView] = useState<TicketView | null>(null);
  const [ticketPerson, setTicketPersonState] = useState<Record<string, string>>(
    () => ({ ...TICKET_SUGGESTED }),
  );
  const [viewingProfile, setViewingProfile] = useState<string | null>(null);
  const [offerKey, setOfferKey] = useState<'setup' | OfferId>('setup');
  const [shaperAsk, setShaperAsk] = useState<ShaperAskId | null>(null);

  const [youStage, setYouStage] = useState<YouStage>('chat');
  const [profile, setProfileState] = useState<Profile>(initialProfile);
  const [intent, setIntentState] = useState<'join' | 'create' | null>(null);
  const [pinnedJob, setPinnedJob] = useState<string | null>(null);

  const [setup, setSetup] = useState<SetupState>('offered');
  const [covers, setCovers] = useState<CoversState>('accepted');
  const [coversQuote, setCoversQuote] = useState<string | null>(null);
  const [subCovers, setSubCovers] = useState<SubCoversState>('drafted');
  const [harvestDri, setHarvestDri] = useState<'asking' | 'held' | 'declined'>(
    'asking',
  );
  const [cashTeach, setCashTeach] = useState<
    'drafted' | 'offered' | 'dismissed'
  >('drafted');
  const [offers, setOffers] =
    useState<Record<OfferId, OfferState>>(initialOffers);
  const [weekday, setWeekday] = useState<WeekdayState>('draft');
  const [rafiJoined, setRafiJoined] = useState(false);
  const [strategyVersion, setStrategyVersion] = useState<4 | 5>(4);
  const [strategyPending, setStrategyPending] = useState(true);
  const [review, setReview] = useState<ReviewState>('due');
  const [proposals, setProposals] = useState<Proposal[]>(seedProposals);
  const [myVotes, setMyVotes] = useState<Record<string, 'yes' | 'no'>>({});
  const [payDraft, setPayDraft] = useState<PayDraft | null>(null);
  const [chatTicket, setChatTicket] = useState<ChatTicket | null>(null);
  const [assistDir, setAssistDir] = useState<AssistDir | null>(null);
  const [assistProject, setAssistProject] = useState<AssistProject | null>(
    null,
  );
  const [assistPay, setAssistPay] = useState<AssistPay | null>(null);
  const [assistDri, setAssistDri] = useState<AssistDri | null>(null);
  const [customChats, setCustomChats] = useState<
    { id: string; title: string; org: OrgId }[]
  >([]);
  const [org, setOrg] = useState<OrgId>(entry.org);
  const [extraMsgs, setExtraMsgs] = useState<Record<string, Msg[]>>({});

  const [eSummary, setESummary] = useState<SummaryState>('accepted');
  const [eMuni, setEMuni] = useState<MuniState>('doing');
  const [eMuniQuote, setEMuniQuote] = useState<string | null>(null);
  const [eCarbon, setECarbon] = useState<CarbonState>('draft');
  const [eJoin, setEJoin] = useState(false);
  const [eStrategyPending, setEStrategyPending] = useState(true);
  const [eStrategyVersion, setEStrategyVersion] = useState<3 | 4>(3);
  const [ePayDraft, setEPayDraft] = useState<PayDraft | null>(null);
  const [eProposals, setEProposals] = useState<Proposal[]>(energyOrg.proposals);
  const [eVotes, setEVotes] = useState<Record<string, 'yes' | 'no'>>({});
  const [notice, setNotice] = useState<string | null>(null);

  const noticeTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const toast = useCallback((text: string) => {
    setNotice(text);
    if (noticeTimer.current) clearTimeout(noticeTimer.current);
    noticeTimer.current = setTimeout(() => setNotice(null), 3600);
  }, []);

  const sendMsg = useCallback((tid: string, msg: Msg) => {
    setExtraMsgs((m) => ({ ...m, [tid]: [...(m[tid] ?? []), msg] }));
  }, []);

  const value = useMemo<Store>(() => {
    const agentKey = org === 'energy' ? 'e-agent' : 'agent';
    const homeRoute = (p: PersonaId): Route => {
      // onboarding is River Commons' door — in Hypha Energy you are already in
      if (p === 'you' && youStage === 'chat' && org === 'river')
        return 'onboarding';
      if (p === 'eli') return 'org';
      return 'my';
    };

    return {
      persona,
      route,
      threadId,
      projectId,
      ticketId,
      proposalId,
      directionKind,
      youStage,
      profile,
      intent,
      pinnedJob,
      setup,
      covers,
      coversQuote,
      weekday,
      harvestDri,
      cashTeach,
      rafiJoined,
      strategyVersion,
      strategyPending,
      review,
      proposals,
      myVotes,
      payDraft,
      extraMsgs,
      notice,

      viewingProfile,
      offerKey,
      shaperAsk,

      go: setRoute,
      openThread: (id) => {
        setThreadId(id);
        setRoute('thread');
      },
      openProject: (id) => {
        setProjectId(id);
        setRoute('project');
      },
      openTicket: (id) => {
        setTicketId(id);
        setRoute('ticket');
      },
      ticketView,
      ticketPerson,
      viewTicket: (t) => {
        const key = t.ticketKey ?? t.id;
        const who = key && ticketPerson[key] ? ticketPerson[key] : t.who;
        setTicketView(
          key
            ? {
                ...t,
                ticketKey: key,
                suggested: t.suggested ?? TICKET_SUGGESTED[key],
                who,
              }
            : t,
        );
        setRoute('ticket-view');
      },
      setTicketPerson: (id, name) => {
        const stored = name === 'the new member' ? 'You' : name;
        setTicketPersonState((m) => ({ ...m, [id]: stored }));
        setTicketView((t) =>
          t && (t.ticketKey === id || t.id === id) ? { ...t, who: stored } : t,
        );
        const label =
          stored === 'You'
            ? persona === 'you'
              ? 'you'
              : 'the new member'
            : stored;
        toast(`Offered to ${label}. They will see it on My Work.`);
      },
      openProposal: (id) => {
        setProposalId(id);
        setRoute('proposal');
      },
      openDirection: (kind) => {
        setDirectionKind(kind);
        setRoute('direction');
      },
      openProfile: (name) => {
        setViewingProfile(name);
        setRoute('profile');
      },
      openMyProfile: () => {
        setViewingProfile(null);
        setRoute('profile');
      },
      openOffer: (id) => {
        setOfferKey(id);
        setRoute('offer');
      },
      openShaperAsk: (id) => {
        setShaperAsk(id);
        setRoute('shaper');
      },
      switchPersona: (id) => {
        setViewingProfile(null);
        setPersona(id);
        setRoute(homeRoute(id));
      },

      setProfile: (p) => setProfileState((prev) => ({ ...prev, ...p })),
      setIntent: setIntentState,
      pinJob: (id) => {
        setPinnedJob(id);
        setRoute('public');
      },
      joinSpace: () => {
        setYouStage('member');
        setRoute('my');
        toast(
          'You are in. Work is offered, never assigned — you say yes or no.',
        );
      },
      requestJoin: () => setRoute('request'),
      skipToOrg: () => {
        setProfileState({
          name: 'You',
          handle: '',
          about: 'Skipped onboarding for the demo',
        });
        setYouStage('member');
        setRoute('my');
        toast('Skipped — you are inside River Commons as a member.');
      },

      acceptSetup: () => {
        setSetup('accepted');
        setRoute('my');
        toast('You are the DRI of this ticket. It is on My Work now.');
      },
      declineSetup: () => {
        setSetup('declined');
        setRoute('my');
        toast('Sent back to Sam with your note. Nothing held against you.');
      },
      finishSetup: () => {
        setSetup('done');
        setRoute('my');
        toast('Done. Sam sees it on Projects — with your name on it.');
      },

      offerWeekday: (to) => {
        setTicketPersonState((m) => ({
          ...m,
          weekday: to === 'lea' ? 'Lea' : 'Rafi',
        }));
        if (to === 'lea') {
          setWeekday('offering-lea');
          setTimeout(() => {
            setWeekday('declined-lea');
            toast(
              'Lea declined — “I can host a Saturday, I cannot sign a licence.”',
            );
          }, 2200);
        } else {
          setWeekday('offering-rafi');
          setTimeout(() => {
            setWeekday('held');
            setProposals((ps) =>
              ps.some((p) => p.id === 'approve-weekday')
                ? ps
                : [
                    {
                      id: 'approve-weekday',
                      kind: 'project' as const,
                      title: 'Approve project: Weekday hall',
                      sub: 'Rafi as DRI',
                      description:
                        'Find a weekday hall and hold it: the licence, the deposit, the keys.',
                      ends: '1 Aug 2026',
                      state: 'passed' as const,
                      decided: 'today',
                      yes: 2,
                      no: 0,
                      needed: 2,
                      openedBy: 'Maya',
                    },
                    ...ps,
                  ],
            );
            toast(
              'Rafi accepted — he holds Weekday hall. Recorded as a Shaper approval in Decisions.',
            );
          }, 1800);
        }
      },
      acceptHarvest: () => {
        setHarvestDri('held');
        setRoute('my');
        toast('You hold Autumn harvest fair. It is on My Work now.');
      },
      declineHarvest: () => {
        setHarvestDri('declined');
        setRoute('my');
        toast('Sent back. The project stays open until someone sits it.');
      },
      offerCashTeach: () => {
        setCashTeach('offered');
        setTicketPersonState((m) => ({ ...m, 'cash-teach': 'Tom' }));
        setRoute('my');
        toast('Offered to Tom — his yes or no.');
      },
      dismissCashTeach: () => {
        setCashTeach('dismissed');
        setRoute('my');
        toast(
          'Dismissed. The agent will not raise this gap again until something changes.',
        );
      },
      acceptRafi: () => {
        setRafiJoined(true);
        setProposals((ps) =>
          ps.map((p) =>
            p.id === 'join-rafi' && p.state === 'open'
              ? {
                  ...p,
                  state: 'passed' as const,
                  decided: 'today',
                  yes: p.needed,
                  no: 0,
                }
              : p,
          ),
        );
        toast(
          'Rafi is in. He is a member — nothing lands on him until he accepts it.',
        );
      },

      confirmStrategy: () => {
        if (org === 'energy') {
          setEStrategyVersion(4);
          setEStrategyPending(false);
          setEProposals((ps) =>
            ps.some((p) => p.id === 'e-dir-strategy-v4')
              ? ps
              : [
                  {
                    id: 'e-dir-strategy-v4',
                    kind: 'direction' as const,
                    artifact: 'strategy' as const,
                    title: 'Conferences wait. Six communities producing first.',
                    sub: 'From Tuesday’s call · consistent with the rejected booth in December.',
                    state: 'passed' as const,
                    decided: 'today',
                    yes: 3,
                    no: 0,
                    needed: 3,
                    openedBy: 'Alex',
                  },
                  ...ps,
                ],
          );
          toast(
            'Strategy agreed. Recorded in Decisions. Everything the agent does now reads from it.',
          );
          return;
        }
        setStrategyVersion(5);
        setStrategyPending(false);
        setProposals((ps) =>
          ps.some((p) => p.id === 'dir-strategy-v5')
            ? ps
            : [
                {
                  id: 'dir-strategy-v5',
                  kind: 'direction' as const,
                  artifact: 'strategy' as const,
                  title: 'We do not take the brand sponsorship. Not this year.',
                  sub: 'From Tuesday’s call · consistent with the vote on 2 May.',
                  state: 'passed' as const,
                  decided: 'today',
                  yes: 2,
                  no: 0,
                  needed: 2,
                  openedBy: 'Maya',
                },
                ...ps,
              ],
        );
        toast(
          'Strategy agreed. Recorded in Decisions. Everything the agent does now reads from it.',
        );
      },
      rejectStrategy: () => {
        if (org === 'energy') {
          setEStrategyPending(false);
          toast('Rejected. The agent keeps reading v3 — and remembers why.');
          return;
        }
        setStrategyPending(false);
        toast('Rejected. The agent keeps reading v4 — and remembers why.');
      },

      triggerDoneDraft: (quote, threadId = 'saturday') => {
        if (subCovers === 'offered' || subCovers === 'accepted') {
          sendMsg(threadId, {
            id: uid(),
            from: 'agent',
            system: true,
            text: 'Heard — but “Print the Saturday cover rota” is still open under this ticket, with Jun. Done moves up the tree, never down: when his piece closes, I draft this one as done.',
          });
          return;
        }
        setCovers('draftDone');
        setCoversQuote(quote);
        sendMsg(threadId, {
          id: uid(),
          from: 'agent',
          system: true,
          text: 'Heard — drafted the ticket as done, receipt attached. Lea confirms, or it confirms itself in 48h if nobody objects.',
          card: 'done-draft',
        });
      },
      confirmCoversDone: () => {
        if (subCovers === 'offered' || subCovers === 'accepted') {
          toast(
            'Not yet — “Print the Saturday cover rota” is still open under this ticket. Done moves up the tree, never down.',
          );
          return;
        }
        setCovers('done');
        setRoute('my');
        toast(
          'Done, with the receipt on it. Ask your assistant to draft the pay proposal — or Sam will.',
        );
      },
      reopenCovers: () => {
        setCovers('accepted');
        setCoversQuote(null);
        toast('Reopened. The draft is gone — nothing changed state silently.');
      },

      offers,
      answerOffer: (id, yes) => {
        setOffers((o) => ({ ...o, [id]: yes ? 'accepted' : 'declined' }));
        setRoute('my');
        const from = OFFERS[id].from;
        toast(
          yes
            ? `You are the DRI of this piece only. It is on My Work — ${
                from === 'AI' ? 'the agent' : from
              } sees it on Projects.`
            : from === 'AI'
            ? 'Sent back. The agent will not raise this again until something changes.'
            : `Sent back to ${from}. Nothing held against you.`,
        );
      },

      subCovers,
      draftSubTicket: (threadId = agentKey) => {
        if (subCovers !== 'none') return;
        setSubCovers('drafted');
        sendMsg(threadId, {
          id: uid(),
          from: 'agent',
          system: threadId !== agentKey,
          text: 'Drafted a ticket under yours — “Find two neighbours who can cover a Saturday”. You hold that one, so you can offer a piece of it yourself; Sam does not need to see this. Nothing exists until Jun says yes.',
          card: 'sub-ticket-draft',
        });
      },
      dismissSubTicket: () => {
        setSubCovers('none');
        toast(
          'Dismissed. The agent will not raise this gap again until something changes.',
        );
      },
      offerSubTicket: () => {
        setSubCovers('offered');
        setTimeout(() => {
          setSubCovers('accepted');
          toast(
            'Jun accepted — he holds the rota. You still hold the covers; his piece sits under yours.',
          );
        }, 2000);
        setTimeout(() => {
          setSubCovers((v) => (v === 'accepted' ? 'done' : v));
          sendMsg('saturday', {
            id: uid(),
            from: 'Jun',
            text: 'Rota printed — 20 copies, on the table by the cash box.',
          });
          sendMsg('saturday', {
            id: uid(),
            from: 'agent',
            system: true,
            text: 'Marked “Print the Saturday cover rota” done — receipt: Jun’s line above. Jun confirmed. The covers ticket above it can close now.',
          });
          toast(
            'Jun printed the rota — his piece is done. The covers ticket can close now.',
          );
        }, 9000);
      },

      chatTicket,
      draftChatTicket: (title, forName, under) => {
        setChatTicket({
          title,
          state: 'drafted',
          org,
          forName,
          under,
          by: persona,
        });
        const project = org === 'energy' ? 'Iberia pilots' : 'Saturday stall';
        const dri = org === 'energy' ? 'Pedro' : 'Sam';
        sendMsg(agentKey, {
          id: uid(),
          from: 'agent',
          text: under
            ? forName
              ? `Drafted a ticket for ${forName} under yours — “${under}”. You hold that one, so you can create this piece yourself. ${dri} does not need to say yes.`
              : `Drafted a ticket under yours — “${under}”. You hold that one, so you can create this piece yourself. ${dri} does not need to say yes.`
            : forName
            ? `Drafted a ticket for ${forName} — under ${project}. Nothing exists until the project DRI says yes, and ${forName} accepts.`
            : `Drafted a ticket from that — under ${project}, since that is where it belongs. Nothing exists until ${dri}, the project DRI, says yes.`,
          card: 'ticket-draft',
        });
      },
      assistDir,
      assistProject,
      assistPay,
      assistDri,
      draftAssistDirection: (kind) => {
        const copy = DIR_COPY[org][kind];
        setAssistDir({ kind, ...copy, opened: false, org });
        sendMsg(agentKey, {
          id: uid(),
          from: 'agent',
          text: `Proposal draft — ${
            kind === 'objectives' ? 'objective' : kind
          }. Open it and the Shapers vote. Nothing is confirmed until they agree.`,
          card: 'assist-direction',
          artifact: kind,
        });
      },
      openAssistDirection: () => {
        if (!assistDir || assistDir.org !== org) return;
        const id = `assist-dir-${assistDir.kind}`;
        const list = org === 'energy' ? eProposals : proposals;
        const existing = list.find((p) => p.id === id);
        if (!existing) {
          const proposal: Proposal = {
            id,
            kind: 'direction',
            artifact: assistDir.kind,
            title: assistDir.quote,
            sub: assistDir.note,
            state: 'open',
            yes: 0,
            no: 0,
            needed: org === 'energy' ? 3 : 2,
            openedBy: personaName(org, persona),
          };
          (org === 'energy' ? setEProposals : setProposals)((ps) => [
            proposal,
            ...ps,
          ]);
        }
        setAssistDir((d) => (d ? { ...d, opened: true } : d));
        setProposalId(id);
        setRoute('proposal');
        toast('Open. The Shapers decide — everyone can watch.');
      },
      draftAssistProject: () => {
        const copy = PROJECT_COPY[org];
        setAssistProject({ ...copy, opened: false, org });
        sendMsg(agentKey, {
          id: uid(),
          from: 'agent',
          text: `Proposal draft — a project. ${copy.title}. Open it and the Shapers vote. Nothing exists until they agree.`,
          card: 'project-draft',
        });
      },
      openAssistProject: () => {
        if (!assistProject || assistProject.org !== org) return;
        const id = 'assist-project';
        const list = org === 'energy' ? eProposals : proposals;
        if (!list.some((p) => p.id === id)) {
          const proposal: Proposal = {
            id,
            kind: 'project',
            title: `Approve project: ${assistProject.title}`,
            sub: 'No DRI yet — someone accepts after it is live',
            description: assistProject.description,
            ends: assistProject.ends,
            state: 'open',
            yes: 0,
            no: 0,
            needed: org === 'energy' ? 3 : 2,
            openedBy: personaName(org, persona),
          };
          (org === 'energy' ? setEProposals : setProposals)((ps) => [
            proposal,
            ...ps,
          ]);
        }
        setAssistProject((d) => (d ? { ...d, opened: true } : d));
        setProposalId(id);
        setRoute('proposal');
        toast('Open. The Shapers decide — everyone can watch.');
      },
      draftAssistPay: () => {
        const copy = PAY_COPY[org];
        setAssistPay({ ...copy, opened: false, org });
        sendMsg(agentKey, {
          id: uid(),
          from: 'agent',
          text: `Proposal draft — money. ${copy.who} and ${copy.withWhom} agreed this in “${copy.room}”. I never move money — open it and the Shapers decide.`,
          card: 'assist-pay',
        });
      },
      openAssistPay: () => {
        if (!assistPay || assistPay.org !== org) return;
        const id = 'assist-pay';
        const list = org === 'energy' ? eProposals : proposals;
        const unit = unitFor(org);
        if (!list.some((p) => p.id === id)) {
          const proposal: Proposal = {
            id,
            kind: 'money',
            title: `Pay ${
              assistPay.who
            } ${assistPay.amount.toLocaleString()} ${unit} — ${assistPay.work}`,
            sub: `Agreed with ${assistPay.withWhom} in “${assistPay.room}”`,
            amount: assistPay.amount,
            to: assistPay.who,
            state: 'open',
            yes: 0,
            no: 0,
            needed: org === 'energy' ? 3 : 2,
            openedBy: personaName(org, persona),
          };
          (org === 'energy' ? setEProposals : setProposals)((ps) => [
            proposal,
            ...ps,
          ]);
        }
        setAssistPay((d) => (d ? { ...d, opened: true } : d));
        setProposalId(id);
        setRoute('proposal');
        toast(
          org === 'energy'
            ? 'Open. Money moves when all three Shapers agree.'
            : 'Open. Money moves when both Shapers agree.',
        );
      },
      draftAssistDri: (work, workKind, who) => {
        const dest =
          threadId === 'shapers' || threadId === 'e-shapers'
            ? threadId
            : agentKey;
        setAssistDri({ work, workKind, who, opened: false, org });
        sendMsg(dest, {
          id: uid(),
          from: 'agent',
          text: `Proposal draft — name ${who} as DRI of ${work}. Open it and the Shapers vote. Nothing is held until they agree.`,
          card: 'dri-draft',
          dri: { work, workKind, who },
        });
      },
      openAssistDri: (draft) => {
        const d =
          draft ?? (assistDri && assistDri.org === org ? assistDri : null);
        if (!d) return;
        const id = driProposalId(d.work);
        const list = org === 'energy' ? eProposals : proposals;
        if (!list.some((p) => p.id === id)) {
          const proposal: Proposal = {
            id,
            kind: 'dri',
            title: `Name ${d.who} as DRI — ${d.work}`,
            sub: `${d.who} as DRI`,
            description: `${d.who} becomes the DRI of this ${d.workKind}. It has no one now — naming them is a Shaper decision.`,
            to: d.who,
            state: 'open',
            yes: 0,
            no: 0,
            needed: org === 'energy' ? 3 : 2,
            openedBy: personaName(org, persona),
          };
          (org === 'energy' ? setEProposals : setProposals)((ps) => [
            proposal,
            ...ps,
          ]);
        }
        setAssistDri((prev) =>
          prev && prev.org === org && prev.work === d.work
            ? { ...prev, opened: true }
            : { ...d, opened: true, org },
        );
        setProposalId(id);
        setRoute('proposal');
        toast('Open. The Shapers decide — everyone can watch.');
      },
      confirmYouDone: () => {
        if (org === 'energy') {
          setESummary('done');
          toast('Done. Marcus sees it on Projects — with your name on it.');
        } else {
          setSetup('done');
          toast('Done. Sam sees it on Projects — with your name on it.');
        }
      },
      confirmChatTicket: () => {
        setChatTicket((t) => (t ? { ...t, state: 'created' } : t));
        const t = chatTicket;
        toast(
          t?.under
            ? t.forName
              ? `Created under your ticket. ${t.forName} can accept it.`
              : 'Created under your ticket. You hold it — the project DRI did not need to agree.'
            : org === 'energy'
            ? 'Ticket created under Iberia pilots — open, no DRI yet. It is on Projects.'
            : 'Ticket created under Saturday stall — open, no DRI yet. It is on Projects.',
        );
      },
      routeChatTicket: () => {
        setChatTicket((t) => (t ? { ...t, state: 'routed' } : t));
        toast(
          org === 'energy'
            ? 'Sent to Pedro — Iberia is his. It exists when he confirms.'
            : 'Sent to Sam — the stall is his. It exists when he confirms.',
        );
      },

      org,
      agentKey,
      switchOrg: (id) => {
        if (id === org) return;
        setOrg(id);
        setRoute('org');
        setThreadId('agent');
        setProjectId(id === 'energy' ? 'iberia' : 'stall');
        setTicketId(id === 'energy' ? 'e-summary' : 'covers');
        toast(
          id === 'energy'
            ? 'Hypha Energy — same you, a different org. Your work here is on My Work.'
            : 'Back to River Commons.',
        );
      },

      customChats,
      createChat: (title) => {
        const id = `c-${uid()}`;
        setCustomChats((cs) => [...cs, { id, title, org }]);
        setThreadId(id);
        setRoute('thread');
        toast('Room open. Everything said here feeds the org, like any room.');
      },

      draftPayment: (amount) => {
        const energy = org === 'energy';
        const a = agreedPay[org];
        const unit = unitFor(org);
        const by = personaName(org, persona);
        const sum = amount ?? a.amount;
        const draft: PayDraft = { by, amount: sum, agreed: a.amount };
        (energy ? setEPayDraft : setPayDraft)(draft);
        const money = (n: number) => `${n.toLocaleString()} ${unit}`;
        const pair =
          by === a.who
            ? `You and ${a.withWhom}`
            : by === a.withWhom
            ? `You and ${a.who}`
            : `${a.who} and ${a.withWhom}`;
        const found = `${pair} agreed ${money(a.amount)} for ${a.work} in “${
          a.roomName
        }” on ${a.when} — I have the line.`;
        const differs =
          sum !== a.amount
            ? ` You named ${money(
                sum,
              )}; the draft carries that, with the agreed line beside it so the Shapers see both.`
            : '';
        sendMsg(agentKey, {
          id: uid(),
          from: 'agent',
          text: `Drafted. ${found} The ticket is done and confirmed, receipt attached.${differs} I never move money — open it as a proposal and ${
            energy ? 'the three Shapers' : 'the Shapers'
          } decide.`,
          card: 'payment-draft',
        });
      },
      openPayProposal: () => {
        const energy = org === 'energy';
        const draft = energy ? ePayDraft : payDraft;
        if (!draft) return;
        const a = agreedPay[org];
        const unit = unitFor(org);
        const id = energy ? PAY_ROGERIO_ID : PAY_LEA_ID;
        const money = (n: number) => `${n.toLocaleString()} ${unit}`;
        const proposal: Proposal = {
          id,
          kind: 'money',
          title: `Pay ${a.who} ${money(draft.amount)} for ${a.work}`,
          sub:
            `Agreed between ${a.who} and ${a.withWhom} in “${a.roomName}”, ${a.when}` +
            (draft.amount !== draft.agreed
              ? ` — the line says ${money(draft.agreed)}`
              : ''),
          amount: draft.amount,
          to: a.who,
          state: 'open',
          yes: 0,
          no: 0,
          needed: energy ? 3 : 2,
          openedBy: draft.by,
        };
        (energy ? setEProposals : setProposals)((ps) =>
          ps.some((p) => p.id === id) ? ps : [proposal, ...ps],
        );
        setProposalId(id);
        setRoute('proposal');
        toast(
          energy
            ? 'Open. Money moves when all three Shapers agree — everyone can watch.'
            : 'Open. Money moves when both Shapers agree — everyone can watch.',
        );
      },
      vote: (id, v) => {
        const isEnergy = org === 'energy';
        const setVotes = isEnergy ? setEVotes : setMyVotes;
        const setList = isEnergy ? setEProposals : setProposals;
        const voter = personaName(org, persona);
        const bench = isEnergy ? energyOrg.space.shapers : space.shapers;
        setVotes((m) => ({ ...m, [id]: v }));
        setList((ps) =>
          ps.map((p) => {
            if (p.id !== id) return p;
            const agreed = p.agreedBy ?? bench.slice(0, p.yes);
            const rejected = p.rejectedBy ?? bench.slice(p.yes, p.yes + p.no);
            return {
              ...p,
              yes: p.yes + (v === 'yes' ? 1 : 0),
              no: p.no + (v === 'no' ? 1 : 0),
              agreedBy: v === 'yes' ? [...new Set([...agreed, voter])] : agreed,
              rejectedBy:
                v === 'no' ? [...new Set([...rejected, voter])] : rejected,
            };
          }),
        );
        // the other Shaper(s) answer shortly after — the rest of the quorum
        setTimeout(() => {
          setList((ps) =>
            ps.map((p) => {
              if (p.id !== id || p.state !== 'open') return p;
              const rest = p.needed - p.yes - p.no;
              if (v === 'yes') {
                if (p.id === 'join-rafi') setRafiJoined(true);
                if (p.id === 'join-ameland') setEJoin(true);
                if (p.id === 'assist-dir-strategy' && !isEnergy) {
                  setStrategyVersion(5);
                  setStrategyPending(false);
                }
                if (p.id === 'dri-weekday-hall') {
                  const who = p.to ?? 'Rafi';
                  if (who === 'Rafi') setRafiJoined(true);
                  setTicketPersonState((m) => ({ ...m, weekday: who }));
                  setWeekday('held');
                }
                if (p.id === 'dri-carbon-credits') {
                  const who = p.to ?? 'Rowan';
                  setTicketPersonState((m) => ({ ...m, carbon: who }));
                  setECarbon('held');
                }
                if (p.id === 'dri-saturday-setup' && p.to === 'You') {
                  setSetup('accepted');
                }
                toast(
                  p.id.startsWith('dri-')
                    ? `${isEnergy ? 'All three' : 'Both'} Shapers agreed. ${
                        p.to ?? 'They'
                      } hold it now.`
                    : p.kind === 'money'
                    ? `${isEnergy ? 'All three' : 'Both'} Shapers agreed. ${(
                        p.amount ?? 0
                      ).toLocaleString()} ${unitFor(
                        org,
                      )} moves from the treasury.`
                    : p.kind === 'direction'
                    ? `${
                        isEnergy ? 'All three' : 'Both'
                      } Shapers agreed. The new version is live.`
                    : p.kind === 'join'
                    ? `${
                        isEnergy ? 'All three' : 'Both'
                      } Shapers agreed. They are in — nothing lands on them until they accept work.`
                    : `${
                        isEnergy ? 'All three' : 'Both'
                      } Shapers agreed. The project is live.`,
                );
                return {
                  ...p,
                  yes: p.yes + rest,
                  agreedBy: bench.slice(0, p.needed),
                  state: 'passed',
                  decided: 'today',
                };
              }
              toast('Rejected — recorded, with the reason. Nothing moved.');
              const already = new Set([
                ...(p.agreedBy ?? []),
                ...(p.rejectedBy ?? []),
              ]);
              return {
                ...p,
                no: p.no + rest,
                rejectedBy: [
                  ...(p.rejectedBy ?? []),
                  ...bench.filter((n) => !already.has(n)),
                ],
                state: 'rejected',
                decided: 'today',
              };
            }),
          );
        }, 1600);
      },

      reviewExtend: () => {
        setReview('extended');
        toast(
          'Kept open until 1 Sep — against the recommendation. Remembered for next time.',
        );
      },
      reviewClose: () => {
        setReview('closed');
        toast(
          'Stall closes 1 Jun, nothing follows. The project stays readable forever.',
        );
      },
      reviewCloseFollowUp: () => {
        setReview('closed');
        setProposals((ps) =>
          ps.some((p) => p.id === 'approve-stall-summer')
            ? ps
            : [
                {
                  id: 'approve-stall-summer',
                  kind: 'project' as const,
                  title: 'Approve project: Saturday stall — summer season',
                  sub: 'Sam as DRI · follow-up to Saturday stall',
                  description:
                    'Keep the Saturday stall running through the summer: the growers, the tables, the cash box, and the setup doc so anyone can open. Follows the spring project, closed 1 Jun.',
                  ends: '31 Oct 2026',
                  state: 'passed' as const,
                  decided: 'today',
                  yes: 2,
                  no: 0,
                  needed: 2,
                  openedBy: 'Maya',
                },
                ...ps,
              ],
        );
        toast(
          'Stall closes 1 Jun. Summer season is live — offered to Sam, recorded in Decisions.',
        );
      },

      /* ---- Hypha Energy ---- */
      eSummary,
      eMuni,
      eMuniQuote,
      eCarbon,
      eJoin,
      eStrategyPending,
      eStrategyVersion,
      ePayDraft,
      eProposals,
      eVotes,
      finishSummary: () => {
        setESummary('done');
        setRoute('my');
        toast('Done. Marcus sees it on Projects — with your name on it.');
      },
      triggerMuniDone: (quote, threadId = 'e-pilots') => {
        setEMuni('draftDone');
        setEMuniQuote(quote);
        sendMsg(threadId, {
          id: uid(),
          from: 'agent',
          system: true,
          text: 'Heard — drafted the municipalities ticket as done, receipt attached. Rogerio confirms, or it confirms itself in 48h if nobody objects.',
          card: 'e-done-draft',
        });
      },
      confirmMuniDone: () => {
        setEMuni('done');
        setRoute('my');
        toast(
          'Done, with the receipt on it. Ask your assistant to draft the pay proposal — or Pedro will.',
        );
      },
      reopenMuni: () => {
        setEMuni('doing');
        setEMuniQuote(null);
        toast('Reopened. The draft is gone — nothing changed state silently.');
      },
      offerCarbon: () => {
        setECarbon('offering');
        setTimeout(() => {
          setECarbon('held');
          setEProposals((ps) =>
            ps.some((p) => p.id === 'e-approve-carbon')
              ? ps
              : [
                  {
                    id: 'e-approve-carbon',
                    kind: 'project' as const,
                    title: 'Approve project: Carbon credits module',
                    sub: 'Rowan as DRI',
                    description:
                      'Measure the CO₂ each community avoids, sell the reductions, and fund new setups from the savings they create.',
                    ends: '31 Mar 2028',
                    state: 'passed' as const,
                    decided: 'today',
                    yes: 3,
                    no: 0,
                    needed: 3,
                    openedBy: 'Alex',
                  },
                  ...ps,
                ],
          );
          toast(
            'Rowan accepted — he holds Carbon credits. Recorded as a Shaper approval in Decisions.',
          );
        }, 2000);
      },
      acceptEnergyJoin: () => {
        setEJoin(true);
        setEProposals((ps) =>
          ps.map((p) =>
            p.id === 'join-ameland' && p.state === 'open'
              ? {
                  ...p,
                  state: 'passed' as const,
                  decided: 'today',
                  yes: p.needed,
                  no: 0,
                }
              : p,
          ),
        );
        toast(
          'Ameland Energy Coop is in — a member community. Nothing lands on them until they accept it.',
        );
      },

      sendMsg,
      toast,
      reset: () => {
        setPersona(entry.persona);
        setRoute(entry.route);
        setThreadId('agent');
        setProjectId(entry.org === 'energy' ? 'iberia' : 'stall');
        setTicketId(entry.org === 'energy' ? 'e-summary' : 'covers');
        setProposalId('');
        setTicketView(null);
        setTicketPersonState({ ...TICKET_SUGGESTED });
        setViewingProfile(null);
        setOfferKey('setup');
        setShaperAsk(null);
        setYouStage('chat');
        setProfileState(initialProfile);
        setIntentState(null);
        setPinnedJob(null);
        setSetup('offered');
        setCovers('accepted');
        setCoversQuote(null);
        setSubCovers('drafted');
        setHarvestDri('asking');
        setCashTeach('drafted');
        setOffers(initialOffers);
        setWeekday('draft');
        setRafiJoined(false);
        setStrategyVersion(4);
        setStrategyPending(true);
        setReview('due');
        setProposals(seedProposals);
        setMyVotes({});
        setPayDraft(null);
        setChatTicket(null);
        setAssistDir(null);
        setAssistProject(null);
        setAssistPay(null);
        setAssistDri(null);
        setCustomChats([]);
        setOrg(entry.org);
        setExtraMsgs({});
        setNotice(null);
        setESummary('accepted');
        setEMuni('doing');
        setEMuniQuote(null);
        setECarbon('draft');
        setEJoin(false);
        setEStrategyPending(true);
        setEStrategyVersion(3);
        setEPayDraft(null);
        setEProposals(energyOrg.proposals);
        setEVotes({});
      },
    };
  }, [
    eSummary,
    eMuni,
    eMuniQuote,
    eCarbon,
    eJoin,
    eStrategyPending,
    eStrategyVersion,
    ePayDraft,
    eProposals,
    eVotes,
    persona,
    route,
    threadId,
    projectId,
    ticketId,
    ticketView,
    ticketPerson,
    viewingProfile,
    offerKey,
    shaperAsk,
    proposalId,
    youStage,
    profile,
    intent,
    pinnedJob,
    setup,
    covers,
    coversQuote,
    subCovers,
    offers,
    weekday,
    harvestDri,
    cashTeach,
    rafiJoined,
    strategyVersion,
    strategyPending,
    review,
    proposals,
    myVotes,
    payDraft,
    chatTicket,
    assistDir,
    assistProject,
    assistPay,
    assistDri,
    customChats,
    org,
    extraMsgs,
    notice,
    toast,
    sendMsg,
    entry,
  ]);

  return <StoreCtx.Provider value={value}>{children}</StoreCtx.Provider>;
}

export function useStore() {
  const ctx = useContext(StoreCtx);
  if (!ctx) throw new Error('useStore outside provider');
  return ctx;
}
