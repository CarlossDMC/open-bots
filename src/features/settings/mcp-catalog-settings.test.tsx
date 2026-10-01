// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { McpCatalogSettings } from "./mcp-catalog-settings";
import {
  discoverMcpServers,
  listMcpCatalog,
  onRuntimeEvent,
  saveMcpCatalog
} from "@/lib/desktop-api";
import type { ProviderSummary } from "@/types/domain";

vi.mock("@/lib/desktop-api", () => ({
  discoverMcpServers: vi.fn(),
  listMcpCatalog: vi.fn(),
  onRuntimeEvent: vi.fn(),
  saveMcpCatalog: vi.fn()
}));

const claude: ProviderSummary = {
  id: "claude-code",
  name: "Claude Code",
  kind: "cli",
  status: "available",
  detail: "",
  capabilities: ["configured_mcp_servers"]
};

beforeEach(() => {
  vi.mocked(onRuntimeEvent).mockResolvedValue(() => undefined);
  vi.mocked(listMcpCatalog).mockResolvedValue([]);
  vi.mocked(discoverMcpServers).mockResolvedValue([
    { name: "claude.ai Atlassian", status: "connected" },
    { name: "claude.ai Notion", status: "needs-authentication" }
  ]);
  vi.mocked(saveMcpCatalog).mockImplementation((providerId, servers) =>
    Promise.resolve(servers.map((name) => ({ providerId, name, addedAt: "2026-10-01T12:00:00Z" })))
  );
});
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("McpCatalogSettings", () => {
  it("imports server names and saves only the chosen ones", async () => {
    render(<McpCatalogSettings providers={[claude]} />);
    await screen.findByText("No servers in the catalog");

    fireEvent.click(screen.getByRole("button", { name: /Import from Claude Code/ }));
    const atlassian = await screen.findByRole("checkbox", { name: "claude.ai Atlassian" });
    expect(screen.getByText("Needs authentication in the provider")).toBeTruthy();
    expect(atlassian.getAttribute("aria-checked")).toBe("false");

    fireEvent.click(atlassian);
    fireEvent.click(screen.getByRole("button", { name: "Save catalog" }));

    await waitFor(() =>
      expect(saveMcpCatalog).toHaveBeenCalledWith("claude-code", ["claude.ai Atlassian"])
    );
    await screen.findByText("claude.ai Atlassian");
    expect(screen.queryByRole("button", { name: "Save catalog" })).toBeNull();
  });

  it("explains when no provider can share servers", () => {
    render(<McpCatalogSettings providers={[{ ...claude, capabilities: [] }]} />);
    expect(screen.getByText(/No installed provider can share/)).toBeTruthy();
  });

  it("reports discovery failures", async () => {
    vi.mocked(discoverMcpServers).mockRejectedValue(new Error("claude was not found"));
    render(<McpCatalogSettings providers={[claude]} />);
    fireEvent.click(screen.getByRole("button", { name: /Import from Claude Code/ }));
    expect((await screen.findByRole("alert")).textContent).toContain("claude was not found");
  });
});
