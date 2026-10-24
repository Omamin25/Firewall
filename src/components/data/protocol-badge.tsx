import type { ProtocolToken } from "@/types/ipc";
import { cn } from "@/lib/cn";
import { PROTOCOL_BADGE_CLASSES } from "@/lib/protocol-classes";

export type ProtocolBadgeProps = { protocol: string; token: ProtocolToken; className?: string };

/** Protocol label + color. The label is always shown, so meaning is never color-only
 *  (`ui_rules.md` §10). Colors come from the `--proto-*` tokens only. */
export function ProtocolBadge({ protocol, token, className }: ProtocolBadgeProps): JSX.Element {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium",
        PROTOCOL_BADGE_CLASSES[token],
        className,
      )}
    >
      {protocol}
    </span>
  );
}
