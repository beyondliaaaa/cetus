import { useTranslation } from "react-i18next";
import type { ModelMeta } from "@cetus/sim-client";
import { formatNumber } from "../lib/format";
import { ParameterCard } from "./ParameterCard";

interface ParameterRailProps {
  meta: ModelMeta;
  values: Record<string, number>;
  disabled: boolean;
  onCommit: (name: string, value: number, unit: string) => void;
}

export function ParameterRail({
  meta,
  values,
  disabled,
  onCommit,
}: ParameterRailProps) {
  const { t, i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? i18n.language;
  const controls = meta.parameters.filter((parameter) => parameter.role !== "fixed");
  const fixed = meta.parameters.filter((parameter) => parameter.role === "fixed");

  return (
    <section className="panel params" aria-labelledby="params-title">
      <header className="panel__head">
        <div>
          <p className="eyebrow">{t("param.section")}</p>
          <h2 id="params-title">{t("param.section")}</h2>
        </div>
        <span className="count-chip">{meta.parameters.length}</span>
      </header>
      <p className="panel__body">{t("param.controlHint")}</p>

      {controls.length === 0 ? (
        <p className="mono-note">{t("param.none")}</p>
      ) : (
        <div className="param-list">
          {controls.map((parameter) => (
            <ParameterCard
              key={parameter.name}
              parameter={parameter}
              value={values[parameter.name] ?? parameter.value}
              disabled={disabled}
              onCommit={onCommit}
            />
          ))}
        </div>
      )}

      {fixed.length > 0 && (
        <details className="fixed-params">
          <summary>
            {t("param.fixed", { count: fixed.length })}
          </summary>
          <ul>
            {fixed.map((parameter) => (
              <li key={parameter.name}>
                <div className="fixed-params__main">
                  <code>{parameter.name}</code>
                  <span className="role-chip">{t(`param.role.${parameter.role}`)}</span>
                </div>
                <div className="fixed-params__value">
                  <span>
                    {formatNumber(parameter.value, locale)} {parameter.unit}
                  </span>
                  <span className="fixed-params__source">
                    {t("param.source")}: {parameter.source ?? t("param.noSource")}
                  </span>
                </div>
              </li>
            ))}
          </ul>
        </details>
      )}
    </section>
  );
}
