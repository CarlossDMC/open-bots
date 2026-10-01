import type { AgentPermissions } from "@/types/domain";

/** Mirrors `AgentPermissions::workspace_access` in `src-tauri/src/domain/agents.rs`. */
export function canWriteWorkspace(permissions: AgentPermissions): boolean {
  return permissions.filesystem === "workspace-only" && permissions.shell === "allowed";
}

/** Mirrors `AgentPermissions::network_access` in `src-tauri/src/domain/agents.rs`. */
export function canUseInternet(permissions: AgentPermissions): boolean {
  return permissions.network === "allowed";
}
