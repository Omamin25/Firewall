import { X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import {
  hasActiveFilters,
  type TrafficFilters,
} from "@/features/traffic/traffic-filters";

export type FilterBarProps = {
  filters: TrafficFilters;
  onChange: (patch: Partial<TrafficFilters>) => void;
  onReset: () => void;
  /** Protocol labels present in the current rows, so no option matches nothing. */
  protocols: readonly string[];
  /** Row counts shown beside the controls: `matching of total`. */
  matching: number;
  total: number;
};

const DIRECTIONS = [
  { value: "", label: "Any direction" },
  { value: "out", label: "Outbound" },
  { value: "in", label: "Inbound" },
] as const;

/** Shared filter row for the Connections and Live tables. All six dimensions from
 *  `build_plan.md` Phase 3: process, user, remote, port, protocol, direction. */
export function FilterBar({
  filters,
  onChange,
  onReset,
  protocols,
  matching,
  total,
}: FilterBarProps): JSX.Element {
  const active = hasActiveFilters(filters);
  const protocolOptions = [
    { value: "", label: "Any protocol" },
    ...protocols.map((protocol) => ({ value: protocol, label: protocol })),
  ];

  return (
    <div className="flex flex-wrap items-end gap-2">
      <Field label="Process" className="min-w-36 flex-1">
        <Input
          value={filters.process}
          onChange={(event) => onChange({ process: event.target.value })}
          placeholder="firefox.exe"
        />
      </Field>
      <Field label="User" className="min-w-36 flex-1">
        <Input
          value={filters.user}
          onChange={(event) => onChange({ user: event.target.value })}
          placeholder="domain\user"
        />
      </Field>
      <Field label="Remote IP or domain" className="min-w-40 flex-1">
        <Input
          mono
          value={filters.remote}
          onChange={(event) => onChange({ remote: event.target.value })}
          placeholder="example.com"
        />
      </Field>
      <Field label="Port" className="w-24">
        <Input
          mono
          inputMode="numeric"
          value={filters.port}
          onChange={(event) => onChange({ port: event.target.value })}
          placeholder="443"
        />
      </Field>
      <Field label="Protocol" className="w-40">
        <Select
          value={filters.protocol}
          options={protocolOptions}
          onChange={(event) => onChange({ protocol: event.target.value })}
        />
      </Field>
      <Field label="Direction" className="w-40">
        <Select
          value={filters.direction}
          options={DIRECTIONS}
          onChange={(event) =>
            onChange({ direction: event.target.value as TrafficFilters["direction"] })
          }
        />
      </Field>

      <div className="flex items-center gap-2 pb-0.5">
        <span className="text-xs text-foreground-muted">
          {active ? `${matching.toLocaleString()} of ${total.toLocaleString()}` : `${total.toLocaleString()} rows`}
        </span>
        <Button
          variant="ghost"
          size="sm"
          icon={<X aria-hidden className="size-3.5" />}
          onClick={onReset}
          disabled={!active}
        >
          Clear
        </Button>
      </div>
    </div>
  );
}

type FieldProps = { label: string; className?: string; children: React.ReactNode };

function Field({ label, className, children }: FieldProps): JSX.Element {
  return (
    <label className={className}>
      <span className="mb-1 block text-xs font-medium text-foreground-muted">{label}</span>
      {children}
    </label>
  );
}
