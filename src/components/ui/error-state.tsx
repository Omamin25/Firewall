import type { ReactNode } from "react";
import { TriangleAlert } from "lucide-react";

export type ErrorStateProps = { title: string; detail: string; action?: ReactNode };

export function ErrorState({ title, detail, action }: ErrorStateProps): JSX.Element {
  return <section className="flex items-start gap-3 rounded-lg border border-danger bg-danger/10 p-4"><TriangleAlert aria-hidden className="shrink-0 text-danger-strong" /><div><h2 className="text-h2 font-semibold">{title}</h2><p className="mt-1 text-sm text-foreground-muted">{detail}</p>{action ? <div className="mt-3">{action}</div> : null}</div></section>;
}
