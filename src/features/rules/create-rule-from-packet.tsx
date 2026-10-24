import {
  CreateRuleFromTraffic,
  type ScopeChoice,
} from "@/features/rules/create-rule-from-traffic";
import { draftRuleFromPacket } from "@/lib/ipc";
import type { PacketDetail } from "@/types/ipc";

export type CreateRuleFromPacketProps = {
  packet: PacketDetail;
  anchor: { x: number; y: number };
  onClose: () => void;
};

/** The inspector's footer action (`ui_rules.md` §7). Rust resolves the packet back to its
 *  connection, so a rule built here is identical to one built from the Connections row. */
export function CreateRuleFromPacket({
  packet,
  anchor,
  onClose,
}: CreateRuleFromPacketProps): JSX.Element | null {
  const scopes: ScopeChoice[] = [
    { scope: "endpoint", action: "block", label: "Block this connection" },
    { scope: "host", action: "block", label: "Block everything to this host" },
    { scope: "port", action: "block", label: "Block this port" },
    {
      scope: "process",
      action: "block",
      label: packet.attribution.processName
        ? `Block all of ${packet.attribution.processName}`
        : "Block this process",
      enabled: packet.attribution.processPath !== null,
    },
    { scope: "endpoint", action: "allow", label: "Allow this connection" },
  ];

  return (
    <CreateRuleFromTraffic
      anchor={anchor}
      subject={`packet #${packet.id}`}
      scopes={scopes}
      // Opened from the inspector drawer, so the menu has to clear it.
      elevated
      loadDraft={(scope, action) => draftRuleFromPacket(packet.id, scope, action)}
      onClose={onClose}
    />
  );
}
