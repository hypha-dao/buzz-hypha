// Date rules for the fixtures (Prototype map § Determinism).
//
// `data.ts` writes dates the way people do — "12 May", "Nov 2025", "Q3",
// "Spring", "yesterday". Every one resolves to a Unix timestamp at 09:00Z:
// a *decision* date ("decided", "approved", "confirmedOn") resolves to the
// start of the period it names, a *deadline* ("review", "due", an objective's
// "by …") to its end. Undated rows take the base clock — 2026-03-01T09:00Z
// plus the row's order, one minute apart — nudged forward past whatever
// they causally follow so the event list reads in relay order.

export const BASE = Date.UTC(2026, 2, 1, 9, 0, 0) / 1000;
// The fixture's present. The prototype's live tickets are due 15 Jun–Jul,
// its newest done row is "6 Jun" (a trail row, ignored), the Saturday stall
// comes up for review on 1 Jun — so the seed sits on the last Sunday of May.
export const NOW = Date.UTC(2026, 4, 31, 9, 0, 0) / 1000;
export const DAY = 86_400;
export const MINUTE = 60;

const MONTHS = {
  jan: 0, january: 0, feb: 1, february: 1, mar: 2, march: 2, apr: 3, april: 3,
  may: 4, jun: 5, june: 5, jul: 6, july: 6, aug: 7, august: 7, sep: 8, sept: 8,
  september: 8, oct: 9, october: 9, nov: 10, november: 10, dec: 11, december: 11,
};

function at(year, month, day) {
  return Date.UTC(year, month, day, 9, 0, 0) / 1000;
}

function lastDay(year, month) {
  return new Date(Date.UTC(year, month + 1, 0)).getUTCDate();
}

function period(year, monthFrom, monthTo, mode) {
  return mode === "end" ? at(year, monthTo, lastDay(year, monthTo)) : at(year, monthFrom, 1);
}

// Resolve a loose date. `mode` is "start" (decisions) or "end" (deadlines).
export function resolveDate(text, mode = "start", year = 2026) {
  const s = String(text).trim().toLowerCase().replace(/\.$/, "");
  let m;
  if (s === "today" || s === "now") return NOW;
  if (s === "yesterday") return NOW - DAY;
  if ((m = s.match(/^(\d+)h ago$/))) return NOW - Number(m[1]) * 3600;
  if (s === "last week") return NOW - 7 * DAY;
  if (s === "last month") return NOW - 30 * DAY;
  if ((m = s.match(/^(\d+|two|three|four) (days?|weeks?) ago$/))) {
    const n = { two: 2, three: 3, four: 4 }[m[1]] ?? Number(m[1]);
    return NOW - n * (m[2].startsWith("week") ? 7 : 1) * DAY;
  }
  if (/^(tue|tuesday)$/.test(s)) {
    // The Tuesday before NOW (NOW is a Sunday).
    const nowDate = new Date(NOW * 1000);
    const back = (nowDate.getUTCDay() - 2 + 7) % 7 || 7;
    return NOW - back * DAY;
  }
  if ((m = s.match(/^(\d{1,2}) ([a-z]+)(?: (\d{4}))?$/)) && MONTHS[m[2]] !== undefined) {
    return at(m[3] ? Number(m[3]) : year, MONTHS[m[2]], Number(m[1]));
  }
  if ((m = s.match(/^([a-z]+)(?: (\d{4}))?$/)) && MONTHS[m[1]] !== undefined) {
    const y = m[2] ? Number(m[2]) : year;
    return period(y, MONTHS[m[1]], MONTHS[m[1]], mode);
  }
  if ((m = s.match(/^(\d{4})$/))) return period(Number(m[1]), 0, 11, mode);
  if ((m = s.match(/^q([1-4])(?: (\d{4}))?$/))) {
    const q = Number(m[1]) - 1;
    return period(m[2] ? Number(m[2]) : year, q * 3, q * 3 + 2, mode);
  }
  if (s === "spring") return period(year, 2, 4, mode);
  if (s === "autumn") return period(year, 8, 10, mode);
  throw new Error(`unparseable date ${JSON.stringify(text)}`);
}

// ISO week label for a timestamp, e.g. `2026-W22`.
export function isoWeek(ts) {
  const d = new Date(ts * 1000);
  const day = (d.getUTCDay() + 6) % 7;
  d.setUTCDate(d.getUTCDate() - day + 3);
  const firstThursday = new Date(Date.UTC(d.getUTCFullYear(), 0, 4));
  const week = 1 + Math.round(((d - firstThursday) / DAY / 1000 - 3 + ((firstThursday.getUTCDay() + 6) % 7)) / 7);
  return `${d.getUTCFullYear()}-W${String(week).padStart(2, "0")}`;
}

// A causal clock. `next(anchor, ...after)` returns `max(anchor ?? base +
// order, max(after) + 1 min)` and advances the base clock by one minute per
// call so undated rows keep their `data.ts` order.
export class Clock {
  constructor() {
    this.order = 0;
  }

  next(anchor, ...after) {
    const base = BASE + this.order * MINUTE;
    this.order += 1;
    let ts = anchor ?? base;
    for (const t of after) {
      if (t !== undefined && t !== null && t + MINUTE > ts) ts = t + MINUTE;
    }
    return ts;
  }
}
