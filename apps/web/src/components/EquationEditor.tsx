import { useEffect, useId, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type {
  Diagnostic,
  ModelMeta,
  ModelPatch,
  ParseResult,
} from "@cetus/sim-client";
import { useExpressionCheck } from "../hooks/useExpressionCheck";
import {
  effectiveMatch,
  patchFor,
  resolveTarget,
  targetGroups,
  targetKey,
} from "../lib/equation";
import type { CanvasTarget } from "../lib/layout";
import { EquationFeedback } from "./EquationFeedback";
import { MathFormulaInput } from "./MathFormulaInput";

interface EquationEditorProps {
  meta: ModelMeta;
  sourceYaml: string;
  target: CanvasTarget;
  disabled: boolean;
  parseExpression: (
    source: string,
    expectedUnit?: string,
  ) => Promise<ParseResult>;
  applyEdit: (
    patch: ModelPatch,
  ) => Promise<{ ok: boolean; diagnostics?: Diagnostic[] }>;
  onSelect: (target: CanvasTarget) => void;
}

export function EquationEditor({
  meta,
  sourceYaml,
  target,
  disabled,
  parseExpression,
  applyEdit,
  onSelect,
}: EquationEditorProps): React.JSX.Element {
  const { t } = useTranslation();
  const statusId = useId();
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);

  const info = useMemo(
    () => resolveTarget(meta, sourceYaml, target),
    [meta, sourceYaml, target],
  );
  const compiledText = info?.compiled ?? "";
  const compiledKey = `${meta.contentHash}:${targetKey(target)}:${compiledText}`;
  const expectedUnit = info?.unit ?? null;

  const [source, setSource] = useState(() => {
    const initial = resolveTarget(meta, sourceYaml, target);
    return initial?.compiled ?? "";
  });
  const { checking, parsed, clearCheck } = useExpressionCheck(
    source,
    expectedUnit,
    parseExpression,
  );
  const effective = effectiveMatch(parsed, target.kind);
  const [applying, setApplying] = useState(false);
  const [mathOpen, setMathOpen] = useState(false);
  const [notice, setNotice] = useState<"applied" | null>(null);
  const [applyFailed, setApplyFailed] = useState(false);
  const [failure, setFailure] = useState<Diagnostic[]>([]);

  useEffect(() => {
    setSource(compiledText);
  }, [compiledKey, compiledText]);

  useEffect(() => {
    setNotice(null);
    setApplyFailed(false);
    setFailure([]);
  }, [targetKey(target)]);

  const updateSource = (next: string): void => {
    if (next !== source) {
      clearCheck();
      setSource(next);
    }
    setNotice(null);
    setApplyFailed(false);
    setFailure([]);
  };

  const canApply =
    !disabled &&
    !applying &&
    !checking &&
    info !== null &&
    source.trim() !== "" &&
    effective !== null &&
    effective.ok &&
    effective.matchesExpected !== false;

  const canReset =
    !disabled && !applying && info?.compiled !== null && source !== compiledText;

  const handleApply = async (): Promise<void> => {
    if (!canApply) {
      return;
    }
    setApplying(true);
    setNotice(null);
    setApplyFailed(false);
    setFailure([]);
    const result = await applyEdit(patchFor(target, source.trim()));
    setApplying(false);
    if (result.ok) {
      setNotice("applied");
      setFailure([]);
    } else {
      setApplyFailed(true);
      setFailure(result.diagnostics ?? []);
    }
  };

  const groups = useMemo(() => targetGroups(meta), [meta]);

  if (info === null) {
    return (
      <section className="equation-editor" aria-labelledby="equation-title">
        <header className="equation-editor__head">
          <div>
            <p className="eyebrow">{t("editor.section")}</p>
            <h3 id="equation-title">{target.name}</h3>
          </div>
        </header>
        <p className="notice notice--error" role="alert">
          {t("editor.missing")}
        </p>
      </section>
    );
  }

  return (
    <section className="equation-editor" aria-labelledby="equation-title">
      <header className="equation-editor__head">
        <div>
          <p className="eyebrow">{t("editor.section")}</p>
          <h3 id="equation-title">
            {t(`editor.kind.${target.kind}`)} · {target.name}
          </h3>
          <p className="equation-editor__unit">
            {t("editor.unit", { unit: info.unit })}
          </p>
        </div>
        <label className="equation-editor__picker">
          <span className="eyebrow">{t("editor.target")}</span>
          <select
            value={targetKey(target)}
            disabled={disabled || applying}
            onChange={(event) => {
              const nextKey = event.target.value;
              for (const group of groups) {
                for (const name of group.names) {
                  if (targetKey({ kind: group.kind, name }) === nextKey) {
                    onSelect({ kind: group.kind, name });
                    return;
                  }
                }
              }
            }}
          >
            {groups.map((group) =>
              group.names.length === 0 ? null : (
                <optgroup
                  key={group.kind}
                  label={t(`editor.kind.${group.kind}`)}
                >
                  {group.names.map((name) => (
                    <option key={name} value={targetKey({ kind: group.kind, name })}>
                      {name}
                    </option>
                  ))}
                </optgroup>
              ),
            )}
          </select>
        </label>
      </header>

      <p className="equation-editor__explain">
        {t(`editor.explain.${target.kind}`)}
      </p>

      <label className="equation-editor__field">
        <span className="eyebrow">
          {target.kind === "stock" ? t("editor.initial") : t("editor.expression")}
        </span>
        <textarea
          ref={textareaRef}
          className="equation-editor__textarea"
          value={source}
          rows={3}
          spellCheck={false}
          disabled={disabled || applying}
          aria-describedby={statusId}
          placeholder={t("editor.placeholder")}
          onChange={(event) => {
            updateSource(event.target.value);
          }}
        />
      </label>

      <EquationFeedback
        id={statusId}
        source={source}
        checking={checking}
        parsed={effective}
        expectedUnit={info.unit}
      />

      <div className="equation-editor__actions">
        <button
          type="button"
          className="button button--primary"
          disabled={!canApply}
          onClick={() => {
            void handleApply();
          }}
        >
          {applying ? t("editor.applying") : t("editor.apply")}
        </button>
        <button
          type="button"
          className="button"
          disabled={!canReset}
          onClick={() => {
            updateSource(compiledText);
          }}
        >
          {t("editor.reset")}
        </button>
      </div>

      {notice === "applied" && (
        <p className="notice" role="status">
          {t("editor.applied")}
        </p>
      )}

      {applyFailed && (
        <div className="notice notice--error" role="alert">
          <p>{t("editor.failed")}</p>
          {failure.length > 0 && (
            <ul className="equation-editor__diagnostics">
              {failure.map((diagnostic, index) => (
                <li key={`${diagnostic.code}-${index}`}>
                  <code>{diagnostic.code}</code> {diagnostic.message}
                </li>
              ))}
            </ul>
          )}
        </div>
      )}

      <button
        type="button"
        className="text-button equation-editor__math-toggle"
        aria-expanded={mathOpen}
        onClick={() => {
          setMathOpen((open) => !open);
        }}
      >
        {mathOpen ? t("editor.mathHide") : t("editor.mathShow")}
      </button>

      {mathOpen && (
        <MathFormulaInput
          disabled={disabled || applying}
          onInsert={(ascii) => {
            updateSource(ascii);
            textareaRef.current?.focus();
          }}
        />
      )}
    </section>
  );
}
