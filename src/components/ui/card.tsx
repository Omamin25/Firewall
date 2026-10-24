import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

export type CardProps = { title?: string; action?: ReactNode; children: ReactNode; className?: string };

export function Card({ title, action, children, className }: CardProps): JSX.Element {
  return <section className={cn("rounded-lg border border-border bg-surface shadow-sm", className)}>{title ? <header className="flex items-center justify-between border-b border-border px-4 py-3"><h2 className="text-h2 font-semibold tracking-tight">{title}</h2>{action}</header> : null}<div className="p-4">{children}</div></section>;
}
