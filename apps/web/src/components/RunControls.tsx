import { useTranslation } from "react-i18next";
import type { Frame, ModelMeta } from "@cetus/sim-client";
import { formatFixed, formatNumber, timeUnitKey } from "../lib/format";
import { HashChip } from "./HashChip";

interface RunControlsProps {
  meta: ModelMeta;
  frame: Frame;
  playing: boolean;
  speed: number;
  onTogglePlay: () => void;
  onStep: () => void;
  onReset: () => void;
  onSpeedChange: (speed: number) => void;
}

const SPEEDS = [1, 10, 60, 300] as const;

function PlayIcon() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <path d="M8 5.5v13l11-6.5z" fill="currentColor" />
    </svg>
  );
}

function PauseIcon() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <path d="M7 5h3.5v14H7zM13.5 5H17v14h-3.5z" fill="currentColor" />
    </svg>
  );
}

function StepIcon() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <path d="M6 5.5v13l9-6.5zM17 5h2v14h-2z" fill="currentColor" />
    </svg>
  );
}

function ResetIcon() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <path
        d="M5 12a7 7 0 1 0 2.2-5.1M5 4v4h4"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function RunControls({
  meta,
  frame,
  playing,
  speed,
  onTogglePlay,
  onStep,
  onReset,
  onSpeedChange,
}: RunControlsProps) {
  const { t, i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? i18n.language;
  const unitKey = timeUnitKey(meta.time.unit);
  const unitLabel = unitKey ? t(unitKey) : meta.time.unit;
  const horizon = meta.time.horizon;
  const percent =
    horizon !== null && horizon > meta.time.start
      ? Math.min(
          100,
          Math.max(
            0,
            ((frame.displayTime - meta.time.start) / (horizon - meta.time.start)) * 100,
          ),
        )
      : 0;
  const status = frame.done ? t("run.done") : playing ? t("run.live") : t("run.paused");
  const readout =
    horizon === null
      ? t("run.timeReadoutOpen", {
          time: formatFixed(frame.displayTime, locale),
          timeUnit: unitLabel,
        })
      : t("run.timeReadout", {
          time: formatFixed(frame.displayTime, locale),
          timeUnit: unitLabel,
          horizon: formatNumber(horizon, locale),
          horizonUnit: unitLabel,
        });

  return (
    <section className="runbar" aria-label={t("run.section")}>
      <div className="runbar__transport">
        <button
          type="button"
          className="transport transport--primary"
          onClick={onTogglePlay}
          disabled={frame.done}
          aria-label={playing ? t("run.pause") : t("run.play")}
        >
          {playing ? <PauseIcon /> : <PlayIcon />}
          <span>{playing ? t("run.pause") : t("run.play")}</span>
        </button>
        <button
          type="button"
          className="transport"
          onClick={onStep}
          disabled={frame.done}
          aria-label={t("run.step")}
        >
          <StepIcon />
          <span>{t("run.step")}</span>
        </button>
        <button
          type="button"
          className="transport"
          onClick={onReset}
          aria-label={t("run.reset")}
        >
          <ResetIcon />
          <span>{t("run.reset")}</span>
        </button>
      </div>

      <div className="runbar__clock">
        <div className="runbar__clock-head">
          <span className="eyebrow">{t("run.time")}</span>
          <span className={frame.done ? "run-status run-status--done" : "run-status"} aria-live="polite">
            {status}
          </span>
        </div>
        <strong className="time-readout">{readout}</strong>
        <div
          className="progress progress--slim"
          role="progressbar"
          aria-label={t("run.progress")}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={Math.round(percent)}
        >
          <span style={{ width: `${percent}%` }} />
        </div>
      </div>

      <div className="runbar__speed" role="group" aria-label={t("run.speed")}>
        <span className="eyebrow">{t("run.speed")}</span>
        <div className="segmented">
          {SPEEDS.map((option) => (
            <button
              key={option}
              type="button"
              className={option === speed ? "segmented__item is-active" : "segmented__item"}
              aria-pressed={option === speed}
              aria-label={t("run.speedAria", { value: option })}
              onClick={() => {
                onSpeedChange(option);
              }}
            >
              {t("run.speedOption", { value: option })}
            </button>
          ))}
        </div>
      </div>

      <div className="runbar__hash">
        <span className="eyebrow">{t("run.hash")}</span>
        <HashChip hash={frame.hash} />
      </div>
    </section>
  );
}
