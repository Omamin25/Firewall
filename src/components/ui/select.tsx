import type { SelectHTMLAttributes } from "react";
import { cn } from "@/lib/cn";

export type SelectOption = { value: string; label: string };

export type SelectProps = Omit<SelectHTMLAttributes<HTMLSelectElement>, "children"> & {
  options: readonly SelectOption[];
};

/** Native select styled to match [`Input`](./input.tsx) (`ui_rules.md` §4). Native keeps
 *  keyboard behaviour and screen-reader support correct for free. */
export function Select({ options, className, ...props }: SelectProps): JSX.Element {
  return (
    <select
      className={cn(
        "h-9 w-full rounded-md border border-border bg-background px-2 text-sm text-foreground",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
        "disabled:cursor-not-allowed disabled:opacity-50",
        className,
      )}
      {...props}
    >
      {options.map((option) => (
        <option key={option.value} value={option.value}>
          {option.label}
        </option>
      ))}
    </select>
  );
}
