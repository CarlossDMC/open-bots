import { describe, expect, it } from "vitest";
import {
  applyMention,
  findMentions,
  linkMentions,
  mentionIdFromHref,
  mentionQuery,
  mentionSuggestions,
  type MentionCandidate
} from "./mentions";

const ana: MentionCandidate = { id: "ana", name: "Ana" };
const anaMaria: MentionCandidate = { id: "ana-maria", name: "Ana Maria" };
const bob: MentionCandidate = { id: "bob", name: "Bob" };
const members = [ana, anaMaria, bob];

describe("findMentions", () => {
  it("matches names ignoring case and prefers longer names", () => {
    expect(
      findMentions("@ana maria and @BOB, then @Ana", members).map((match) => match.candidate.id)
    ).toEqual(["ana-maria", "bob", "ana"]);
  });

  it("ignores names that continue into a longer word", () => {
    expect(findMentions("@Bobby is not a member", members)).toEqual([]);
  });
});

describe("linkMentions", () => {
  it("links mentions outside code and keeps the typed spelling", () => {
    const linked = linkMentions("Ask @bob, not `@Bob`.", members);
    expect(linked).toBe("Ask [@bob](#mention-bob), not `@Bob`.");
    expect(mentionIdFromHref("#mention-bob")).toBe("bob");
    expect(mentionIdFromHref("https://example.com")).toBeUndefined();
  });
});

describe("mentionQuery", () => {
  it("finds the mention being typed before the caret", () => {
    expect(mentionQuery("Hi @An", 6)).toEqual({ start: 3, query: "An" });
    expect(mentionQuery("@", 1)).toEqual({ start: 0, query: "" });
  });

  it("ignores email addresses and finished lines", () => {
    expect(mentionQuery("mail bob@x", 10)).toBeUndefined();
    expect(mentionQuery("@Ana\nnext", 9)).toBeUndefined();
    expect(mentionQuery("no mention", 10)).toBeUndefined();
  });
});

describe("mentionSuggestions and applyMention", () => {
  it("suggests names by prefix and inserts the chosen one", () => {
    expect(mentionSuggestions("an", members).map((candidate) => candidate.id)).toEqual([
      "ana",
      "ana-maria"
    ]);
    expect(applyMention("Hi @an please", 6, 3, anaMaria)).toEqual({
      text: "Hi @Ana Maria please",
      caret: 14
    });
  });
});
