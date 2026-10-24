/**
 * Pure chart-shaping helpers: unit grouping for axes and aligned, downsampled
 * uPlot data. No React and no DOM, so the logic is easy to reason about.
 */

import type uPlot from "uplot";
import type { ModelMeta } from "@cetus/sim-client";
import { downsampleSeries, type SeriesStore } from "./series";

export interface UnitGroup {
  scale: string;
  unit: string;
  indexes: number[];
}

const MAX_POINTS = 2400;

export function unitGroups(meta: ModelMeta): UnitGroup[] {
  const groups = new Map<string, UnitGroup>();
  meta.observables.forEach((observable, index) => {
    const existing = groups.get(observable.unit);
    if (existing) {
      existing.indexes.push(index);
    } else {
      groups.set(observable.unit, {
        scale: "",
        unit: observable.unit,
        indexes: [index],
      });
    }
  });
  return [...groups.values()].map((group, position) => ({
    ...group,
    scale: position === 0 ? "y" : `y${position + 1}`,
  }));
}

/** Scale key per observable; the third and later unit groups share no axis. */
export function scaleByObservable(
  meta: ModelMeta,
  groups: readonly UnitGroup[],
): string[] {
  const scales: string[] = meta.observables.map(() => "y");
  for (const group of groups) {
    for (const index of group.indexes) {
      scales[index] = group.scale;
    }
  }
  return scales;
}

export function buildChartData(
  meta: ModelMeta,
  store: SeriesStore,
): uPlot.AlignedData {
  const buffers = meta.observables.map((observable) => store[observable.name]);
  const x = buffers[0]?.x ?? [];
  const ys = buffers.map((buffer) => {
    if (buffer && buffer.x.length === x.length) {
      return buffer.y;
    }
    return Array.from({ length: x.length }, () => Number.NaN);
  });
  const sampled = downsampleSeries(x, ys, MAX_POINTS);
  return [sampled.x, ...sampled.ys];
}
