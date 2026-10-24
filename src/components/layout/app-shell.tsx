import type { ReactNode } from "react";
import { Activity, Database, LayoutDashboard, Network, Settings, ShieldCheck } from "lucide-react";
import { NavLink } from "react-router-dom";
import { EngineStatusPill } from "@/components/layout/engine-status-pill";
import { ThemeToggle } from "@/components/layout/theme-toggle";
import { cn } from "@/lib/cn";
import type { AppStatus, EnforcementStatus } from "@/types/ipc";

const NAV_ITEMS = [
  { to: "/", label: "Overview", icon: LayoutDashboard },
  { to: "/live", label: "Live traffic", icon: Activity },
  { to: "/connections", label: "Connections", icon: Network },
  { to: "/rules", label: "Rules", icon: ShieldCheck },
  { to: "/settings", label: "Settings", icon: Settings },
];
export type AppShellProps = { children: ReactNode; title: string; dark: boolean; onToggleTheme: () => void; status: AppStatus | null; enforcement: EnforcementStatus | null };
export function AppShell({ children, title, dark, onToggleTheme, status, enforcement }: AppShellProps): JSX.Element {
  return <div className="flex min-h-screen bg-background"><aside className="flex w-56 shrink-0 flex-col border-r border-border bg-surface p-3"><div className="flex items-center gap-2 px-2 py-3 text-h2 font-semibold tracking-tight"><ShieldCheck aria-hidden className="text-primary-strong" size={20} />Cognton</div><nav className="mt-4 space-y-1">{NAV_ITEMS.map(({ to, label, icon: Icon }) => <NavLink key={to} to={to} className={({ isActive }) => cn("flex h-9 items-center gap-3 rounded-md px-3 text-sm text-foreground-muted hover:bg-surface-muted hover:text-foreground", isActive && "bg-primary/10 text-primary-strong")}><Icon aria-hidden size={16} />{label}</NavLink>)}</nav><div className="mt-auto flex items-center gap-2 border-t border-border px-2 py-3 text-xs text-foreground-muted"><Database aria-hidden size={14} />Local-only</div></aside><div className="min-w-0 flex-1"><header className="flex h-14 items-center justify-between border-b border-border bg-surface px-6"><h1 className="text-h1 font-semibold tracking-tight">{title}</h1><div className="flex items-center gap-3"><EngineStatusPill status={status} enforcement={enforcement} /><ThemeToggle dark={dark} onToggle={onToggleTheme} /></div></header><main className="px-6 py-5">{children}</main></div></div>;
}
