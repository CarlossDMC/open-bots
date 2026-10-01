import { ShieldCheck } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { ApprovalRequest } from "@/types/domain";

export function ApprovalsPage({
  approvals,
  onResolve
}: {
  approvals: ApprovalRequest[];
  onResolve: (id: string, decision: "approved" | "denied") => void;
}) {
  const pending = approvals.filter((approval) => approval.status === "pending");
  return (
    <div className="animate-fade-in">
      <header className="mb-7">
        <h1 className="text-xl font-semibold text-foreground">Approvals</h1>
        <p className="mt-1 text-sm text-foreground-subtle">
          Human decisions required by runtime policy.
        </p>
      </header>
      {pending.length === 0 ? (
        <div className="grid min-h-64 place-items-center rounded-lg border border-dashed border-border text-center">
          <div>
            <ShieldCheck className="mx-auto mb-3 text-foreground-faint" size={25} />
            <p className="text-sm text-foreground-secondary">No pending approvals</p>
            <p className="mt-1 text-xs text-foreground-faint">
              Policy-gated actions will appear here.
            </p>
          </div>
        </div>
      ) : (
        <div className="space-y-3">
          {pending.map((approval) => (
            <article key={approval.id} className="panel max-w-2xl">
              <p className="text-sm text-foreground-muted">
                <span className="font-medium text-foreground">{approval.agentName}</span> wants to
                execute:
              </p>
              <pre className="my-4 overflow-x-auto rounded-md border border-border bg-muted p-3 font-mono text-xs text-foreground">
                {approval.action}
              </pre>
              <p className="label">Reason</p>
              <p className="mt-1 text-sm text-foreground-muted">{approval.reason}</p>
              <div className="mt-5 flex gap-2">
                <Button size="sm" onClick={() => onResolve(approval.id, "approved")}>
                  Approve
                </Button>
                <Button size="sm" variant="danger" onClick={() => onResolve(approval.id, "denied")}>
                  Deny
                </Button>
              </div>
              <p className="mt-3 text-2xs text-warning">
                Demonstration only — decisions are not persisted yet.
              </p>
            </article>
          ))}
        </div>
      )}
    </div>
  );
}
