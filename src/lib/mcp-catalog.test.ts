import { describe, expect, it } from "vitest";
import { mergeFound } from "./mcp-catalog";

describe("mergeFound", () => {
  it("keeps found servers and marks catalog entries the provider no longer reports", () => {
    expect(
      mergeFound(
        [
          { name: "claude.ai Atlassian", status: "connected" },
          { name: "claude.ai Notion", status: "needs-authentication" }
        ],
        ["claude.ai Atlassian", "github"]
      )
    ).toEqual([
      { name: "claude.ai Atlassian", status: "connected" },
      { name: "claude.ai Notion", status: "needs-authentication" },
      { name: "github", status: "missing" }
    ]);
  });
});
