import { useTranslation } from "react-i18next";
import { Handle, Position, type Node, type NodeProps, type NodeTypes } from "@xyflow/react";
import type { CanvasTarget } from "../lib/layout";
import { formatNumber } from "../lib/format";

export interface CanvasNodeData extends Record<string, unknown> {
  kind: "stock" | "auxiliary" | "boundary";
  name: string;
  unit: string | null;
  value: number | null;
  initial: number | null;
  bounds: { min: number | null; max: number | null } | null;
  target: CanvasTarget | null;
}

export type CanvasNode = Node<
  CanvasNodeData,
  "stock" | "auxiliary" | "boundary"
>;

type CanvasNodeProps = NodeProps<CanvasNode>;

function BoundaryHandles(): React.JSX.Element {
  return (
    <>
      <Handle
        id="target-left"
        type="target"
        position={Position.Left}
        className="canvas-handle"
        isConnectable={false}
      />
      <Handle
        id="source-right"
        type="source"
        position={Position.Right}
        className="canvas-handle"
        isConnectable={false}
      />
    </>
  );
}

function NodeHandles(): React.JSX.Element {
  return (
    <>
      <Handle
        id="target-left"
        type="target"
        position={Position.Left}
        className="canvas-handle"
        isConnectable={false}
      />
      <Handle
        id="target-top"
        type="target"
        position={Position.Top}
        className="canvas-handle"
        isConnectable={false}
      />
      <Handle
        id="target-bottom"
        type="target"
        position={Position.Bottom}
        className="canvas-handle"
        isConnectable={false}
      />
      <Handle
        id="source-right"
        type="source"
        position={Position.Right}
        className="canvas-handle"
        isConnectable={false}
      />
      <Handle
        id="source-top"
        type="source"
        position={Position.Top}
        className="canvas-handle"
        isConnectable={false}
      />
      <Handle
        id="source-bottom"
        type="source"
        position={Position.Bottom}
        className="canvas-handle"
        isConnectable={false}
      />
    </>
  );
}

function StockNode({ data, selected }: CanvasNodeProps): React.JSX.Element {
  const { t, i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? i18n.language;
  const value = data.value === null ? "—" : formatNumber(data.value, locale);
  const bounds = data.bounds;

  return (
    <div
      className={`canvas-node canvas-node--stock${selected ? " is-selected" : ""}`}
    >
      <NodeHandles />
      <span className="canvas-node__kind">{t("canvas.stock")}</span>
      <strong className="canvas-node__name">{data.name}</strong>
      <span className="canvas-node__value">
        {value}
        {data.unit !== null && <code>{data.unit}</code>}
      </span>
      <span className="canvas-node__meta">
        {data.initial !== null &&
          t("canvas.initial", {
            value: formatNumber(data.initial, locale),
            unit: data.unit ?? "",
          })}
        {bounds !== null && (bounds.min !== null || bounds.max !== null) && (
          <span>
            {t("canvas.bounds", {
              min: bounds.min === null ? "−∞" : formatNumber(bounds.min, locale),
              max: bounds.max === null ? "∞" : formatNumber(bounds.max, locale),
              unit: data.unit ?? "",
            })}
          </span>
        )}
      </span>
    </div>
  );
}

function AuxiliaryNode({ data, selected }: CanvasNodeProps): React.JSX.Element {
  const { t, i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? i18n.language;
  const value = data.value === null ? "—" : formatNumber(data.value, locale);

  return (
    <div
      className={`canvas-node canvas-node--auxiliary${selected ? " is-selected" : ""}`}
    >
      <NodeHandles />
      <span className="canvas-node__kind">{t("canvas.auxiliary")}</span>
      <strong className="canvas-node__name">{data.name}</strong>
      <span className="canvas-node__value">
        {value}
        {data.unit !== null && <code>{data.unit}</code>}
      </span>
    </div>
  );
}

function BoundaryNode({ selected }: CanvasNodeProps): React.JSX.Element {
  const { t } = useTranslation();

  return (
    <div
      className={`canvas-node canvas-node--boundary${selected ? " is-selected" : ""}`}
    >
      <BoundaryHandles />
      <span>{t("canvas.outside")}</span>
    </div>
  );
}

export const NODE_TYPES = {
  stock: StockNode,
  auxiliary: AuxiliaryNode,
  boundary: BoundaryNode,
} satisfies NodeTypes;
