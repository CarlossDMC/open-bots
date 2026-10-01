// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Markdown } from "./markdown";

describe("Markdown", () => {
  it("renders emphasis, lists, and code instead of raw markdown syntax", () => {
    const { container } = render(
      <Markdown content={"**Done**\n\n- first\n- second\n\nRun `npm test`."} />
    );

    expect(screen.getByText("Done").tagName).toBe("STRONG");
    expect(screen.getAllByRole("listitem")).toHaveLength(2);
    expect(screen.getByText("npm test").tagName).toBe("CODE");
    expect(container.textContent).not.toContain("**");
  });

  it("does not render raw HTML from message content", () => {
    const { container } = render(<Markdown content={'<img src="x" onerror="alert(1)">'} />);

    expect(container.querySelector("img")).toBeNull();
  });
});
