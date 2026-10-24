import { Area, AreaChart, ResponsiveContainer, YAxis } from "recharts";
import { cn } from "@/lib/cn";

export type SparklinePoint = { t: number; value: number };

export type SparklineProps = {
  /** Oldest sample first. Fewer than two points renders a flat baseline. */
  series: readonly SparklinePoint[];
  /** Token reference, e.g. `hsl(var(--primary-strong))`. Never a literal colour. The
   *  stroke is a non-text mark, so it uses a `-strong` pair (`ui_tokens.md` §1.1). */
  color?: string;
  /** Accessible description of what the line shows. */
  label: string;
  /** Height comes from the caller's layout class, not a prop. */
  className?: string;
};

const GRADIENT_ID = "sparkline-fill";

/** Throughput sparkline (`library_docs.md` §7 — Recharts, minimal, tokenized). No axes,
 *  no tooltip, no animation: it updates once a second and must not repaint the page. */
export function Sparkline({
  series,
  color = "hsl(var(--primary-strong))",
  label,
  className,
}: SparklineProps): JSX.Element {
  return (
    <div className={cn("w-full", className)} role="img" aria-label={label}>
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart data={[...series]} margin={{ top: 0, right: 0, bottom: 0, left: 0 }}>
          <defs>
            <linearGradient id={GRADIENT_ID} x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor={color} stopOpacity={0.28} />
              <stop offset="100%" stopColor={color} stopOpacity={0} />
            </linearGradient>
          </defs>
          {/* A hidden axis anchored at zero keeps idle stretches on the baseline instead
              of amplifying noise into a full-height wobble. */}
          <YAxis hide domain={[0, "dataMax"]} />
          <Area
            type="monotone"
            dataKey="value"
            stroke={color}
            strokeWidth={1.5}
            fill={`url(#${GRADIENT_ID})`}
            isAnimationActive={false}
            dot={false}
            activeDot={false}
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}
