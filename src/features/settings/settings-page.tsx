import { Database, Monitor, Puzzle, SlidersHorizontal } from "lucide-react";
import { StatusBadge } from "@/components/ui/status-badge";
import type { ProviderSummary } from "@/types/domain";

export function SettingsPage({ providers }: { providers: ProviderSummary[] }) {
  return (
    <div className="animate-fade-in">
      <header className="mb-7">
        <h1 className="text-xl font-semibold text-zinc-100">Settings</h1>
        <p className="mt-1 text-sm text-zinc-500">Local application and provider configuration.</p>
      </header>
      <div className="grid gap-4 lg:grid-cols-2">
        <SettingsCard icon={SlidersHorizontal} title="General">
          <Row name="Startup behavior" value="Open last view" />
          <Row name="Runtime mode" value="Local worker" />
        </SettingsCard>
        <SettingsCard icon={Puzzle} title="Providers">
          <div className="space-y-3">
            {providers.map((provider) => (
              <div key={provider.id} className="rounded-md border border-zinc-900 bg-zinc-950 p-3">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-sm text-zinc-200">{provider.name}</p>
                    <p className="mt-0.5 text-[10px] uppercase tracking-wider text-zinc-600">
                      {provider.kind} adapter
                    </p>
                  </div>
                  <StatusBadge
                    status={
                      provider.status === "available"
                        ? "completed"
                        : provider.status === "not-installed"
                          ? "failed"
                          : "idle"
                    }
                  />
                </div>
                <p className="mt-2 text-xs leading-5 text-zinc-500">{provider.detail}</p>
              </div>
            ))}
          </div>
        </SettingsCard>
        <SettingsCard icon={Monitor} title="Appearance">
          <Row name="Theme" value="Dark" />
          <Row name="Density" value="Compact" />
        </SettingsCard>
        <SettingsCard icon={Database} title="Data">
          <Row name="Storage" value="Local SQLite" />
          <Row name="Cloud sync" value="Not implemented" />
        </SettingsCard>
      </div>
    </div>
  );
}

function SettingsCard({
  icon: Icon,
  title,
  children
}: {
  icon: typeof SlidersHorizontal;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section className="panel">
      <h2 className="section-title">
        <Icon size={14} /> {title}
      </h2>
      {children}
    </section>
  );
}
function Row({ name, value }: { name: string; value: string }) {
  return (
    <div className="flex items-center justify-between border-b border-zinc-900 py-2.5 last:border-0">
      <span className="text-xs text-zinc-500">{name}</span>
      <span className="text-xs text-zinc-300">{value}</span>
    </div>
  );
}
