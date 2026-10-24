import type { InputHTMLAttributes } from "react";
import { cn } from "@/lib/cn";

export type InputProps = InputHTMLAttributes<HTMLInputElement> & {
  invalid?: boolean;
  /** Technical values (IP, port, CIDR) render in mono per `ui_tokens.md` §2. */
  mono?: boolean;
};

/** Text field styled per `ui_rules.md` §4. */
export function Input({ invalid = false, mono = false, className, ...props }: InputProps): JSX.Element {
  return (
    <input
      aria-invalid={invalid || undefined}
      className={cn(
        "h-9 w-full rounded-md border bg-background px-3 text-sm text-foreground placeholder:text-foreground-subtle",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
        "disabled:cursor-not-allowed disabled:opacity-50",
        invalid ? "border-danger" : "border-border focus-visible:border-border-strong",
        mono && "font-mono",
        className,
      )}
      {...props}
    />
  );
}
