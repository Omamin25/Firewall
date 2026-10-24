import type { ReactNode } from "react";

export type LayerSectionProps = { title: string; children: ReactNode };

/** One titled layer group inside the inspector (L2 → L7 + attribution). */
export function LayerSection({ title, children }: LayerSectionProps): JSX.Element {
  return (
    <section className="border-b border-border py-3 last:border-b-0">
      <h3 className="mb-1 text-xs font-medium uppercase tracking-wide text-foreground-muted">
        {title}
      </h3>
      <dl>{children}</dl>
    </section>
  );
}
