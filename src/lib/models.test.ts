import { describe, expect, it } from "vitest";
import { describeModel, supportsModelSelection } from "./models";
import { demoProviders } from "./demo-data";

describe("models", () => {
  it("describes the agent model and reasoning effort", () => {
    expect(describeModel({ model: null })).toBe("Provider default");
    expect(describeModel({ model: "gpt-5.5" })).toBe("gpt-5.5");
    expect(describeModel({ model: "gpt-5.5", reasoningEffort: "high" })).toBe("gpt-5.5 · high");
  });

  it("detects model selection from provider capabilities", () => {
    expect(supportsModelSelection(demoProviders[0])).toBe(false);
    expect(supportsModelSelection({ ...demoProviders[0], capabilities: ["model_selection"] })).toBe(
      true
    );
    expect(supportsModelSelection(undefined)).toBe(false);
  });
});
