import { KeyValueRow } from "@/components/ui/key-value-row";
import { LayerSection } from "@/features/inspector/layer-section";
import type { ApplicationLayer } from "@/types/ipc";

export type ApplicationLayerViewProps = { application: ApplicationLayer };

/** L7 detail rows, one shape per classifier. Mirrors `ipc::dto::ApplicationLayerDto`. */
export function ApplicationLayerView({ application }: ApplicationLayerViewProps): JSX.Element {
  switch (application.type) {
    case "dns":
      return (
        <LayerSection title="L7 · DNS">
          <KeyValueRow label="Message" value={application.isResponse ? "Response" : "Query"} />
          <KeyValueRow label="Questions" value={<NameList names={application.questions} />} mono />
          {application.answers.length > 0 ? (
            <KeyValueRow label="Answers" value={<NameList names={application.answers} />} mono />
          ) : null}
        </LayerSection>
      );
    case "mdns":
      return (
        <LayerSection title="L7 · mDNS">
          <KeyValueRow label="Message" value={application.isResponse ? "Response" : "Query"} />
          <KeyValueRow label="Records" value={<NameList names={application.questions} />} mono />
        </LayerSection>
      );
    case "http":
      return (
        <LayerSection title="L7 · HTTP">
          {application.method ? <KeyValueRow label="Method" value={application.method} /> : null}
          {application.host ? <KeyValueRow label="Host" value={application.host} mono /> : null}
          {application.path ? <KeyValueRow label="Path" value={application.path} mono /> : null}
          {application.status !== null ? (
            <KeyValueRow label="Status" value={application.status} mono />
          ) : null}
        </LayerSection>
      );
    case "tls":
      return (
        <LayerSection title="L7 · TLS">
          <KeyValueRow label="Record" value={application.handshake} />
          {application.version ? <KeyValueRow label="Version" value={application.version} /> : null}
          <KeyValueRow
            label="SNI"
            value={application.sni ?? "Not present in this record"}
            mono={application.sni !== null}
          />
        </LayerSection>
      );
    case "quic":
      return (
        <LayerSection title="L7 · QUIC">
          <KeyValueRow
            label="Header"
            value={application.longHeader ? "Long (handshake)" : "Short (1-RTT)"}
          />
          <KeyValueRow label="SNI" value={application.sni ?? "Encrypted in the QUIC handshake"} mono={application.sni !== null} />
        </LayerSection>
      );
    case "dhcp":
      return (
        <LayerSection title="L7 · DHCP">
          <KeyValueRow label="Message type" value={application.messageType} />
        </LayerSection>
      );
    case "opaque":
      return (
        <LayerSection title="L7 · Unclassified">
          <KeyValueRow label="Best guess" value={application.hint} />
        </LayerSection>
      );
  }
}

function NameList({ names }: { names: string[] }): JSX.Element {
  if (names.length === 0) return <span className="text-foreground-muted">None</span>;
  return (
    <ul className="space-y-0.5">
      {names.map((name) => (
        <li key={name}>{name}</li>
      ))}
    </ul>
  );
}
