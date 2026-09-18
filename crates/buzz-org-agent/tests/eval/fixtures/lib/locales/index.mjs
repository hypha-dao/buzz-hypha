// Locale tables: `en` source string → translation. `pt` for River and `es`
// for Energy (Prototype map §3 Locales). The generator refuses a table that
// misses or over-covers the strings the `en` build passes through `T`, so
// each table is exactly the translatable surface of its seed.

import { RIVER_PT } from "./river.pt.mjs";
import { ENERGY_ES } from "./energy.es.mjs";

const TABLES = { "river/pt": RIVER_PT, "energy/es": ENERGY_ES };

export function localeFor(orgId, locale) {
  const table = TABLES[`${orgId}/${locale}`];
  if (!table) throw new Error(`no locale table for ${orgId}/${locale}`);
  return table;
}
