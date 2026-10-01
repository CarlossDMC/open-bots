/** A group member that can be mentioned as `@Name`. */
export interface MentionCandidate {
  id: string;
  name: string;
  detail?: string;
}

export interface MentionMatch {
  start: number;
  end: number;
  candidate: MentionCandidate;
}

const maxSuggestions = 8;
/** How far back from the caret an `@` still starts the mention being typed. */
const maxQueryLength = 40;

/**
 * Mentions in `text`, in order. Mirrors `mentioned_members` in
 * `src-tauri/src/domain/groups.rs`: names match ignoring case, the next character must not
 * continue a word, and longer names win, so `@Ana Maria` is not also `@Ana`.
 */
export function findMentions(text: string, candidates: MentionCandidate[]): MentionMatch[] {
  const lower = text.toLowerCase();
  const byLength = [...candidates]
    .filter((candidate) => candidate.name.trim().length > 0)
    .sort((a, b) => b.name.length - a.name.length);
  const matches: MentionMatch[] = [];
  for (const candidate of byLength) {
    const needle = `@${candidate.name.trim().toLowerCase()}`;
    let from = 0;
    for (;;) {
      const start = lower.indexOf(needle, from);
      if (start < 0) break;
      const end = start + needle.length;
      from = end;
      const next = text.charAt(end);
      if (next && /[\p{L}\p{N}_]/u.test(next)) continue;
      if (matches.some((match) => start < match.end && match.start < end)) continue;
      matches.push({ start, end, candidate });
    }
  }
  return matches.sort((a, b) => a.start - b.start);
}

/** Splits text into plain parts and mentions, for rendering. */
export function splitMentions(
  text: string,
  candidates: MentionCandidate[]
): (string | MentionMatch)[] {
  const parts: (string | MentionMatch)[] = [];
  let cursor = 0;
  for (const match of findMentions(text, candidates)) {
    if (match.start > cursor) parts.push(text.slice(cursor, match.start));
    parts.push(match);
    cursor = match.end;
  }
  if (cursor < text.length) parts.push(text.slice(cursor));
  return parts;
}

const mentionHrefPrefix = "#mention-";

/** Turns mentions outside code into markdown links that `mentionIdFromHref` recognizes. */
export function linkMentions(markdown: string, candidates: MentionCandidate[]): string {
  if (candidates.length === 0) return markdown;
  // Code spans and blocks keep their text as written.
  return markdown
    .split(/(```[\s\S]*?```|`[^`\n]*`)/)
    .map((segment, index) =>
      index % 2 === 1
        ? segment
        : splitMentions(segment, candidates)
            .map((part) =>
              typeof part === "string"
                ? part
                : `[${segment.slice(part.start, part.end)}](${mentionHrefPrefix}${encodeURIComponent(part.candidate.id)})`
            )
            .join("")
    )
    .join("");
}

export function mentionIdFromHref(href: string | undefined): string | undefined {
  if (!href?.startsWith(mentionHrefPrefix)) return undefined;
  return decodeURIComponent(href.slice(mentionHrefPrefix.length));
}

/** The mention being typed just before the caret: where its `@` is and what follows it. */
export function mentionQuery(
  text: string,
  caret: number
): { start: number; query: string } | undefined {
  const before = text.slice(Math.max(0, caret - maxQueryLength - 1), caret);
  const at = before.lastIndexOf("@");
  if (at < 0) return undefined;
  const start = caret - before.length + at;
  const query = text.slice(start + 1, caret);
  if (query.includes("\n")) return undefined;
  // An `@` inside a word, as in an email address, does not start a mention.
  const previous = text.charAt(start - 1);
  if (previous && /[\p{L}\p{N}_]/u.test(previous)) return undefined;
  return { start, query };
}

/** Candidates whose name starts with the query, ignoring case. */
export function mentionSuggestions(
  query: string,
  candidates: MentionCandidate[]
): MentionCandidate[] {
  const normalized = query.toLowerCase();
  return candidates
    .filter((candidate) => candidate.name.toLowerCase().startsWith(normalized))
    .slice(0, maxSuggestions);
}

/** Replaces the typed mention with the chosen name and returns the new text and caret. */
export function applyMention(
  text: string,
  caret: number,
  start: number,
  candidate: MentionCandidate
): { text: string; caret: number } {
  const inserted = `@${candidate.name} `;
  const rest = text.slice(caret).replace(/^ /, "");
  return { text: text.slice(0, start) + inserted + rest, caret: start + inserted.length };
}
