// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ModelPicker } from "./model-picker";
import { resetProviderModelCache } from "@/hooks/use-provider-models";
import { listProviderModels } from "@/lib/desktop-api";
import type { ProviderSummary } from "@/types/domain";

vi.mock("@/lib/desktop-api", () => ({ listProviderModels: vi.fn() }));

const codex: ProviderSummary = {
  id: "codex",
  name: "OpenAI Codex CLI",
  kind: "cli",
  status: "available",
  detail: "",
  capabilities: ["model_selection"]
};

beforeEach(() => {
  resetProviderModelCache();
  vi.mocked(listProviderModels)
    .mockReset()
    .mockResolvedValue([
      {
        id: "gpt-5.5",
        displayName: "GPT-5.5",
        description: "",
        isDefault: true,
        reasoningEfforts: ["low", "high"],
        defaultReasoningEffort: "low"
      }
    ]);
});

afterEach(cleanup);

describe("ModelPicker", () => {
  it("says when a provider cannot select models", () => {
    render(
      <ModelPicker
        provider={{ ...codex, name: "Mock Provider", capabilities: [] }}
        model={null}
        reasoningEffort={null}
        onChange={() => undefined}
      />
    );
    expect(screen.getByText("Model selection is not supported by Mock Provider.")).toBeTruthy();
    expect(listProviderModels).not.toHaveBeenCalled();
  });

  it("offers catalog models and resets the effort when the model changes", async () => {
    const onChange = vi.fn();
    render(
      <ModelPicker provider={codex} model={null} reasoningEffort={null} onChange={onChange} />
    );
    const modelBox = screen.getByRole("combobox", { name: "Model" });
    await waitFor(() => expect(modelBox.hasAttribute("disabled")).toBe(false));
    expect(screen.getByRole("combobox", { name: "Reasoning" }).hasAttribute("disabled")).toBe(true);

    fireEvent.click(modelBox);
    fireEvent.change(screen.getByRole("searchbox"), { target: { value: "5.5" } });
    expect(screen.getAllByRole("option").map((option) => option.textContent)).toEqual([
      "GPT-5.5 (default)"
    ]);
    fireEvent.keyDown(screen.getByRole("searchbox"), { key: "Enter" });
    expect(onChange).toHaveBeenCalledWith({ model: "gpt-5.5", reasoningEffort: null });
  });

  it("lists the efforts of the selected model", async () => {
    const onChange = vi.fn();
    render(
      <ModelPicker provider={codex} model="gpt-5.5" reasoningEffort={null} onChange={onChange} />
    );
    const effortBox = screen.getByRole("combobox", { name: "Reasoning" });
    await waitFor(() => expect(effortBox.textContent).toBe("Model default (low)"));
    fireEvent.click(effortBox);
    fireEvent.click(screen.getByRole("option", { name: "high" }));
    expect(onChange).toHaveBeenCalledWith({ model: "gpt-5.5", reasoningEffort: "high" });
  });

  it("shows catalog errors", async () => {
    vi.mocked(listProviderModels).mockRejectedValue("Codex app-server timed out");
    render(<ModelPicker provider={codex} model={null} reasoningEffort={null} onChange={vi.fn()} />);
    expect((await screen.findByRole("alert")).textContent).toBe("Codex app-server timed out");
  });
});
