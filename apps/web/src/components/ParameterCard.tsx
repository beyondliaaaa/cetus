import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { ParameterMeta } from "@cetus/sim-client";
import { formatInputNumber, formatNumber } from "../lib/format";

interface ParameterCardProps {
  parameter: ParameterMeta;
  value: number;
  disabled: boolean;
  onCommit: (name: string, value: number, unit: string) => void;
}

const COMMIT_DELAY_MS = 120;

export function sliderStep(range: readonly [number, number]): number {
  const span = Math.abs(range[1] - range[0]);
  if (!(span > 0)) {
    return 1;
  }
  const rough = span / 100;
  const magnitude = 10 ** Math.floor(Math.log10(rough));
  const normalized = rough / magnitude;
  const nice = normalized <= 1 ? 1 : normalized <= 2 ? 2 : normalized <= 5 ? 5 : 10;
  return nice * magnitude;
}

export function ParameterCard({
  parameter,
  value,
  disabled,
  onCommit,
}: ParameterCardProps) {
  const { t, i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? i18n.language;
  const [draft, setDraft] = useState(value);
  const [text, setText] = useState(() => formatInputNumber(value));
  const timer = useRef<number | null>(null);

  useEffect(() => {
    setDraft(value);
    setText(formatInputNumber(value));
  }, [value, parameter.name]);

  useEffect(
    () => () => {
      if (timer.current !== null) {
        window.clearTimeout(timer.current);
      }
    },
    [],
  );

  const commitNow = (next: number): void => {
    if (timer.current !== null) {
      window.clearTimeout(timer.current);
      timer.current = null;
    }
    setDraft(next);
    setText(formatInputNumber(next));
    onCommit(parameter.name, next, parameter.unit);
  };

  const scheduleCommit = (next: number): void => {
    setDraft(next);
    setText(formatInputNumber(next));
    if (timer.current !== null) {
      window.clearTimeout(timer.current);
    }
    timer.current = window.setTimeout(() => {
      timer.current = null;
      onCommit(parameter.name, next, parameter.unit);
    }, COMMIT_DELAY_MS);
  };

  const range = parameter.range;
  const step = range ? sliderStep(range) : 0;
  const sliderValue = range
    ? Math.min(Math.max(draft, range[0]), range[1])
    : draft;

  return (
    <article className="param-card">
      <div className="param-card__head">
        <div>
          <h3>{parameter.name}</h3>
          <span className="role-chip">{t(`param.role.${parameter.role}`)}</span>
        </div>
        <button
          type="button"
          className="text-button"
          disabled={disabled}
          aria-label={t("param.resetAria", { name: parameter.name })}
          onClick={() => {
            commitNow(parameter.value);
          }}
        >
          {t("param.reset")}
        </button>
      </div>

      <div className="param-card__control">
        {range && (
          <input
            className="slider"
            type="range"
            min={range[0]}
            max={range[1]}
            step={step}
            value={sliderValue}
            disabled={disabled}
            aria-label={t("param.sliderAria", { name: parameter.name })}
            onChange={(event) => {
              scheduleCommit(Number(event.target.value));
            }}
          />
        )}
        <label className="num-field">
          <input
            type="number"
            inputMode="decimal"
            step={range ? step : "any"}
            min={range ? range[0] : undefined}
            max={range ? range[1] : undefined}
            value={text}
            disabled={disabled}
            aria-label={t("param.numberAria", {
              name: parameter.name,
              unit: parameter.unit,
            })}
            onChange={(event) => {
              const nextText = event.target.value;
              setText(nextText);
              if (!disabled && nextText.trim() !== "") {
                const parsed = Number(nextText);
                if (Number.isFinite(parsed)) {
                  setDraft(parsed);
                }
              }
            }}
            onBlur={() => {
              if (disabled) {
                return;
              }
              const parsed = Number(text);
              if (text.trim() !== "" && Number.isFinite(parsed)) {
                commitNow(parsed);
              } else {
                setText(formatInputNumber(draft));
              }
            }}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.currentTarget.blur();
              }
            }}
          />
          <span className="num-field__unit">{parameter.unit}</span>
        </label>
      </div>

      <div className="param-card__meta">
        {range && (
          <span>
            {t("param.range", {
              min: formatNumber(range[0], locale),
              max: formatNumber(range[1], locale),
            })}
          </span>
        )}
        <span>
          {t("param.default", {
            value: formatNumber(parameter.value, locale),
          })}
        </span>
      </div>
    </article>
  );
}
