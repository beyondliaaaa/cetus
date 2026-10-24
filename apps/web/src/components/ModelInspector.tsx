import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type {
  Diagnostic,
  ModelMeta,
  ModelPatch,
  ParseResult,
} from "@cetus/sim-client";
import { defaultTarget, editableTargets } from "../lib/equation";
import type { CanvasTarget } from "../lib/layout";
import { sameTarget } from "../lib/layout";
import { EquationEditor } from "./EquationEditor";
import { ModelCanvas } from "./ModelCanvas";

interface ModelInspectorProps {
  meta: ModelMeta;
  sourceYaml: string;
  disabled: boolean;
  parseExpression: (
    source: string,
    expectedUnit?: string,
  ) => Promise<ParseResult>;
  queryValue: (name: string) => Promise<number | null>;
  applyEdit: (
    patch: ModelPatch,
  ) => Promise<{ ok: boolean; diagnostics?: Diagnostic[] }>;
}

export function ModelInspector({
  meta,
  sourceYaml,
  disabled,
  parseExpression,
  queryValue,
  applyEdit,
}: ModelInspectorProps): React.JSX.Element {
  const { t } = useTranslation();
  const [selected, setSelected] = useState<CanvasTarget | null>(() =>
    defaultTarget(meta),
  );
  const [canvasOpen, setCanvasOpen] = useState(true);

  const handleSelect = useCallback((target: CanvasTarget): void => {
    setSelected((current) => (sameTarget(current, target) ? current : target));
  }, []);

  useEffect(() => {
    const targets = editableTargets(meta);
    setSelected((current) => {
      if (current !== null && targets.some((target) => sameTarget(target, current))) {
        return current;
      }
      return targets[0] ?? null;
    });
  }, [meta]);

  useEffect(() => {
    if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
      return;
    }
    const query = window.matchMedia("(max-width: 900px)");
    const apply = (): void => {
      setCanvasOpen(!query.matches);
    };
    apply();
    query.addEventListener("change", apply);
    return () => {
      query.removeEventListener("change", apply);
    };
  }, []);

  const targets = editableTargets(meta);

  return (
    <section className="panel inspector" aria-labelledby="inspector-title">
      <header className="panel__head">
        <div>
          <p className="eyebrow">{t("inspector.title")}</p>
          <h2 id="inspector-title">{t("inspector.title")}</h2>
        </div>
        <span className="count-chip">{t("canvas.count", { count: targets.length })}</span>
      </header>
      <p className="panel__body">{t("inspector.body")}</p>

      <div className="inspector__grid">
        <div className="inspector__canvas">
          <div className="inspector__canvas-head">
            <h3>{t("canvas.title")}</h3>
            <button
              type="button"
              className="text-button inspector__canvas-toggle"
              aria-expanded={canvasOpen}
              aria-controls="model-canvas"
              onClick={() => {
                setCanvasOpen((open) => !open);
              }}
            >
              {canvasOpen ? t("canvas.collapse") : t("canvas.expand")}
            </button>
          </div>
          <div
            id="model-canvas"
            className={
              canvasOpen
                ? "inspector__canvas-body"
                : "inspector__canvas-body is-collapsed"
            }
          >
            <ModelCanvas
              meta={meta}
              selected={selected}
              onSelect={handleSelect}
              queryValue={queryValue}
            />
          </div>
        </div>

        {selected === null ? (
          <p className="notice">{t("inspector.noTargets")}</p>
        ) : (
          <EquationEditor
            meta={meta}
            sourceYaml={sourceYaml}
            target={selected}
            disabled={disabled}
            parseExpression={parseExpression}
            applyEdit={applyEdit}
            onSelect={handleSelect}
          />
        )}
      </div>
    </section>
  );
}
