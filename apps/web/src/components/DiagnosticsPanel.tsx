import { useTranslation } from "react-i18next";
import type { Diagnostic } from "@cetus/sim-client";
import { HashChip } from "./HashChip";

interface DiagnosticsPanelProps {
  diagnostics: readonly Diagnostic[];
  hash: string;
}

function locationLabel(diagnostic: Diagnostic): string | null {
  if (!diagnostic.path) {
    return null;
  }
  if (diagnostic.start === null) {
    return diagnostic.path;
  }
  const end = diagnostic.end === null ? diagnostic.start : diagnostic.end;
  return `${diagnostic.path}:${diagnostic.start}-${end}`;
}

export function DiagnosticsPanel({ diagnostics, hash }: DiagnosticsPanelProps) {
  const { t } = useTranslation();

  return (
    <section className="panel diagnostics" aria-labelledby="diagnostics-title">
      <header className="panel__head">
        <div>
          <p className="eyebrow">{t("diagnostics.section")}</p>
          <h2 id="diagnostics-title">{t("diagnostics.title")}</h2>
        </div>
        <span className="count-chip">
          {t("diagnostics.count", { count: diagnostics.length })}
        </span>
      </header>

      {diagnostics.length === 0 ? (
        <p className="panel__body">{t("diagnostics.empty")}</p>
      ) : (
        <ul className="diagnostic-list" aria-live="polite">
          {diagnostics.map((diagnostic, index) => {
            const location = locationLabel(diagnostic);
            return (
              <li
                key={`${diagnostic.code}-${index}`}
                className={`diagnostic diagnostic--${diagnostic.severity}`}
              >
                <div className="diagnostic__head">
                  <span className={`severity severity--${diagnostic.severity}`}>
                    {t(`diagnostics.${diagnostic.severity}`)}
                  </span>
                  <code>{diagnostic.code}</code>
                  {location && <span className="diagnostic__path">{location}</span>}
                </div>
                <p>{diagnostic.message}</p>
                {diagnostic.help && (
                  <p className="diagnostic__help">
                    <span>{t("diagnostics.help")}</span> {diagnostic.help}
                  </p>
                )}
              </li>
            );
          })}
        </ul>
      )}

      <div className="diagnostics__hash">
        <span className="eyebrow">{t("diagnostics.hash")}</span>
        <HashChip hash={hash} full />
      </div>
    </section>
  );
}
