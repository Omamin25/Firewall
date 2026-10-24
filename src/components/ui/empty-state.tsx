import type { ReactNode } from "react";

export type EmptyStateProps = { icon: ReactNode; title: string; description: string };
export function EmptyState({ icon, title, description }: EmptyStateProps): JSX.Element {
  return <div className="flex min-h-40 flex-col items-center justify-center gap-2 text-center text-foreground-muted">{icon}<h2 className="text-h2 font-semibold text-foreground">{title}</h2><p className="text-sm">{description}</p></div>;
}
