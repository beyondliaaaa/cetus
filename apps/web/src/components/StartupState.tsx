import { useTranslation } from "react-i18next";

interface StartupStateProps {
  variant: "boot" | "session";
  done?: number;
  total?: number;
  modelName?: string;
}

/**
 * Calm loading surface: a determinate compile counter plus a skeleton plot, so
 * the workbench never shows a spinner on an otherwise blank screen.
 */
export function StartupState({
  variant,
  done = 0,
  total = 0,
  modelName,
}: StartupStateProps) {
  const { t } = useTranslation();
  const booting = variant === "boot";
  const percent = total > 0 ? Math.round((done / total) * 100) : 0;

  return (
    <section className="panel startup" aria-live="polite" aria-busy="true">
      <p className="eyebrow">{booting ? t("startup.kernel") : t("header.model")}</p>
      <h2>{booting ? t("startup.title") : t("startup.sessionTitle")}</h2>
      <p className="panel__body">
        {booting ? t("startup.body", { total }) : t("startup.sessionBody")}
      </p>

      {booting ? (
        <div className="startup__progress">
          <div
            className="progress"
            role="progressbar"
            aria-label={t("startup.title")}
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={percent}
          >
            <span style={{ width: `${percent}%` }} />
          </div>
          <span className="mono-note">{t("startup.progress", { done, total })}</span>
        </div>
      ) : (
        modelName && <p className="mono-note">{t("startup.model", { name: modelName })}</p>
      )}

      <div className="startup__skeleton" aria-hidden="true">
        <span />
        <span />
        <span />
        <span />
        <span />
      </div>
    </section>
  );
}
