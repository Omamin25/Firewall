import { Pause, Play, Square } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { formatCount, formatRate } from "@/lib/formatters";
import { useTrafficStore } from "@/store/use-traffic-store";

export type CaptureToolbarProps = {
  onStart: () => void;
  onStop: () => void;
};

/** Capture controls plus the live throughput/loss strip. */
export function CaptureToolbar({ onStart, onStop }: CaptureToolbarProps): JSX.Element {
  const captureState = useTrafficStore((state) => state.captureState);
  const stats = useTrafficStore((state) => state.stats);
  const paused = useTrafficStore((state) => state.paused);
  const setPaused = useTrafficStore((state) => state.setPaused);
  const interfaces = useTrafficStore((state) => state.interfaces);

  const running = captureState === "running";
  const starting = captureState === "starting";

  return (
    <div className="flex flex-wrap items-center justify-between gap-3">
      <div className="flex items-center gap-2">
        {running ? (
          <Button variant="outline" size="sm" icon={<Square aria-hidden className="size-3.5" />} onClick={onStop}>
            Stop capture
          </Button>
        ) : (
          <Button
            variant="accent"
            size="sm"
            icon={<Play aria-hidden className="size-3.5" />}
            onClick={onStart}
            disabled={starting}
          >
            {starting ? "Starting…" : "Start capture"}
          </Button>
        )}
        <Button
          variant="ghost"
          size="sm"
          icon={paused ? <Play aria-hidden className="size-3.5" /> : <Pause aria-hidden className="size-3.5" />}
          onClick={() => setPaused(!paused)}
          disabled={!running}
        >
          {paused ? "Resume" : "Pause"}
        </Button>
        {running ? (
          <Badge variant={paused ? "warning" : "success"}>
            {paused ? "Paused" : "Live"}
          </Badge>
        ) : null}
      </div>

      <dl className="flex flex-wrap items-center gap-x-6 gap-y-1 text-xs text-foreground-muted">
        <Stat label="Throughput" value={formatRate(stats.bytesPerSecond)} />
        <Stat label="Packets/s" value={formatCount(stats.packetsPerSecond)} />
        <Stat label="Captured" value={formatCount(stats.packetsCaptured)} />
        <Stat label="Connections" value={formatCount(stats.activeFlows)} />
        <Stat
          label="Dropped"
          value={formatCount(stats.packetsDropped)}
          emphasis={stats.packetsDropped > 0}
        />
        {running ? <Stat label="Interfaces" value={formatCount(interfaces.length)} /> : null}
      </dl>
    </div>
  );
}

type StatProps = { label: string; value: string; emphasis?: boolean };

function Stat({ label, value, emphasis = false }: StatProps): JSX.Element {
  return (
    <div className="flex items-baseline gap-1.5">
      <dt>{label}</dt>
      <dd className={emphasis ? "font-mono text-warning" : "font-mono text-foreground"}>{value}</dd>
    </div>
  );
}
