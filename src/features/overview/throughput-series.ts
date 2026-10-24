/** One throughput reading: wall-clock millis and bytes per second at that moment. */
export type ThroughputSample = { t: number; value: number };

/** Stats arrive at the stream's ~15 Hz cadence, which is far more than a sparkline can
 *  show. One sample a second over two minutes is enough to read a trend and cheap enough
 *  that the array identity only changes once per second. */
export const THROUGHPUT_INTERVAL_MS = 1_000;
export const THROUGHPUT_CAPACITY = 120;

/** Appends a reading if enough time has passed, otherwise returns the series unchanged —
 *  the identical reference matters, because a new array would re-render the chart. */
export function appendSample(
  series: readonly ThroughputSample[],
  value: number,
  now: number,
  intervalMs: number = THROUGHPUT_INTERVAL_MS,
  capacity: number = THROUGHPUT_CAPACITY,
): ThroughputSample[] {
  const last = series[series.length - 1];
  if (last !== undefined && now - last.t < intervalMs) return series as ThroughputSample[];
  const next = [...series, { t: now, value: Math.max(0, value) }];
  return next.length > capacity ? next.slice(next.length - capacity) : next;
}

/** Highest reading in the window, for the sparkline's caption. */
export function seriesPeak(series: readonly ThroughputSample[]): number {
  return series.reduce((peak, sample) => Math.max(peak, sample.value), 0);
}

/** Mean reading over the window; 0 for an empty series. */
export function seriesAverage(series: readonly ThroughputSample[]): number {
  if (series.length === 0) return 0;
  return series.reduce((total, sample) => total + sample.value, 0) / series.length;
}

/** Seconds the window currently covers, for the "last N s" caption. */
export function seriesSpanSeconds(series: readonly ThroughputSample[]): number {
  const first = series[0];
  const last = series[series.length - 1];
  if (first === undefined || last === undefined) return 0;
  return Math.round((last.t - first.t) / 1_000);
}
