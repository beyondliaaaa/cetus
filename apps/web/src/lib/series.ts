/**
 * Pure helpers for turning incremental probe reads into chart-ready series.
 *
 * Probes arrive as interleaved `[tick, valueSi, ...]` pairs. The kernel records
 * one pair per macro step and only returns ticks strictly greater than
 * `sinceTick`, but we still de-duplicate defensively so overlapping reads
 * (e.g. single-step racing the playback loop) never bend the x axis backwards.
 */

import type { TimeMeta } from "@cetus/sim-client";

export interface SeriesBuffer {
  /** Display time (model unit) for every point. */
  x: number[];
  /** Display value (SI converted by the observable's factor) for every point. */
  y: number[];
  /** Highest tick already appended; keeps x strictly increasing. */
  lastTick: number;
}

export interface SeriesStore {
  [name: string]: SeriesBuffer;
}

export function createSeriesBuffer(): SeriesBuffer {
  return { x: [], y: [], lastTick: Number.NEGATIVE_INFINITY };
}

/** Display time of an integer kernel tick. */
export function tickToDisplayTime(tick: number, time: TimeMeta): number {
  return time.start + (tick - time.startTick) * time.dt;
}

/**
 * Appends the new points of one probe to its buffer.
 *
 * @returns the number of points actually appended.
 */
export function appendProbePoints(
  buffer: SeriesBuffer,
  points: Float64Array,
  time: TimeMeta,
  unitFactor: number,
  offset = 0,
): number {
  let appended = 0;
  for (let index = 0; index + 1 < points.length; index += 2) {
    const tick = points[index];
    const siValue = points[index + 1];
    if (tick === undefined || siValue === undefined) {
      continue;
    }
    if (!Number.isFinite(tick) || !Number.isFinite(siValue)) {
      continue;
    }
    if (tick <= buffer.lastTick) {
      continue;
    }
    buffer.lastTick = tick;
    buffer.x.push(tickToDisplayTime(tick, time));
    buffer.y.push((siValue - offset) / unitFactor);
    appended += 1;
  }
  return appended;
}

export function createSeriesStore(names: readonly string[]): SeriesStore {
  const store: SeriesStore = {};
  for (const name of names) {
    store[name] = createSeriesBuffer();
  }
  return store;
}

export interface SampledSeries {
  x: number[];
  ys: number[][];
}

/**
 * Stride-samples a set of aligned series down to `maxPoints` while keeping the
 * first and last samples. Used to keep the chart cheap on long runs; probes
 * themselves stay lossless.
 */
export function downsampleSeries(
  x: readonly number[],
  ys: readonly (readonly number[])[],
  maxPoints: number,
): SampledSeries {
  const limit = Math.max(2, Math.floor(maxPoints));
  if (x.length <= limit) {
    return { x: [...x], ys: ys.map((values) => [...values]) };
  }

  const step = (x.length - 1) / (limit - 1);
  const indexes: number[] = [];
  for (let index = 0; index < limit; index += 1) {
    const source = Math.round(index * step);
    if (indexes[indexes.length - 1] !== source) {
      indexes.push(source);
    }
  }

  const sampledX: number[] = [];
  const sampledYs: number[][] = ys.map(() => []);
  for (const source of indexes) {
    const value = x[source];
    if (value === undefined) {
      continue;
    }
    sampledX.push(value);
    ys.forEach((values, seriesIndex) => {
      const target = sampledYs[seriesIndex];
      if (target) {
        // NaN is uPlot's gap marker and keeps every y array aligned with x.
        target.push(sampledYValue(values, source) ?? Number.NaN);
      }
    });
  }

  return { x: sampledX, ys: sampledYs };
}

function sampledYValue(
  values: readonly number[],
  index: number,
): number | null {
  const value = values[index];
  return value === undefined ? null : value;
}
