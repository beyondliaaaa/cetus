import "uplot/dist/uPlot.min.css";
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import uPlot from "uplot";
import type { Frame, ModelMeta } from "@cetus/sim-client";
import {
  buildChartData,
  scaleByObservable,
  unitGroups,
} from "../lib/chart";
import { formatNumber, timeUnitKey } from "../lib/format";
import type { SeriesStore } from "../lib/series";
import { LatestValues } from "./LatestValues";

interface ChartPanelProps {
  meta: ModelMeta;
  frame: Frame;
  revision: number;
  seriesRef: { current: SeriesStore };
}

interface PaletteToken {
  token: string;
  fallback: string;
}

const GRID_COLOR = "rgba(122, 162, 255, 0.12)";
const AXIS_FONT = '10px "JetBrains Mono", ui-monospace, monospace';
const LABEL_FONT = '10px "JetBrains Mono", ui-monospace, monospace';

const PALETTE: readonly PaletteToken[] = [
  { token: "--accent", fallback: "#7fd4ff" },
  { token: "--accent-warm", fallback: "#ffd9a0" },
  { token: "--chart-3", fallback: "#9ef0c0" },
  { token: "--chart-4", fallback: "#c9a7ff" },
  { token: "--chart-5", fallback: "#ff9b9b" },
  { token: "--chart-6", fallback: "#ffe28a" },
];

function readToken(element: HTMLElement, token: string, fallback: string): string {
  const value = getComputedStyle(element).getPropertyValue(token).trim();
  return value.length > 0 ? value : fallback;
}

export function ChartPanel({ meta, frame, revision, seriesRef }: ChartPanelProps) {
  const { t, i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? i18n.language;
  const containerRef = useRef<HTMLDivElement | null>(null);
  const chartRef = useRef<uPlot | null>(null);
  const [hidden, setHidden] = useState<ReadonlySet<string>>(() => new Set());

  const { groups, scales } = useMemo(() => {
    const nextGroups = unitGroups(meta);
    return { groups: nextGroups, scales: scaleByObservable(meta, nextGroups) };
  }, [meta]);
  const unitKey = timeUnitKey(meta.time.unit);
  const unitLabel = unitKey ? t(unitKey) : meta.time.unit;
  const hasData = meta.observables.some(
    (observable) => (seriesRef.current[observable.name]?.x.length ?? 0) > 0,
  );

  useEffect(() => {
    setHidden(new Set());
  }, [meta]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) {
      return;
    }
    const colors = PALETTE.map((entry) => readToken(container, entry.token, entry.fallback));
    const width = Math.max(260, Math.floor(container.clientWidth));
    const height = Math.max(220, Math.floor(container.clientHeight));
    const textColor = readToken(container, "--ink-dim", "#9aa8c7");
    const accent = readToken(container, "--accent", "#7fd4ff");

    const chartScales: uPlot.Scales = { x: { time: false } };
    for (const group of groups) {
      chartScales[group.scale] = {};
    }

    const axes: uPlot.Axis[] = [
      {
        side: 2,
        stroke: textColor,
        font: AXIS_FONT,
        label: i18n.t("chart.axisTime", { unit: unitLabel }),
        labelFont: LABEL_FONT,
        labelSize: 18,
        grid: { stroke: GRID_COLOR, width: 1 },
        ticks: { stroke: GRID_COLOR, width: 1, size: 4 },
        values: (_self, splits) => splits.map((value) => formatNumber(value, locale, 3)),
      },
    ];
    groups.slice(0, 2).forEach((group, index) => {
      axes.push({
        scale: group.scale,
        side: index === 0 ? 3 : 1,
        stroke: index === 0 ? accent : colors[1] ?? accent,
        font: AXIS_FONT,
        label: group.unit === "1" ? i18n.t("chart.unitless") : group.unit,
        labelFont: LABEL_FONT,
        labelSize: 20,
        grid: { stroke: GRID_COLOR, width: 1 },
        ticks: { stroke: GRID_COLOR, width: 1, size: 4 },
        values: (_self, splits) => splits.map((value) => formatNumber(value, locale, 3)),
      });
    });

    const series: uPlot.Series[] = [
      { label: i18n.t("run.time") },
      ...meta.observables.map((observable, index) => ({
        label:
          observable.unit === "1"
            ? observable.name
            : `${observable.name} (${observable.unit})`,
        scale: scales[index] ?? "y",
        stroke: colors[index % colors.length] ?? accent,
        width: 1.9,
        spanGaps: true,
        points: { show: false },
      })),
    ];

    const chart = new uPlot(
      {
        width,
        height,
        padding: [10, 12, 0, 0],
        legend: { show: false },
        cursor: { drag: { x: true, y: false, setScale: true } },
        scales: chartScales,
        axes,
        series,
      },
      buildChartData(meta, seriesRef.current),
      container,
    );
    chartRef.current = chart;

    const observer = new ResizeObserver((entries) => {
      const rect = entries[0]?.contentRect;
      if (!rect) {
        return;
      }
      chart.setSize({
        width: Math.max(260, Math.floor(rect.width)),
        height: Math.max(220, Math.floor(rect.height)),
      });
    });
    observer.observe(container);

    return () => {
      observer.disconnect();
      chart.destroy();
      chartRef.current = null;
    };
  }, [groups, i18n, locale, meta, scales, seriesRef, unitLabel]);

  useEffect(() => {
    const chart = chartRef.current;
    if (!chart) {
      return;
    }
    chart.setData(buildChartData(meta, seriesRef.current));
  }, [meta, revision, seriesRef]);

  useEffect(() => {
    const chart = chartRef.current;
    if (!chart) {
      return;
    }
    meta.observables.forEach((observable, index) => {
      chart.setSeries(index + 1, { show: !hidden.has(observable.name) });
    });
  }, [hidden, meta]);

  const toggle = (name: string): void => {
    const next = new Set(hidden);
    if (next.has(name)) {
      next.delete(name);
    } else {
      next.add(name);
    }
    setHidden(next);
    const index = meta.observables.findIndex((observable) => observable.name === name);
    if (index >= 0) {
      chartRef.current?.setSeries(index + 1, { show: !next.has(name) });
    }
  };

  const colors = PALETTE.map((entry) => entry.fallback);

  return (
    <section className="panel chart-panel" aria-labelledby="chart-title">
      <header className="panel__head">
        <div>
          <p className="eyebrow">{t("chart.section")}</p>
          <h2 id="chart-title">{t("chart.title")}</h2>
        </div>
        <span className="count-chip">{meta.observables.length}</span>
      </header>

      <div className="chart-legend" role="group" aria-label={t("chart.legend")}>
        {meta.observables.map((observable, index) => {
          const active = !hidden.has(observable.name);
          return (
            <button
              key={observable.name}
              type="button"
              className={active ? "legend-chip is-active" : "legend-chip"}
              aria-pressed={active}
              aria-label={t("chart.toggle", { name: observable.name })}
              onClick={() => {
                toggle(observable.name);
              }}
            >
              <span
                className="legend-chip__swatch"
                style={{ backgroundColor: colors[index % colors.length] ?? "#7fd4ff" }}
                aria-hidden="true"
              />
              <span>{observable.name}</span>
              {observable.unit !== "1" && <code>{observable.unit}</code>}
            </button>
          );
        })}
      </div>

      <div className="chart-shell">
        <div
          ref={containerRef}
          className="chart-host"
          aria-hidden="true"
          data-extension-point="uplot-series"
        />
        {!hasData && <p className="chart-empty">{t("chart.empty")}</p>}
      </div>

      <LatestValues meta={meta} frame={frame} />
    </section>
  );
}
