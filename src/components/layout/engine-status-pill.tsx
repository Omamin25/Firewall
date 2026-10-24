import { CircleAlert, CircleCheck, ShieldAlert, ShieldCheck, ShieldX } from "lucide-react";
import { cn } from "@/lib/cn";
import type { AppStatus, EnforcementStatus } from "@/types/ipc";

export type EngineStatusPillProps = {
  status: AppStatus | null;
  /** Null until the first poll answers; the pill then reports the enforcement half too. */
  enforcement: EnforcementStatus | null;
};

type Tone = "success" | "warning" | "danger";

// Tinted fill, `-strong` label and icon (`ui_tokens.md` §1.1).
const TONES: Record<Tone, string> = {
  success: "bg-success/10 text-success-strong",
  warning: "bg-warning/10 text-warning-strong",
  danger: "bg-danger/10 text-danger-strong",
};

const ICONS = {
  success: CircleCheck,
  enforcing: ShieldCheck,
  warning: ShieldAlert,
  danger: ShieldX,
} as const;

/** Foundation health plus enforcement state (`ui_rules.md` §1). Status is never encoded by
 *  color alone: every tone carries its own icon and a label. */
export function EngineStatusPill({ status, enforcement }: EngineStatusPillProps): JSX.Element {
  const { tone, label, icon: Icon, detail } = describe(status, enforcement);
  return (
    <span
      title={detail}
      className={cn(
        "inline-flex items-center gap-2 rounded-full px-2 py-1 text-xs font-medium",
        TONES[tone],
      )}
    >
      <Icon aria-hidden size={14} />
      {label}
      {status === null ? <CircleAlert aria-hidden size={14} /> : null}
    </span>
  );
}

function describe(
  status: AppStatus | null,
  enforcement: EnforcementStatus | null,
): { tone: Tone; label: string; icon: (typeof ICONS)[keyof typeof ICONS]; detail: string } {
  if (!status?.isElevated || !status.databaseReady) {
    return {
      tone: "danger",
      label: "Engine unavailable",
      icon: ICONS.danger,
      detail: status?.error ?? "Relaunch Cognton as Administrator to capture and enforce.",
    };
  }
  if (enforcement === null) {
    return {
      tone: "success",
      label: "Foundation ready",
      icon: ICONS.success,
      detail: "Checking the enforcement engine…",
    };
  }
  if (enforcement.error !== null || !enforcement.available) {
    return {
      tone: "warning",
      label: "Capture only",
      icon: ICONS.warning,
      detail:
        enforcement.error ??
        "The WinDivert driver is not available, so rules cannot be enforced. Install WinDivert 2.x and relaunch as Administrator.",
    };
  }
  if (enforcement.enforcing) {
    return {
      tone: "success",
      label: `Enforcing ${enforcement.blockRules} ${enforcement.blockRules === 1 ? "rule" : "rules"}`,
      icon: ICONS.enforcing,
      detail: `${enforcement.blockedPackets} packets blocked so far.`,
    };
  }
  return {
    tone: "success",
    label: "Observing",
    icon: ICONS.success,
    detail: "The enforcement engine is ready. No block rule is enabled, so nothing is dropped.",
  };
}
