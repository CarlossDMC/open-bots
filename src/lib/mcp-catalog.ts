import type { ConfiguredMcpServer, McpServerStatus } from "@/types/domain";

/** A server shown while reviewing an import; `missing` is in the catalog but not reported. */
export interface ReviewServer {
  name: string;
  status: McpServerStatus | "missing";
}

/** Found servers first, then catalog entries the provider no longer reports. */
export function mergeFound(found: ConfiguredMcpServer[], current: string[]): ReviewServer[] {
  const missing = current
    .filter((name) => !found.some((server) => server.name === name))
    .map((name) => ({ name, status: "missing" as const }));
  return [...found, ...missing];
}
