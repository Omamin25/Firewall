import { useEffect, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { Radio } from "lucide-react";
import { DirectionIcon } from "@/components/data/direction-icon";
import { ProtocolBadge } from "@/components/data/protocol-badge";
import { EmptyState } from "@/components/ui/empty-state";
import { cn } from "@/lib/cn";
import { formatAddressPort, formatBytes, formatEndpoint, formatTime } from "@/lib/formatters";
import { useTrafficStore } from "@/store/use-traffic-store";
import type { Packet } from "@/types/ipc";

const ROW_HEIGHT = 36;
const COLUMNS =
  "grid-cols-[92px_68px_minmax(160px,1.3fr)_92px_minmax(120px,0.9fr)_minmax(150px,1.2fr)_80px]";

export type LiveTrafficTableProps = {
  /** Already filtered by the page (see `use-filtered-packets.ts`). */
  packets: Packet[];
  /** True when filters are hiding rows, so the empty state can say so. */
  filtered: boolean;
};

/** Virtualized live packet table. Rows are not animated — under load that costs more
 *  than it communicates (`ui_rules.md` §5). */
export function LiveTrafficTable({ packets, filtered }: LiveTrafficTableProps): JSX.Element {
  const scrollRef = useRef<HTMLDivElement>(null);
  const selectedPacketId = useTrafficStore((state) => state.selectedPacketId);
  const selectPacket = useTrafficStore((state) => state.selectPacket);
  // Roving focus target for arrow-key navigation (`ui_rules.md` §10).
  const [activeIndex, setActiveIndex] = useState<number | null>(null);

  const virtualizer = useVirtualizer({
    count: packets.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT,
    getItemKey: (index) => packets[index]?.id ?? index,
    overscan: 12,
  });

  // Focus follows the active row once the virtualizer has rendered it.
  useEffect(() => {
    if (activeIndex === null) return;
    scrollRef.current
      ?.querySelector<HTMLElement>(`[data-row-index="${activeIndex}"]`)
      ?.focus({ preventScroll: true });
  }, [activeIndex, packets]);

  const onKeyDown = (event: React.KeyboardEvent<HTMLDivElement>): void => {
    const last = packets.length - 1;
    if (last < 0) return;
    const current = activeIndex ?? 0;
    const next =
      event.key === "ArrowDown"
        ? Math.min(last, current + 1)
        : event.key === "ArrowUp"
          ? Math.max(0, current - 1)
          : event.key === "Home"
            ? 0
            : event.key === "End"
              ? last
              : null;
    if (next === null) return;
    event.preventDefault();
    virtualizer.scrollToIndex(next, { align: "auto" });
    setActiveIndex(next);
  };

  if (packets.length === 0) {
    return (
      <EmptyState
        icon={<Radio aria-hidden size={20} />}
        title={filtered ? "No packets match these filters" : "No traffic yet"}
        description={
          filtered
            ? "Clear or widen the filters to see the rest of the captured packets."
            : "Start capture and generate some network activity — packets appear here as they arrive."
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
        <span>Time</span>
        <span>Dir</span>
        <span>Remote</span>
        <span>Protocol</span>
        <span>Process</span>
        <span>Detail</span>
        <span className="text-right">Bytes</span>
      </div>
      <div
        ref={scrollRef}
        className="h-table-body scrollbar-gutter-stable overflow-auto bg-surface"
        onKeyDown={onKeyDown}
      >
        <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
          {virtualizer.getVirtualItems().map((row) => {
            const packet = packets[row.index];
            if (!packet) return null;
            return (
              <PacketRow
                key={packet.id}
                packet={packet}
                index={row.index}
                selected={packet.id === selectedPacketId}
                /** One row at a time is tabbable; arrows move within the table. */
                tabbable={row.index === (activeIndex ?? 0)}
                offset={row.start}
                height={row.size}
                onSelect={(id, index) => {
                  setActiveIndex(index);
                  selectPacket(id);
                }}
              />
            );
          })}
        </div>
      </div>
    </div>
  );
}

type PacketRowProps = {
  packet: Packet;
  index: number;
  selected: boolean;
  tabbable: boolean;
  offset: number;
  height: number;
  onSelect: (id: number, index: number) => void;
};

function PacketRow({
  packet,
  index,
  selected,
  tabbable,
  offset,
  height,
  onSelect,
}: PacketRowProps): JSX.Element {
  return (
    <button
      type="button"
      aria-current={selected}
      data-row-index={index}
      tabIndex={tabbable ? 0 : -1}
      onClick={() => onSelect(packet.id, index)}
      className={cn(
        "virt-row absolute left-0 grid w-full items-center gap-3 border-b border-border px-3 text-left text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring",
        COLUMNS,
        selected ? "virt-row-selected" : "hover:bg-surface-muted",
      )}
      style={{ height, top: offset }}
    >
      {/* 2px accent bar for the selected row, drawn over the row so nothing shifts
          (`ui_rules.md` §5). */}
      {selected ? (
        <span aria-hidden className="absolute inset-y-0 left-0 w-0.5 bg-primary-strong" />
      ) : null}
      <span className="font-mono text-xs text-foreground-muted">{formatTime(packet.timestamp)}</span>
      <DirectionIcon direction={packet.direction} />
      <span
        className="truncate font-mono text-xs"
        title={formatAddressPort(packet.remoteAddress, packet.remotePort)}
      >
        {formatEndpoint(packet.remoteHost, packet.remoteAddress)}
      </span>
      <ProtocolBadge
        protocol={packet.protocol}
        token={packet.protocolToken}
        className="justify-self-start"
      />
      <span className="truncate text-xs" title={attributionTitle(packet)}>
        {packet.processName ?? "—"}
      </span>
      <span className="truncate text-xs text-foreground-muted" title={packet.summary}>
        {packet.summary}
      </span>
      <span className="text-right font-mono text-xs" title={`${packet.bytes} bytes`}>
        {formatBytes(packet.bytes)}
      </span>
    </button>
  );
}

/** Process and user in one tooltip; the inspector shows the full attribution block. */
function attributionTitle(packet: Packet): string {
  if (!packet.processName && !packet.userAccount) {
    return "Owner not resolved — open the inspector for details";
  }
  return [packet.processName, packet.userAccount].filter(Boolean).join(" · ");
}
