import type { ElementType, HTMLAttributes } from "react";
import ReactMarkdown, { type Components, type ExtraProps } from "react-markdown";
import remarkGfm from "remark-gfm";
import { cn } from "@/lib/utils";

// Builds a markdown element renderer with compact chat styling. The hast `node`
// prop that react-markdown passes is dropped so it never reaches the DOM.
function styled(Tag: ElementType, baseClassName: string, extra?: object) {
  return function MarkdownElement({
    node,
    className,
    ...props
  }: HTMLAttributes<HTMLElement> & ExtraProps) {
    void node;
    return <Tag className={cn(baseClassName, className)} {...extra} {...props} />;
  };
}

const block = "my-2 first:mt-0 last:mb-0";

const components: Components = {
  p: styled("p", block),
  h1: styled("h1", "mb-2 mt-4 text-base font-semibold text-foreground first:mt-0"),
  h2: styled("h2", "mb-2 mt-4 text-sm font-semibold text-foreground first:mt-0"),
  h3: styled("h3", "mb-1.5 mt-3 text-sm font-semibold text-foreground first:mt-0"),
  h4: styled("h4", "mb-1.5 mt-3 text-sm font-medium text-foreground first:mt-0"),
  strong: styled("strong", "font-semibold text-foreground"),
  a: styled("a", "text-foreground underline underline-offset-2 hover:text-foreground-secondary", {
    target: "_blank",
    rel: "noreferrer"
  }),
  ul: styled("ul", cn(block, "list-disc space-y-1 pl-5")),
  ol: styled("ol", cn(block, "list-decimal space-y-1 pl-5")),
  li: styled("li", "pl-0.5"),
  blockquote: styled(
    "blockquote",
    cn(block, "border-l-2 border-border pl-3 text-foreground-muted")
  ),
  hr: styled("hr", "my-3 border-border"),
  pre: styled(
    "pre",
    cn(
      block,
      "overflow-x-auto rounded-md border border-border bg-muted px-3 py-2 text-xs leading-5 [&>code]:bg-transparent [&>code]:p-0"
    )
  ),
  code: styled("code", "rounded bg-muted px-1 py-0.5 font-mono text-[0.85em]"),
  table: styled("table", cn(block, "block w-full overflow-x-auto border-collapse text-xs")),
  th: styled("th", "border border-border px-2 py-1 text-left font-medium text-foreground"),
  td: styled("td", "border border-border px-2 py-1")
};

export function Markdown({ content, className }: { content: string; className?: string }) {
  return (
    <div className={cn("min-w-0 break-words", className)}>
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
        {content}
      </ReactMarkdown>
    </div>
  );
}
