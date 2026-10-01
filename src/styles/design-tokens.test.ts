import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import tailwindConfig from "../../tailwind.config";

const tokensCss = readFileSync(path.join(process.cwd(), "src/styles/tokens.css"), "utf8");

function declaredTokens(selector: ":root" | ".dark"): Set<string> {
  const escaped = selector.replace(".", "\\.");
  const block = new RegExp(`(?:^|\\n)${escaped}\\s*\\{([^}]*)\\}`).exec(tokensCss)?.[1] ?? "";
  return new Set([...block.matchAll(/(--[a-z0-9-]+)\s*:/g)].map((match) => match[1]));
}

function referencedTokens(pattern: RegExp): string[] {
  const theme = JSON.stringify(tailwindConfig.theme);
  return [...new Set([...theme.matchAll(pattern)].map((match) => match[1]))];
}

describe("design tokens", () => {
  const light = declaredTokens(":root");
  const dark = declaredTokens(".dark");

  it("declares every token Tailwind references", () => {
    const missing = referencedTokens(/var\((--[a-z0-9-]+)\)/g).filter((token) => !light.has(token));
    expect(missing).toEqual([]);
  });

  it("gives every color token a dark theme value", () => {
    const colors = referencedTokens(/hsl\(var\((--[a-z0-9-]+)\)/g);
    expect(colors.length).toBeGreaterThan(30);
    expect(colors.filter((token) => !dark.has(token))).toEqual([]);
  });

  it("keeps light hover and selected surfaces visibly distinct", () => {
    const lightness = (token: string) => {
      const value = new RegExp(
        `(?:^|\\n):root\\s*\\{[^}]*${token}:\\s*[\\d.]+ [\\d.]+% ([\\d.]+)%`
      ).exec(tokensCss)?.[1];
      return Number(value);
    };
    const pairs = [
      ["--surface", "--muted"],
      ["--card", "--muted"],
      ["--muted", "--accent"]
    ] as const;
    for (const [base, hover] of pairs) {
      expect(lightness(base) - lightness(hover), `${base} vs ${hover}`).toBeGreaterThanOrEqual(3);
    }
  });

  it("does not declare dark-only tokens", () => {
    expect([...dark].filter((token) => !light.has(token))).toEqual([]);
  });
});

const sourceRoot = path.join(process.cwd(), "src");
const palette =
  "slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose|white|black";
const rawPaletteClass = new RegExp(
  `(?<![\\w-])(?:[a-z-]+:)*(?:bg|text|border|ring|from|via|to|fill|stroke|accent|outline|divide|placeholder|decoration|shadow)-(?:${palette})(?:-\\d{2,3})?(?:\\/\\d{1,3})?(?![\\w-])`,
  "g"
);
const arbitraryColor = /(?:bg|text|border|ring|from|via|to)-\[(?:#|rgb|hsl)[^\]]*\]/g;
const arbitraryFontSize = /text-\[\d+px\]/g;

describe("component styling", () => {
  const files = readdirSync(sourceRoot, { recursive: true, encoding: "utf8" }).filter(
    (file) =>
      /\.(?:tsx?|css)$/.test(file) &&
      !/\.test\.tsx?$/.test(file) &&
      file !== path.join("styles", "tokens.css")
  );

  it("uses design tokens instead of raw palette values", () => {
    const violations = files.flatMap((file) => {
      const source = readFileSync(path.join(sourceRoot, file), "utf8");
      return [rawPaletteClass, arbitraryColor, arbitraryFontSize].flatMap((pattern) =>
        [...source.matchAll(pattern)].map((match) => `${file}: ${match[0]}`)
      );
    });
    expect(violations).toEqual([]);
  });
});
