import { useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { Pencil, ShieldCheck, Trash2 } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { EmptyState } from "@/components/ui/empty-state";
import { IconButton } from "@/components/ui/icon-button";
import { Switch } from "@/components/ui/switch";
import { TableSkeleton } from "@/components/ui/skeleton";
import { summarizeCriteria } from "@/features/rules/rule-criteria";
import { cn } from "@/lib/cn";
import { formatAbsoluteTime, formatCount } from "@/lib/formatters";
import { useRulesStore } from "@/store/use-rules-store";
import { toastError, toastSuccess } from "@/store/use-toast-store";
import type { Rule } from "@/types/ipc";

const ROW_HEIGHT = 36;
const COLUMNS =
  "grid-cols-[48px_minmax(150px,1.3fr)_76px_84px_minmax(190px,1.6fr)_68px_64px_72px]";

const DIRECTION_LABELS: Record<Rule["direction"], string> = {
  both: "Both",
  out: "Outbound",
  in: "Inbound",
};

export type RulesTableProps = {
  /** Already in evaluation order (see `sortRules`). */
  rules: Rule[];
  loading: boolean;
  onEdit: (rule: Rule) => void;
  onDelete: (rule: Rule) => void;
};

/** Virtualized rules list in the order the engine evaluates them. */
export function RulesTable({ rules, loading, onEdit, onDelete }: RulesTableProps): JSX.Element {
  const scrollRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: rules.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT,
    getItemKey: (index) => rules[index]?.id ?? index,
    overscan: 8,
  });

  if (loading) return <TableSkeleton />;

  if (rules.length === 0) {
    return (
      <EmptyState
        icon={<ShieldCheck aria-hidden size={20} />}
        title="No rules yet"
        description="Cognton observes without blocking until you add a rule. Create one here, or right-click a connection to build one from live traffic."
      />
    );
  }

  return (
    <div className="overflow-hidden rounded-md border border-border">
      <div
        className={cn(
          "grid items-center gap-3 border-b border-border bg-surface py-2 pl-3 pr-table-gutter text-xs font-medium uppercase tracking-wide text-foreground-muted",
          COLUMNS,
        )}
      >
        <span>On</span>
        <span>Name</span>
        <span>Action</span>
        <span>Direction</span>
        <span>Criteria</span>
        <span className="text-right">Priority</span>
        <span className="text-right">Hits</span>
        <span className="text-right">Manage</span>
      </div>
      <div ref={scrollRef} className="max-h-table-body scrollbar-gutter-stable overflow-auto bg-surface">
        <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
          {virtualizer.getVirtualItems().map((row) => {
            const rule = rules[row.index];
            if (!rule) return null;
            return (
              <RuleRow
                key={rule.id}
                rule={rule}
                offset={row.start}
                height={row.size}
                onEdit={onEdit}
                onDelete={onDelete}
              />
            );
          })}
        </div>
      </div>
    </div>
  );
}

type RuleRowProps = {
  rule: Rule;
  offset: number;
  height: number;
  onEdit: (rule: Rule) => void;
  onDelete: (rule: Rule) => void;
};

function RuleRow({ rule, offset, height, onEdit, onDelete }: RuleRowProps): JSX.Element {
  const toggle = useRulesStore((state) => state.toggle);
  const busy = useRulesStore((state) => state.pending.includes(rule.id));
  const criteria = summarizeCriteria(rule);

  return (
    <div
      className={cn(
        "virt-row group absolute left-0 grid w-full items-center gap-3 border-b border-border px-3 text-sm hover:bg-surface-muted",
        COLUMNS,
        !rule.enabled && "text-foreground-muted",
      )}
      style={{ height, top: offset }}
    >
      <Switch
        checked={rule.enabled}
        disabled={busy}
        label={`${rule.enabled ? "Disable" : "Enable"} rule ${rule.name}`}
        onChange={(enabled) => {
          toggle(rule.id, enabled)
            .then(() =>
              toastSuccess(`Rule "${rule.name}" ${enabled ? "enabled" : "disabled"}.`),
            )
            .catch((cause: unknown) =>
              toastError(typeof cause === "string" ? cause : "The rule could not be toggled."),
            );
        }}
      />
      <span className="truncate" title={`Created ${formatAbsoluteTime(rule.createdAt)}`}>
        {rule.name}
      </span>
      {/* Action is the whole point of the row, so it gets the semantic badge. */}
      <Badge
        variant={rule.action === "block" ? "danger" : "success"}
        className="justify-self-start"
      >
        {rule.action === "block" ? "Block" : "Allow"}
      </Badge>
      <span className="truncate text-xs">{DIRECTION_LABELS[rule.direction]}</span>
      <span className="truncate font-mono text-xs text-foreground-muted" title={criteria}>
        {criteria}
      </span>
      <span className="text-right font-mono text-xs">{rule.priority}</span>
      <span
        className="text-right font-mono text-xs"
        title={`Matched ${rule.hitCount} packets since launch`}
      >
        {formatCount(rule.hitCount)}
      </span>
      {/* Revealed on hover and on keyboard focus, so they are reachable either way. */}
      <span className="flex justify-end gap-1 opacity-0 transition-opacity duration-fast group-hover:opacity-100 group-focus-within:opacity-100">
        <IconButton
          icon={<Pencil aria-hidden className="size-3.5" />}
          label={`Edit rule ${rule.name}`}
          disabled={busy}
          onClick={() => onEdit(rule)}
        />
        <IconButton
          variant="destructive"
          icon={<Trash2 aria-hidden className="size-3.5" />}
          label={`Delete rule ${rule.name}`}
          disabled={busy}
          onClick={() => onDelete(rule)}
        />
      </span>
    </div>
  );
}
