import { useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { MoreHorizontal, Network, ShieldX } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { EmptyState } from "@/components/ui/empty-state";
import { IconButton } from "@/components/ui/icon-button";
import { TableSkeleton } from "@/components/ui/skeleton";
import { DirectionIcon } from "@/components/data/direction-icon";
import { ProtocolBadge } from "@/components/data/protocol-badge";
import { cn } from "@/lib/cn";
import {
  formatAbsoluteTime,
  formatAddressPort,
  formatBytes,
  formatDuration,
  formatEndpoint,
} from "@/lib/formatters";
import type { Flow } from "@/types/ipc";

const ROW_HEIGHT = 36;
const COLUMNS =
  "grid-cols-[76px_68px_minmax(180px,1.5fr)_92px_minmax(130px,1fr)_minmax(120px,1fr)_minmax(96px,0.8fr)_84px_84px_36px]";
const UNKNOWN = "—";

/** Where a row menu should open, in viewport coordinates. */
export type MenuAnchor = { x: number; y: number };

export type ConnectionsTableProps = {
  /** Already filtered and sorted by the page (see `use-filtered-flows.ts`). */
  flows: Flow[];
  loading: boolean;
  /** True when filters are hiding rows, so the empty state can say so. */
  filtered: boolean;
  /** Opens the row's action menu — hover button or right-click (`ui_rules.md` §5). */
  onRowMenu: (flow: Flow, anchor: MenuAnchor) => void;
};

/** Virtualized table of live connections with process and user attribution. */
export function ConnectionsTable({
  flows,
  loading,
  filtered,
  onRowMenu,
}: ConnectionsTableProps): JSX.Element {
  const scrollRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: flows.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT,
    getItemKey: (index) => flows[index]?.id ?? index,
    overscan: 12,
  });

  if (loading) return <TableSkeleton />;

  if (flows.length === 0) {
    return (
      <EmptyState
        icon={<Network aria-hidden size={20} />}
        title={filtered ? "No connections match these filters" : "No connections yet"}
        description={
          filtered
            ? "Clear or widen the filters to see the rest of the live connections."
            : "Start capture and generate some network activity — connections appear here as packets are grouped into flows."
        }
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
        <span>State</span>
        <span>Dir</span>
        <span>Remote</span>
        <span>Protocol</span>
        <span>Process</span>
        <span>User</span>
        <span>Rule</span>
        <span className="text-right">Bytes</span>
        <span className="text-right">Duration</span>
        <span className="sr-only">Actions</span>
      </div>
      <div ref={scrollRef} className="h-table-body scrollbar-gutter-stable overflow-auto bg-surface">
        <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
          {virtualizer.getVirtualItems().map((row) => {
            const flow = flows[row.index];
            if (!flow) return null;
            return (
              <ConnectionRow
                key={flow.id}
                flow={flow}
                offset={row.start}
                height={row.size}
                onRowMenu={onRowMenu}
              />
            );
          })}
        </div>
      </div>
    </div>
  );
}

type ConnectionRowProps = {
  flow: Flow;
  offset: number;
  height: number;
  onRowMenu: (flow: Flow, anchor: MenuAnchor) => void;
};

function ConnectionRow({ flow, offset, height, onRowMenu }: ConnectionRowProps): JSX.Element {
  return (
    <div
      className={cn(
        "virt-row group absolute left-0 grid w-full items-center gap-3 border-b border-border px-3 text-sm hover:bg-surface-muted",
        COLUMNS,
        !flow.active && "text-foreground-muted",
        // A blocked conversation is tinted as well as badged, so it stands out in a long
        // list without relying on the badge alone (`ui_rules.md` §5).
        flow.blocked && "virt-row-blocked",
      )}
      style={{ height, top: offset }}
      title={`First seen ${formatAbsoluteTime(flow.firstSeen)} · last seen ${formatAbsoluteTime(flow.lastSeen)}`}
      onContextMenu={(event) => {
        event.preventDefault();
        onRowMenu(flow, { x: event.clientX, y: event.clientY });
      }}
    >
      <StateBadge flow={flow} />
      <DirectionIcon direction={flow.direction} />
      <span
        className="truncate font-mono text-xs"
        title={formatAddressPort(flow.remoteAddress, flow.remotePort)}
      >
        {formatEndpoint(flow.remoteHost, flow.remoteAddress)}
        {flow.remotePort === null ? null : (
          <span className="text-foreground-subtle">:{flow.remotePort}</span>
        )}
      </span>
      <ProtocolBadge
        protocol={flow.protocol}
        token={flow.protocolToken}
        className="justify-self-start"
      />
      <span
        className="truncate text-xs"
        title={flow.processPath ?? (flow.pid === null ? "Owner not resolved" : `PID ${flow.pid}`)}
      >
        {flow.processName ?? (flow.pid === null ? UNKNOWN : `PID ${flow.pid}`)}
      </span>
      <span className="truncate font-mono text-xs" title={flow.userSid ?? undefined}>
        {flow.userAccount ?? UNKNOWN}
      </span>
      <span
        className={cn("truncate text-xs", flow.blocked ? "text-danger-strong" : "text-foreground-subtle")}
        title={
          flow.matchedRuleName === null
            ? "No rule has acted on this connection"
            : `Blocked by rule "${flow.matchedRuleName}"`
        }
      >
        {flow.matchedRuleName ?? UNKNOWN}
      </span>
      <span
        className="text-right font-mono text-xs"
        title={`${flow.bytesSent} sent / ${flow.bytesRecv} received over ${flow.packets} packets`}
      >
        {formatBytes(flow.bytesSent + flow.bytesRecv)}
      </span>
      <span className="text-right font-mono text-xs">{formatDuration(flow.durationMs)}</span>
      {/* Hover/focus-revealed so a dense table stays calm but the action is reachable by
          keyboard; right-clicking the row opens the same menu. */}
      <span className="flex justify-end opacity-0 transition-opacity duration-fast group-hover:opacity-100 group-focus-within:opacity-100">
        <IconButton
          icon={<MoreHorizontal aria-hidden className="size-3.5" />}
          label={`Actions for ${formatEndpoint(flow.remoteHost, flow.remoteAddress)}`}
          onClick={(event) => {
            const rect = event.currentTarget.getBoundingClientRect();
            onRowMenu(flow, { x: rect.left, y: rect.bottom });
          }}
        />
      </span>
    </div>
  );
}

/** Blocked outranks live/closed: it is the fact the user needs first. */
function StateBadge({ flow }: { flow: Flow }): JSX.Element {
  if (flow.blocked) {
    return (
      <Badge variant="danger" className="justify-self-start">
        <ShieldX aria-hidden size={12} />
        Blocked
      </Badge>
    );
  }
  return (
    <Badge variant={flow.active ? "success" : "neutral"} className="justify-self-start">
      {flow.active ? "Live" : "Closed"}
    </Badge>
  );
}
