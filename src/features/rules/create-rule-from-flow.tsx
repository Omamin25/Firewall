import { Copy } from "lucide-react";
import type { MenuEntry } from "@/components/ui/menu";
import {
  CreateRuleFromTraffic,
  type ScopeChoice,
} from "@/features/rules/create-rule-from-traffic";
import { fileName } from "@/features/rules/rule-criteria";
import { copyToClipboard } from "@/lib/clipboard";
import { formatAddressPort, formatEndpoint } from "@/lib/formatters";
import { draftRuleFromFlow } from "@/lib/ipc";
import type { Flow } from "@/types/ipc";

export type CreateRuleFromFlowProps = {
  flow: Flow;
  anchor: { x: number; y: number };
  onClose: () => void;
};

/** Row menu for a connection: build a rule from it at one of four scopes, or copy its
 *  identifiers (`ui_rules.md` §5). */
export function CreateRuleFromFlow({
  flow,
  anchor,
  onClose,
}: CreateRuleFromFlowProps): JSX.Element | null {
  const remote = formatEndpoint(flow.remoteHost, flow.remoteAddress);
  const process = flow.processName ?? (flow.processPath && fileName(flow.processPath));

  const scopes: ScopeChoice[] = [
    { scope: "endpoint", action: "block", label: "Block this connection" },
    { scope: "host", action: "block", label: `Block everything to ${remote}` },
    {
      scope: "port",
      action: "block",
      label: flow.remotePort === null ? "Block this port" : `Block port ${flow.remotePort}`,
      enabled: flow.remotePort !== null,
    },
    {
      scope: "process",
      action: "block",
      label: process ? `Block all of ${process}` : "Block this process",
      // A rule naming an unresolved process could never match anything.
      enabled: flow.processPath !== null,
    },
    { scope: "endpoint", action: "allow", label: "Allow this connection" },
  ];

  const extraItems: MenuEntry[] = [
    { kind: "label", label: "Copy" },
    {
      kind: "action",
      label: "Copy remote address",
      icon: <Copy aria-hidden className="size-3.5 shrink-0" />,
      onSelect: () => {
        void copyToClipboard(
          formatAddressPort(flow.remoteAddress, flow.remotePort),
          "Remote address",
        );
      },
    },
    {
      kind: "action",
      label: "Copy process path",
      icon: <Copy aria-hidden className="size-3.5 shrink-0" />,
      disabled: flow.processPath === null,
      onSelect: () => {
        if (flow.processPath) void copyToClipboard(flow.processPath, "Process path");
      },
    },
  ];

  return (
    <CreateRuleFromTraffic
      anchor={anchor}
      subject={`${remote}${flow.remotePort === null ? "" : `:${flow.remotePort}`}`}
      scopes={scopes}
      extraItems={extraItems}
      loadDraft={(scope, action) => draftRuleFromFlow(flow.id, scope, action)}
      onClose={onClose}
    />
  );
}
