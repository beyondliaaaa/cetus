import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { MathfieldElement } from "mathlive";
import { latexToAscii, type LatexResult } from "../lib/latex";

interface MathFormulaInputProps {
  disabled: boolean;
  onInsert: (ascii: string) => void;
}

export function MathFormulaInput({
  disabled,
  onInsert,
}: MathFormulaInputProps): React.JSX.Element {
  const { t } = useTranslation();
  const hostRef = useRef<HTMLDivElement | null>(null);
  const [preview, setPreview] = useState<LatexResult | null>(null);
  const [unavailable, setUnavailable] = useState(false);

  useEffect(() => {
    setPreview(null);
    setUnavailable(false);
    let disposed = false;
    let field: MathfieldElement | null = null;
    let detach: (() => void) | null = null;

    const mount = async (): Promise<void> => {
      try {
        await Promise.all([
          import("mathlive/static.css"),
          import("mathlive/fonts.css"),
          import("mathlive"),
        ]);
      } catch {
        if (!disposed) {
          setUnavailable(true);
        }
        return;
      }
      const host = hostRef.current;
      if (disposed || host === null) {
        return;
      }
      const element = document.createElement("math-field") as MathfieldElement;
      element.smartFence = true;
      element.mathVirtualKeyboardPolicy = "manual";
      element.placeholder = "\\frac{a}{b} + \\sin(t)";
      element.setAttribute("aria-label", t("editor.mathAria"));
      const onInput = (): void => {
        const latex = element.value;
        setPreview(latex.trim() === "" ? null : latexToAscii(latex));
      };
      element.addEventListener("input", onInput);
      host.appendChild(element);
      field = element;
      detach = () => {
        element.removeEventListener("input", onInput);
      };
    };

    void mount();
    return () => {
      disposed = true;
      detach?.();
      field?.remove();
    };
  }, [t]);

  const insert = (): void => {
    if (preview?.ok === true) {
      onInsert(preview.ascii);
    }
  };

  return (
    <div className="equation-editor__math">
      <p className="eyebrow">{t("editor.math")}</p>
      <p className="equation-editor__hint">{t("editor.mathHint")}</p>
      <div ref={hostRef} className="equation-editor__mathfield" />
      {unavailable && (
        <p className="notice notice--error">{t("editor.mathUnavailable")}</p>
      )}
      {preview?.ok === true && (
        <p className="equation-editor__preview">
          <span className="eyebrow">{t("editor.preview")}</span>
          <code>{preview.ascii}</code>
        </p>
      )}
      {preview?.ok === false && (
        <p className="notice notice--error" role="alert">
          {t(`editor.latex.${preview.code}`, {
            detail: preview.detail ?? "",
          })}
        </p>
      )}
      <button
        type="button"
        className="button"
        disabled={preview?.ok !== true || disabled}
        onClick={insert}
      >
        {t("editor.useFormula")}
      </button>
    </div>
  );
}
