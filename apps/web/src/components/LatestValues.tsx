import { useTranslation } from "react-i18next";
import type { Frame, ModelMeta } from "@cetus/sim-client";
import { formatNumber } from "../lib/format";
import { toDisplayValue } from "../lib/units";

interface LatestValuesProps {
  meta: ModelMeta;
  frame: Frame;
}

export function LatestValues({ meta, frame }: LatestValuesProps) {
  const { t, i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? i18n.language;

  return (
    <details className="latest-values">
      <summary>{t("latest.title", { count: meta.observables.length })}</summary>
      <div className="table-scroll">
        <table>
          <caption className="visually-hidden">{t("latest.caption")}</caption>
          <thead>
            <tr>
              <th scope="col">{t("latest.observable")}</th>
              <th scope="col">{t("latest.value")}</th>
              <th scope="col">{t("latest.unit")}</th>
            </tr>
          </thead>
          <tbody>
            {meta.observables.map((observable) => {
              const siValue = frame.observables[observable.name] ?? 0;
              const display = toDisplayValue(siValue, observable.unitFactor, 0);
              return (
                <tr key={observable.name}>
                  <th scope="row">
                    <code>{observable.name}</code>
                  </th>
                  <td className="numeric">{formatNumber(display, locale, 5)}</td>
                  <td>
                    <code>{observable.unit}</code>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </details>
  );
}
