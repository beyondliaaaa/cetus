import { describe, expect, it } from "vitest";
import type { TimeMeta } from "@cetus/sim-client";
import {
  appendProbePoints,
  createSeriesBuffer,
  createSeriesStore,
  downsampleSeries,
  tickToDisplayTime,
} from "./series";

const TIME: TimeMeta = {
  unit: "day",
  unitFactor: 86_400,
  dt: 0.05,
  horizon: 160,
  integrator: "rk4",
  kind: "continuous",
  start: 0,
  startTick: 0,
};

function pairPoints(values: readonly number[]): Float64Array {
  return Float64Array.from(values);
}

describe("probe accumulation", () => {
  it("maps ticks to display time and SI values to display values", () => {
    const buffer = createSeriesBuffer();
    const appended = appendProbePoints(
      buffer,
      pairPoints([1, 10, 2, 20]),
      TIME,
      1,
    );

    expect(appended).toBe(2);
    expect(buffer.x).toEqual([0.05, 0.1]);
    expect(buffer.y).toEqual([10, 20]);
    expect(buffer.lastTick).toBe(2);
  });

  it("applies the observable unit factor (percent)", () => {
    const buffer = createSeriesBuffer();
    appendProbePoints(buffer, pairPoints([1, 0.5, 2, 0.75]), TIME, 0.01, 0);
    expect(buffer.y).toEqual([50, 75]);
  });

  it("respects the model start offset", () => {
    const shifted: TimeMeta = { ...TIME, start: 5, startTick: 100, dt: 0.5 };
    expect(tickToDisplayTime(104, shifted)).toBe(7);
  });

  it("drops duplicate and non-monotonic ticks", () => {
    const buffer = createSeriesBuffer();
    appendProbePoints(buffer, pairPoints([1, 1, 2, 2, 2, 99]), TIME, 1);
    const appended = appendProbePoints(
      buffer,
      pairPoints([2, 5, 3, 7, 1, 4]),
      TIME,
      1,
    );

    expect(appended).toBe(1);
    expect(buffer.x).toHaveLength(3);
    expect(buffer.x[2]).toBeCloseTo(0.15, 12);
    expect(buffer.y).toEqual([1, 2, 7]);
    expect(buffer.lastTick).toBe(3);
  });

  it("ignores malformed pairs without breaking the x axis", () => {
    const buffer = createSeriesBuffer();
    appendProbePoints(
      buffer,
      pairPoints([1, Number.NaN, 2, 4, 3]),
      TIME,
      1,
    );
    expect(buffer.x).toEqual([0.1]);
    expect(buffer.y).toEqual([4]);
  });

  it("creates one empty buffer per observable", () => {
    const store = createSeriesStore(["a", "b"]);
    expect(Object.keys(store)).toEqual(["a", "b"]);
    expect(store["a"]?.lastTick).toBe(Number.NEGATIVE_INFINITY);
  });
});

describe("downsampling", () => {
  it("keeps the first and last sample and a monotonic x axis", () => {
    const x = Array.from({ length: 100 }, (_, index) => index);
    const y = x.map((value) => value * 2);
    const sampled = downsampleSeries(x, [y], 10);

    expect(sampled.x.length).toBeLessThanOrEqual(10);
    expect(sampled.x[0]).toBe(0);
    expect(sampled.x[sampled.x.length - 1]).toBe(99);
    expect(sampled.ys[0]?.length).toBe(sampled.x.length);
    for (let index = 1; index < sampled.x.length; index += 1) {
      expect(sampled.x[index]).toBeGreaterThan(sampled.x[index - 1] ?? -Infinity);
    }
  });

  it("returns a copy unchanged when the series is short", () => {
    const sampled = downsampleSeries([0, 1, 2], [[3, 4, 5]], 10);
    expect(sampled.x).toEqual([0, 1, 2]);
    expect(sampled.ys[0]).toEqual([3, 4, 5]);
  });

  it("keeps multiple y series aligned with the sampled x axis", () => {
    const x = Array.from({ length: 40 }, (_, index) => index);
    const sampled = downsampleSeries(
      x,
      [
        x.map((value) => value),
        x.map((value) => value * 10),
      ],
      7,
    );
    expect(sampled.ys[0]?.length).toBe(sampled.x.length);
    expect(sampled.ys[1]?.length).toBe(sampled.x.length);
  });
});
