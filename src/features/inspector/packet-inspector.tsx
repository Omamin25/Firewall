import { useEffect, useState } from "react";
import { ShieldPlus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Drawer } from "@/components/ui/drawer";
import { ErrorState } from "@/components/ui/error-state";
import { KeyValueRow } from "@/components/ui/key-value-row";
import { ProtocolBadge } from "@/components/data/protocol-badge";
import { CreateRuleFromPacket } from "@/features/rules/create-rule-from-packet";
import { ApplicationLayerView } from "@/features/inspector/application-layer-view";
import { LayerSection } from "@/features/inspector/layer-section";
import { getPacketDetail } from "@/lib/ipc";
import { formatBytes, formatTime } from "@/lib/formatters";
import { useTrafficStore } from "@/store/use-traffic-store";
import type { NetworkLayer, PacketDetail, TransportLayer } from "@/types/ipc";

/** Right-side drawer with the full L2→L7 breakdown for the selected packet. */
export function PacketInspector(): JSX.Element | null {
  const selectedPacketId = useTrafficStore((state) => state.selectedPacketId);
  const selectPacket = useTrafficStore((state) => state.selectPacket);
  const [detail, setDetail] = useState<PacketDetail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [ruleMenu, setRuleMenu] = useState<{ x: number; y: number } | null>(null);

  useEffect(() => {
    if (selectedPacketId === null) {
      setDetail(null);
      setError(null);
      return;
    }
    let active = true;
    setLoading(true);
    setError(null);
    getPacketDetail(selectedPacketId)
      .then((result) => {
        if (active) setDetail(result);
      })
      .catch((cause: unknown) => {
        if (active) setError(typeof cause === "string" ? cause : "Could not load this packet.");
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [selectedPacketId]);

  if (selectedPacketId === null) return null;

  return (
    <>
      <Drawer
        open
        onClose={() => selectPacket(null)}
        title="Packet inspector"
        footer={
          detail ? (
            // Pinned primary action from `ui_rules.md` §7.
            <Button
              variant="accent"
              size="sm"
              icon={<ShieldPlus aria-hidden className="size-3.5" />}
              onClick={(event) => {
                const rect = event.currentTarget.getBoundingClientRect();
                setRuleMenu({ x: rect.left, y: rect.top });
              }}
            >
              Create rule from this packet
            </Button>
          ) : undefined
        }
      >
        {loading ? <InspectorSkeleton /> : null}
        {error ? <ErrorState title="Packet unavailable" detail={error} /> : null}
        {detail && !loading ? <PacketLayers detail={detail} /> : null}
      </Drawer>
      {detail && ruleMenu ? (
        <CreateRuleFromPacket
          packet={detail}
          anchor={ruleMenu}
          onClose={() => setRuleMenu(null)}
        />
      ) : null}
    </>
  );
}

function PacketLayers({ detail }: { detail: PacketDetail }): JSX.Element {
  return (
    <div>
      <div className="mb-2 flex items-center gap-2">
        <ProtocolBadge protocol={detail.protocol} token={detail.protocolToken} />
        <span className="text-xs text-foreground-muted">
          {detail.direction === "out" ? "Outbound" : "Inbound"}
        </span>
      </div>

      <LayerSection title="Frame">
        <KeyValueRow label="Packet #" value={detail.id} mono />
        <KeyValueRow label="Time" value={formatTime(detail.timestamp)} mono />
        <KeyValueRow label="Interface" value={detail.interface} mono />
        <KeyValueRow label="Wire length" value={formatBytes(detail.frameLen)} mono />
        {detail.capturedLen < detail.frameLen ? (
          <KeyValueRow label="Captured" value={formatBytes(detail.capturedLen)} mono />
        ) : null}
        <KeyValueRow label="Payload" value={formatBytes(detail.payloadLen)} mono />
      </LayerSection>

      <LayerSection title="L2 · Ethernet">
        <KeyValueRow label="Source MAC" value={detail.link.sourceMac} mono />
        <KeyValueRow label="Destination MAC" value={detail.link.destinationMac} mono />
        <KeyValueRow label="EtherType" value={detail.link.etherType} mono />
      </LayerSection>

      {detail.network ? <NetworkLayerView network={detail.network} /> : null}
      {detail.transport ? <TransportLayerView transport={detail.transport} /> : null}
      {detail.application ? <ApplicationLayerView application={detail.application} /> : null}

      <LayerSection title="Attribution">
        {detail.attribution.pid !== null ? (
          <>
            <KeyValueRow label="Process" value={detail.attribution.processName ?? "—"} />
            <KeyValueRow label="PID" value={detail.attribution.pid} mono />
            <KeyValueRow label="Path" value={detail.attribution.processPath ?? "—"} mono />
            <KeyValueRow label="User" value={detail.attribution.userAccount ?? "—"} mono />
            {detail.attribution.userSid ? (
              <KeyValueRow label="SID" value={detail.attribution.userSid} mono />
            ) : null}
          </>
        ) : (
          <p className="text-xs text-foreground-muted">
            No owning process. ICMP and ARP have no socket, and a connection can be gone
            from the OS table before its last frames are decoded.
          </p>
        )}
      </LayerSection>
    </div>
  );
}

function NetworkLayerView({ network }: { network: NetworkLayer }): JSX.Element {
  switch (network.type) {
    case "ipv4":
      return (
        <LayerSection title="L3 · IPv4">
          <KeyValueRow label="Source" value={network.source} mono />
          <KeyValueRow label="Destination" value={network.destination} mono />
          <KeyValueRow label="TTL" value={network.ttl} mono />
          <KeyValueRow label="Protocol" value={network.protocol} mono />
          <KeyValueRow
            label="Fragmentation"
            value={fragmentationLabel(network.dontFragment, network.moreFragments, network.fragmentOffset)}
          />
        </LayerSection>
      );
    case "ipv6":
      return (
        <LayerSection title="L3 · IPv6">
          <KeyValueRow label="Source" value={network.source} mono />
          <KeyValueRow label="Destination" value={network.destination} mono />
          <KeyValueRow label="Hop limit" value={network.hopLimit} mono />
          <KeyValueRow label="Next header" value={network.nextHeader} mono />
        </LayerSection>
      );
    case "arp":
      return (
        <LayerSection title="L3 · ARP">
          <KeyValueRow label="Operation" value={network.operation} />
          <KeyValueRow label="Sender IP" value={network.senderIp} mono />
          <KeyValueRow label="Target IP" value={network.targetIp} mono />
        </LayerSection>
      );
  }
}

function TransportLayerView({ transport }: { transport: TransportLayer }): JSX.Element {
  switch (transport.type) {
    case "tcp":
      return (
        <LayerSection title="L4 · TCP">
          <KeyValueRow label="Source port" value={transport.sourcePort} mono />
          <KeyValueRow label="Destination port" value={transport.destinationPort} mono />
          <KeyValueRow
            label="Flags"
            value={transport.flags.length > 0 ? transport.flags.join(", ") : "None"}
            mono
          />
          <KeyValueRow label="Sequence" value={transport.sequence} mono />
          <KeyValueRow label="Acknowledgment" value={transport.acknowledgment} mono />
          <KeyValueRow label="Window" value={transport.window} mono />
        </LayerSection>
      );
    case "udp":
      return (
        <LayerSection title="L4 · UDP">
          <KeyValueRow label="Source port" value={transport.sourcePort} mono />
          <KeyValueRow label="Destination port" value={transport.destinationPort} mono />
          <KeyValueRow label="Length" value={transport.length} mono />
        </LayerSection>
      );
    case "icmp":
      return (
        <LayerSection title="L4 · ICMP">
          <KeyValueRow label="Type" value={`${transport.description} (${transport.kind})`} />
          <KeyValueRow label="Code" value={transport.code} mono />
        </LayerSection>
      );
    case "igmp":
      return (
        <LayerSection title="L4 · IGMP">
          <KeyValueRow label="Type" value={`${transport.description} (${transport.kind})`} />
        </LayerSection>
      );
  }
}

function fragmentationLabel(
  dontFragment: boolean,
  moreFragments: boolean,
  offset: number,
): string {
  const parts: string[] = [];
  if (dontFragment) parts.push("Don't fragment");
  if (moreFragments) parts.push("More fragments");
  if (offset > 0) parts.push(`Offset ${offset}`);
  return parts.length > 0 ? parts.join(" · ") : "None";
}

function InspectorSkeleton(): JSX.Element {
  return (
    <div className="space-y-3" aria-hidden>
      {[0, 1, 2, 3].map((row) => (
        <div
          key={row}
          className="space-y-2 border-b border-border pb-3 motion-safe:animate-pulse"
        >
          <div className="h-3 w-24 rounded bg-surface-muted" />
          <div className="h-3 w-full rounded bg-surface-muted" />
          <div className="h-3 w-2/3 rounded bg-surface-muted" />
        </div>
      ))}
    </div>
  );
}
