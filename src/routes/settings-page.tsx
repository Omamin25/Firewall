import { useEffect, useState } from "react";
import {
  Activity,
  Clock,
  Database,
  Folder,
  HardDrive,
  Lock,
  Save,
  ShieldCheck,
  Trash2,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ErrorState } from "@/components/ui/error-state";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import {
  getFlowHistorySummary,
  getSystemHealth,
  pruneFlowHistory,
  updateRetentionPolicy,
} from "@/lib/ipc";
import { toastError, toastSuccess } from "@/store/use-toast-store";
import type { FlowHistorySummary, SystemHealth } from "@/types/ipc";

export function SettingsPage(): JSX.Element {
  const [summary, setSummary] = useState<FlowHistorySummary | null>(null);
  const [health, setHealth] = useState<SystemHealth | null>(null);
  const [hours, setHours] = useState<number>(24);
  const [maxRows, setMaxRows] = useState<number>(100_000);
  const [saving, setSaving] = useState(false);
  const [pruning, setPruning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = (): void => {
    setError(null);
    Promise.all([getFlowHistorySummary(), getSystemHealth()])
      .then(([historySummary, systemHealth]) => {
        setSummary(historySummary);
        setHealth(systemHealth);
        setHours(historySummary.retentionHours);
        setMaxRows(historySummary.maxRows);
      })
      .catch((cause: unknown) => {
        setError(typeof cause === "string" ? cause : "Unable to load settings.");
      });
  };

  useEffect(() => {
    load();
  }, []);

  const handleSave = (): void => {
    if (hours < 1 || hours > 7200) {
      toastError("Retention window must be between 1 and 7200 hours.");
      return;
    }
    if (maxRows < 100 || maxRows > 1_000_000) {
      toastError("Max rows cap must be between 100 and 1,000,000.");
      return;
    }

    setSaving(true);
    updateRetentionPolicy(hours, maxRows)
      .then((updated) => {
        setSummary(updated);
        toastSuccess("Retention policy updated.");
      })
      .catch((cause: unknown) => {
        toastError(typeof cause === "string" ? cause : "Could not update settings.");
      })
      .finally(() => setSaving(false));
  };

  const handlePrune = (): void => {
    setPruning(true);
    pruneFlowHistory()
      .then((result) => {
        setSummary(result.summary);
        toastSuccess(`Pruned ${result.removedCount} expired history rows.`);
      })
      .catch((cause: unknown) => {
        toastError(typeof cause === "string" ? cause : "Could not prune history.");
      })
      .finally(() => setPruning(false));
  };

  return (
    <div className="space-y-6">
      {error ? <ErrorState title="Settings unavailable" detail={error} /> : null}

      <Card title="Flow history & retention">
        <div className="space-y-5">
          <p className="text-xs text-foreground-muted">
            Configure rolling window retention and database capacity limits for connection history.
            Per-packet payloads are never stored.
          </p>

          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <div className="space-y-1.5">
              <label htmlFor="retention-hours" className="text-xs font-medium text-foreground">
                Retention window (hours)
              </label>
              <div className="relative flex items-center">
                <Clock
                  className="pointer-events-none absolute left-3 size-4 text-foreground-muted"
                  aria-hidden
                />
                <Input
                  id="retention-hours"
                  type="number"
                  min={1}
                  max={7200}
                  mono
                  value={hours}
                  aria-describedby="retention-hours-note"
                  onChange={(event) => setHours(Number(event.target.value))}
                  className="pl-9"
                />
              </div>
              <p id="retention-hours-note" className="text-xs text-foreground-subtle">
                Default: 24h. Maximum: 7200h (300 days).
              </p>
            </div>

            <div className="space-y-1.5">
              <label htmlFor="max-rows" className="text-xs font-medium text-foreground">
                Maximum history rows
              </label>
              <div className="relative flex items-center">
                <Database
                  className="pointer-events-none absolute left-3 size-4 text-foreground-muted"
                  aria-hidden
                />
                <Input
                  id="max-rows"
                  type="number"
                  min={100}
                  max={1_000_000}
                  step={1000}
                  mono
                  value={maxRows}
                  aria-describedby="max-rows-note"
                  onChange={(event) => setMaxRows(Number(event.target.value))}
                  className="pl-9"
                />
              </div>
              <p id="max-rows-note" className="text-xs text-foreground-subtle">
                Default: 100,000 rows limit.
              </p>
            </div>
          </div>

          {summary ? (
            <div className="grid grid-cols-1 gap-3 rounded-lg border border-border bg-surface-muted p-4 sm:grid-cols-3">
              <div>
                <div className="text-xs text-foreground-muted">Stored Connections</div>
                <div className="mt-1 font-mono text-h1 font-semibold text-foreground">
                  {summary.storedFlows.toLocaleString()}
                </div>
              </div>
              <div>
                <div className="text-xs text-foreground-muted">Oldest Recorded</div>
                <div className="mt-1 text-xs font-mono text-foreground">
                  {summary.oldestSeen ? new Date(summary.oldestSeen).toLocaleString() : "None"}
                </div>
              </div>
              <div>
                <div className="text-xs text-foreground-muted">Newest Recorded</div>
                <div className="mt-1 text-xs font-mono text-foreground">
                  {summary.newestSeen ? new Date(summary.newestSeen).toLocaleString() : "None"}
                </div>
              </div>
            </div>
          ) : null}

          <div className="flex flex-wrap items-center justify-between gap-3 border-t border-border pt-4">
            <Button
              variant="outline"
              size="sm"
              icon={<Trash2 aria-hidden className="size-3.5" />}
              disabled={pruning}
              onClick={handlePrune}
            >
              {pruning ? "Pruning..." : "Prune expired history now"}
            </Button>
            <Button
              variant="accent"
              size="sm"
              icon={<Save aria-hidden className="size-3.5" />}
              disabled={saving}
              onClick={handleSave}
            >
              {saving ? "Saving..." : "Save retention settings"}
            </Button>
          </div>
        </div>
      </Card>

      <Card title="System health & diagnostics">
        {health ? (
          <div className="space-y-4">
            <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
              <div className="rounded-md border border-border p-3">
                <div className="flex items-center justify-between">
                  <span className="text-xs text-foreground-muted">Privilege</span>
                  <Lock className="size-4 text-foreground-muted" aria-hidden />
                </div>
                <div className="mt-2 flex items-center gap-2">
                  <Badge variant={health.isElevated ? "success" : "danger"}>
                    {health.isElevated ? "Administrator" : "Non-elevated"}
                  </Badge>
                </div>
              </div>

              <div className="rounded-md border border-border p-3">
                <div className="flex items-center justify-between">
                  <span className="text-xs text-foreground-muted">Database</span>
                  <HardDrive className="size-4 text-foreground-muted" aria-hidden />
                </div>
                <div className="mt-2 flex items-center gap-2">
                  <Badge variant={health.databaseReady ? "success" : "danger"}>
                    {health.databaseReady ? "Online" : "Fallback"}
                  </Badge>
                </div>
              </div>

              <div className="rounded-md border border-border p-3">
                <div className="flex items-center justify-between">
                  <span className="text-xs text-foreground-muted">Npcap Capture</span>
                  <Activity className="size-4 text-foreground-muted" aria-hidden />
                </div>
                <div className="mt-2 flex items-center gap-2">
                  <Badge variant={health.npcapAvailable ? "success" : "warning"}>
                    {health.npcapAvailable ? "Ready" : "Not detected"}
                  </Badge>
                </div>
              </div>

              <div className="rounded-md border border-border p-3">
                <div className="flex items-center justify-between">
                  <span className="text-xs text-foreground-muted">WinDivert Driver</span>
                  <ShieldCheck className="size-4 text-foreground-muted" aria-hidden />
                </div>
                <div className="mt-2 flex items-center gap-2">
                  <Badge
                    variant={
                      health.enforcementStatus.enforcing
                        ? "success"
                        : health.enforcementStatus.available
                          ? "neutral"
                          : "warning"
                    }
                  >
                    {health.enforcementStatus.enforcing
                      ? "Enforcing"
                      : health.enforcementStatus.available
                        ? "Observing"
                        : "Capture only"}
                  </Badge>
                </div>
              </div>
            </div>

            <div className="grid grid-cols-2 gap-3 rounded-lg border border-border bg-surface-muted p-4 sm:grid-cols-4">
              <div>
                <div className="text-xs text-foreground-muted">Active Interfaces</div>
                <div className="mt-1 font-mono text-h2 font-semibold text-foreground">
                  {health.activeInterfacesCount}
                </div>
              </div>
              <div>
                <div className="text-xs text-foreground-muted">Live Flows</div>
                <div className="mt-1 font-mono text-h2 font-semibold text-foreground">
                  {health.activeFlowsCount.toLocaleString()}
                </div>
              </div>
              <div>
                <div className="text-xs text-foreground-muted">Active Rules</div>
                <div className="mt-1 font-mono text-h2 font-semibold text-foreground">
                  {health.activeRulesCount}
                </div>
              </div>
              <div>
                <div className="text-xs text-foreground-muted">Blocked Packets</div>
                <div className="mt-1 font-mono text-h2 font-semibold text-foreground">
                  {health.enforcementStatus.blockedPackets.toLocaleString()}
                </div>
              </div>
            </div>

            <div className="space-y-2 border-t border-border pt-3">
              <div className="flex items-center justify-between text-xs">
                <span className="flex items-center gap-1.5 font-medium text-foreground">
                  <Folder className="size-3.5 text-foreground-muted" aria-hidden />
                  Log Directory
                </span>
                <span className="font-mono text-foreground-muted">{health.logDirectory}</span>
              </div>
              <div className="flex items-center justify-between text-xs">
                <span className="flex items-center gap-1.5 font-medium text-foreground">
                  <Database className="size-3.5 text-foreground-muted" aria-hidden />
                  Database Location
                </span>
                <span className="font-mono text-foreground-muted">{health.databasePath}</span>
              </div>
            </div>
          </div>
        ) : (
          <Skeleton rows={4} />
        )}
      </Card>
    </div>
  );
}
