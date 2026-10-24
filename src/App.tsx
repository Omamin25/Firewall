import { useEffect, useState } from "react";
import { Route, Routes, useLocation } from "react-router-dom";
import { AppShell } from "@/components/layout/app-shell";
import { ErrorState } from "@/components/ui/error-state";
import { Toaster } from "@/components/ui/toast";
import { getAppStatus, runDatabaseSmokeTest } from "@/lib/ipc";
import { ConnectionsPage } from "@/routes/connections-page";
import { OverviewPage } from "@/routes/overview-page";
import { LivePage } from "@/routes/live-page";
import { RulesPage } from "@/routes/rules-page";
import { SettingsPage } from "@/routes/settings-page";
import { useRulesStore } from "@/store/use-rules-store";
import type { AppStatus } from "@/types/ipc";

const TITLES: Record<string, string> = {
  "/": "Overview",
  "/live": "Live traffic",
  "/connections": "Connections",
  "/rules": "Rules",
  "/settings": "Settings",
};

/** How often the enforcement half of the status pill is refreshed. Rule changes reconcile
 *  the engine synchronously, so this only has to keep counters and driver health current. */
const ENFORCEMENT_REFRESH_MS = 5_000;

export default function App(): JSX.Element {
  const [status, setStatus] = useState<AppStatus | null>(null);
  const enforcement = useRulesStore((state) => state.enforcement);
  const refreshEnforcement = useRulesStore((state) => state.refreshEnforcement);
  const [dark, setDark] = useState(false);
  const [smokeResult, setSmokeResult] = useState<string | null>(null);
  const location = useLocation();

  useEffect(() => {
    document.documentElement.classList.toggle("dark", dark);
  }, [dark]);

  useEffect(() => {
    getAppStatus()
      .then(setStatus)
      .catch((error: unknown) =>
        setStatus({
          isElevated: false,
          databaseReady: false,
          error:
            error instanceof Error ? error.message : "Unable to contact the application core.",
        }),
      );
  }, []);

  useEffect(() => {
    void refreshEnforcement();
    const timer = window.setInterval(() => void refreshEnforcement(), ENFORCEMENT_REFRESH_MS);
    return () => window.clearInterval(timer);
  }, [refreshEnforcement]);

  return (
    <AppShell
      title={TITLES[location.pathname] ?? "Cognton"}
      dark={dark}
      onToggleTheme={() => setDark((value) => !value)}
      status={status}
      enforcement={enforcement}
    >
      {status?.error ? (
        <ErrorState
          title="Startup needs attention"
          detail={status.error}
          action={
            <button
              type="button"
              className="rounded-md bg-secondary px-3 py-2 text-sm font-medium text-secondary-foreground"
              onClick={() => {
                void runDatabaseSmokeTest().then(setSmokeResult);
              }}
            >
              Test local database
            </button>
          }
        />
      ) : null}
      {smokeResult ? (
        <p className="mb-4 text-sm text-success-strong">Local database write confirmed: {smokeResult}</p>
      ) : null}
      <Routes>
        <Route path="/" element={<OverviewPage />} />
        <Route path="/live" element={<LivePage />} />
        <Route path="/connections" element={<ConnectionsPage />} />
        <Route path="/rules" element={<RulesPage />} />
        <Route path="/settings" element={<SettingsPage />} />
      </Routes>
      <Toaster />
    </AppShell>
  );
}
