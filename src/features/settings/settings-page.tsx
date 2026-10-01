import { Database, Download, Monitor, Puzzle, SlidersHorizontal } from "lucide-react";
import { StatusBadge } from "@/components/ui/status-badge";
import { ThemeSelector } from "@/features/appearance/theme-selector";
import { UpdateSettings } from "@/features/updates/update-settings";
import type { AppUpdater } from "@/hooks/use-app-updater";
import type { ProviderSummary } from "@/types/domain";

export function SettingsPage({
  providers,
  updater
}: {
  providers: ProviderSummary[];
  updater: AppUpdater;
}) {
  return (
    <div className="animate-fade-in">
      <header className="mb-7">
        <h1 className="text-xl font-semibold text-foreground">Settings</h1>
        <p className="mt-1 text-sm text-foreground-subtle">
          Local application and provider configuration.
        </p>
      </header>
      <div className="grid gap-4 lg:grid-cols-2">
        <SettingsCard icon={SlidersHorizontal} title="General">
          <Row name="Startup behavior" value="Open last view" />
          <Row name="Runtime mode" value="Local worker" />
        </SettingsCard>
        <SettingsCard icon={Puzzle} title="Providers">
          <div className="space-y-3">
            {providers.map((provider) => (
              <div key={provider.id} className="rounded-md border border-border-subtle bg-card p-3">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-sm text-foreground">{provider.name}</p>
                    <p className="mt-0.5 text-2xs uppercase tracking-wider text-foreground-faint">
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
                <p className="mt-2 text-xs leading-5 text-foreground-subtle">{provider.detail}</p>
              </div>
            ))}
          </div>
        </SettingsCard>
        <SettingsCard icon={Monitor} title="Appearance">
          <Row name="Theme" value={<ThemeSelector />} />
          <Row name="Density" value="Compact" />
        </SettingsCard>
        <SettingsCard icon={Database} title="Data">
          <Row name="Storage" value="Local SQLite" />
          <Row name="Cloud sync" value="Not implemented" />
        </SettingsCard>
        <SettingsCard icon={Download} title="Updates">
          <UpdateSettings updater={updater} />
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
function Row({ name, value }: { name: string; value: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between border-b border-border-subtle py-2.5 last:border-0">
      <span className="text-xs text-foreground-subtle">{name}</span>
      <div className="text-xs text-foreground-secondary">{value}</div>
    </div>
  );
}
