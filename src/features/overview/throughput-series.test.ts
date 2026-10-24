import { describe, expect, it } from "vitest";
import {
  appendSample,
  seriesAverage,
  seriesPeak,
  seriesSpanSeconds,
  THROUGHPUT_CAPACITY,
  type ThroughputSample,
} from "@/features/overview/throughput-series";

describe("appendSample", () => {
  it("records the first reading immediately", () => {
    expect(appendSample([], 1024, 1_000)).toEqual([{ t: 1_000, value: 1024 }]);
  });

  it("keeps the same array when called again inside the interval", () => {
    const series = appendSample([], 1024, 1_000);
    expect(appendSample(series, 2048, 1_500)).toBe(series);
  });

  it("appends once the interval has elapsed", () => {
    const series = appendSample([], 1024, 1_000);
    expect(appendSample(series, 2048, 2_000)).toEqual([
      { t: 1_000, value: 1024 },
      { t: 2_000, value: 2048 },
    ]);
  });

  it("clamps negative readings to zero", () => {
    expect(appendSample([], -5, 1_000)).toEqual([{ t: 1_000, value: 0 }]);
  });

  it("drops the oldest samples past capacity", () => {
    let series: ThroughputSample[] = [];
    for (let tick = 0; tick <= THROUGHPUT_CAPACITY + 10; tick += 1) {
      series = appendSample(series, tick, tick * 1_000);
    }
    expect(series).toHaveLength(THROUGHPUT_CAPACITY);
    expect(series[0]?.value).toBe(11);
    expect(series[series.length - 1]?.value).toBe(THROUGHPUT_CAPACITY + 10);
  });
});

describe("series summaries", () => {
  const series: ThroughputSample[] = [
    { t: 1_000, value: 100 },
    { t: 2_000, value: 300 },
    { t: 5_000, value: 200 },
  ];

  it("reports the peak reading", () => {
    expect(seriesPeak(series)).toBe(300);
    expect(seriesPeak([])).toBe(0);
  });

  it("averages the window", () => {
    expect(seriesAverage(series)).toBe(200);
    expect(seriesAverage([])).toBe(0);
  });

  it("measures the window in whole seconds", () => {
    expect(seriesSpanSeconds(series)).toBe(4);
    expect(seriesSpanSeconds([])).toBe(0);
  });
});
