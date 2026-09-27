/** Read Protocol tag rows without inventing names. */

export function firstTag(
  tags: readonly string[][],
  name: string,
  marker?: string,
): string | null {
  for (const tag of tags) {
    if (tag[0] !== name || !tag[1]) continue;
    if (marker === undefined) {
      if (!tag[3]) return tag[1];
      continue;
    }
    if (tag[3] === marker) return tag[1];
  }
  return null;
}

export function anyTag(tags: readonly string[][], name: string): string | null {
  for (const tag of tags) {
    if (tag[0] === name && tag[1]) return tag[1];
  }
  return null;
}

export function allTags(
  tags: readonly string[][],
  name: string,
  marker?: string,
): string[] {
  const values: string[] = [];
  for (const tag of tags) {
    if (tag[0] !== name || !tag[1]) continue;
    if (marker === undefined) {
      values.push(tag[1]);
      continue;
    }
    if (tag[3] === marker) values.push(tag[1]);
  }
  return values;
}

export function parseJsonObject(
  content: string,
): Record<string, unknown> | null {
  try {
    const value: unknown = JSON.parse(content);
    if (typeof value !== "object" || value === null || Array.isArray(value)) {
      return null;
    }
    return value as Record<string, unknown>;
  } catch {
    return null;
  }
}

export function asString(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 ? value : null;
}

export function asNumber(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}
