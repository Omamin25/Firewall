import { Eye, ShieldCheck, ShieldOff, ShieldX } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { formatCount } from "@/lib/formatters";
import type { EnforcementStatus } from "@/types/ipc";

export type EnforcementSummaryProps = { enforcement: EnforcementStatus | null };

/** What the engine is actually doing right now, beside the rules that drive it. The
 *  Connections view shows the effect; this shows the cause. */
export function EnforcementSummary({ enforcement }: EnforcementSummaryProps): JSX.Element {
  if (!enforcement) {
    return <p className="text-xs text-foreground-muted">Reading enforcement state…</p>;
  }

  return (
    <dl className="flex flex-wrap items-center gap-x-6 gap-y-1 text-xs text-foreground-muted">
      <State enforcement={enforcement} />
      <Stat label="Enabled" value={formatCount(enforcement.enabledRules)} />
      <Stat label="Blocking" value={formatCount(enforcement.blockRules)} />
      <Stat label="Packets dropped" value={formatCount(enforcement.blockedPackets)} />
      {enforcement.observing ? (
        <Stat label="Socket events" value={formatCount(enforcement.socketEvents)} />
      ) : null}
    </dl>
  );
}

/** Tone is always paired with an icon and a word (`ui_rules.md` §10). */
function State({ enforcement }: { enforcement: EnforcementStatus }): JSX.Element {
  if (enforcement.error) {
    return (
      <Badge variant="danger" title={enforcement.error}>
        <ShieldX aria-hidden size={12} />
        Enforcement unavailable
      </Badge>
    );
  }
  if (enforcement.enforcing) {
    return (
      <Badge variant="success">
        <ShieldCheck aria-hidden size={12} />
        Enforcing
      </Badge>
    );
  }
  if (enforcement.observing) {
    return (
      <Badge variant="neutral">
        <Eye aria-hidden size={12} />
        Observing only
      </Badge>
    );
  }
  return (
    <Badge variant="warning">
      <ShieldOff aria-hidden size={12} />
      Idle
    </Badge>
  );
}

type StatProps = { label: string; value: string };

function Stat({ label, value }: StatProps): JSX.Element {
  return (
    <div className="flex items-baseline gap-1.5">
      <dt>{label}</dt>
      <dd className="font-mono text-foreground">{value}</dd>
    </div>
  );
}
