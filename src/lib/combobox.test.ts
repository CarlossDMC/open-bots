import { describe, expect, it } from "vitest";
import { filterOptions } from "./combobox";

const options = [
  { value: "gpt-5.5", label: "GPT-5.5", description: "Legacy coding model." },
  { value: "gpt-6-sol", label: "GPT-6 Sol", description: "Frontier reasoning." },
  { value: "daily", label: "Diário" }
];

describe("filterOptions", () => {
  it("returns every option for an empty query", () => {
    expect(filterOptions(options, "  ")).toBe(options);
  });

  it("matches label, value, and description case-insensitively", () => {
    expect(filterOptions(options, "sol").map((option) => option.value)).toEqual(["gpt-6-sol"]);
    expect(filterOptions(options, "GPT-5").map((option) => option.value)).toEqual(["gpt-5.5"]);
    expect(filterOptions(options, "legacy").map((option) => option.value)).toEqual(["gpt-5.5"]);
  });

  it("requires every word and ignores accents", () => {
    expect(filterOptions(options, "gpt reasoning").map((option) => option.value)).toEqual([
      "gpt-6-sol"
    ]);
    expect(filterOptions(options, "diario").map((option) => option.value)).toEqual(["daily"]);
    expect(filterOptions(options, "missing")).toEqual([]);
  });
});
