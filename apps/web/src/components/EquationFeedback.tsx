import { useTranslation } from "react-i18next";
import type { ParseResult } from "@cetus/sim-client";

interface EquationFeedbackProps {
  id: string;
  source: string;
  checking: boolean;
  parsed: ParseResult | null;
  expectedUnit: string;
}

interface CaretPreviewProps {
  source: string;
  start: number;
  end: number;
}

function CaretPreview({ source, start, end }: CaretPreviewProps): React.JSX.Element {
  const safeStart = Math.max(0, Math.min(start, source.length));
  const safeEnd = Math.max(safeStart, Math.min(end, source.length));
  const width = Math.max(1, safeEnd - safeStart);
  return (
    <pre className="equation-editor__caret" aria-hidden="true">
      <span>{source.slice(0, safeStart)}</span>
      <mark>{source.slice(safeStart, safeEnd) || " "}</mark>
      <span>{source.slice(safeEnd)}</span>
      {"\n"}
      <span>{" ".repeat(safeStart)}</span>
      <span className="equation-editor__caret-mark">{"^".repeat(width)}</span>
    </pre>
  );
}

export function EquationFeedback({
  id,
  source,
  checking,
  parsed,
  expectedUnit,
}: EquationFeedbackProps): React.JSX.Element {
  const { t } = useTranslation();
  const unitlessAsTarget =
    parsed?.ok === true &&
    parsed.dim === "1" &&
    expectedUnit !== "1" &&
    parsed.matchesExpected === true;

  return (
    <div id={id} className="equation-editor__status" aria-live="polite">
      {checking && (
        <p className="equation-editor__checking">{t("editor.checking")}</p>
      )}
      {!checking && parsed?.ok === true && unitlessAsTarget && (
        <p className="notice">
          {t("editor.unitlessInitial", { unit: expectedUnit })}
        </p>
      )}
      {!checking && parsed?.ok === true && !unitlessAsTarget && (
        <>
          <div className="equation-editor__dimensions">
            <span className="chip">
              {t("editor.dim", { dim: parsed.dim ?? "1" })}
            </span>
            {parsed.unit !== null && (
              <span className="chip">
                {t("editor.suggestedUnit", { unit: parsed.unit })}
              </span>
            )}
          </div>
          {parsed.matchesExpected === false && (
            <p className="notice notice--error">
              {t("editor.mismatch", {
                found: parsed.unit ?? parsed.dim ?? "1",
                expected: parsed.expectedUnit ?? expectedUnit,
              })}
            </p>
          )}
          {parsed.matchesExpected === true && (
            <p className="notice">{t("editor.matches")}</p>
          )}
        </>
      )}
      {!checking && parsed?.ok === false && (
        <div className="equation-editor__error" role="alert">
          <p>
            <code>{parsed.code}</code> {parsed.message}
          </p>
          <CaretPreview source={source} start={parsed.start} end={parsed.end} />
        </div>
      )}
      {!checking && parsed === null && source.trim() === "" && (
        <p className="equation-editor__hint">{t("editor.empty")}</p>
      )}
    </div>
  );
}
