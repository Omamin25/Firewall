import { cn } from "@/lib/cn";

export type SwitchProps = {
  checked: boolean;
  onChange: (checked: boolean) => void;
  /** Accessible name — the switch itself carries no visible text. */
  label: string;
  disabled?: boolean;
  className?: string;
};

/** Toggle used for rule enable/disable. A native button with `role="switch"` keeps
 *  keyboard and screen-reader behaviour correct; state is never colour-only because
 *  `aria-checked` and the knob position both carry it (`ui_rules.md` §10). */
export function Switch({
  checked,
  onChange,
  label,
  disabled = false,
  className,
}: SwitchProps): JSX.Element {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cn(
        "inline-flex h-5 w-9 shrink-0 items-center rounded-full border transition-colors duration-fast ease-enter",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
        "disabled:cursor-not-allowed disabled:opacity-50",
        checked ? "border-success bg-success/30" : "border-border-strong bg-surface-muted",
        className,
      )}
    >
      <span
        aria-hidden
        className={cn(
          "size-4 rounded-full transition-transform duration-fast ease-enter",
          // The knob is a non-text mark, so it uses the `-strong` pair to stay visible
          // against the track (`ui_tokens.md` §1.1).
          checked ? "translate-x-4 bg-success-strong" : "translate-x-1 bg-foreground-subtle",
        )}
      />
    </button>
  );
}
