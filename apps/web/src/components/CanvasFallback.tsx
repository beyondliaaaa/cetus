import { useTranslation } from "react-i18next";
import { sameTarget, type CanvasLayout, type CanvasTarget } from "../lib/layout";

interface CanvasFallbackProps {
  layout: CanvasLayout;
  selected: CanvasTarget | null;
  onSelect: (target: CanvasTarget) => void;
}

export function CanvasFallback({
  layout,
  selected,
  onSelect,
}: CanvasFallbackProps): React.JSX.Element {
  const { t } = useTranslation();

  return (
    <details className="canvas-fallback">
      <summary>{t("canvas.list")}</summary>
      <ul className="canvas-fallback__list">
        {layout.nodes
          .filter((node) => node.target !== null)
          .map((node) => (
            <li key={node.id}>
              <button
                type="button"
                className={
                  sameTarget(node.target, selected)
                    ? "text-button is-active"
                    : "text-button"
                }
                onClick={() => {
                  if (node.target !== null) {
                    onSelect(node.target);
                  }
                }}
              >
                {t(`canvas.kind.${node.kind}`)} · {node.name}
              </button>
            </li>
          ))}
        {layout.edges
          .filter((edge) => edge.kind === "flow")
          .map((edge) => (
            <li key={edge.id}>
              <button
                type="button"
                className={
                  sameTarget(edge.selection, selected)
                    ? "text-button is-active"
                    : "text-button"
                }
                onClick={() => {
                  if (edge.selection !== null) {
                    onSelect(edge.selection);
                  }
                }}
              >
                {t("canvas.flow")} · {edge.name}
              </button>
            </li>
          ))}
      </ul>
    </details>
  );
}
