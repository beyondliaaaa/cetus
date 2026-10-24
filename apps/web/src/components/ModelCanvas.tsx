import "@xyflow/react/dist/style.css";
import { useCallback, useMemo } from "react";
import type { MouseEvent as ReactMouseEvent } from "react";
import { useTranslation } from "react-i18next";
import {
  Background,
  BackgroundVariant,
  Controls,
  MarkerType,
  ReactFlow,
  type Edge,
} from "@xyflow/react";
import type { ModelMeta } from "@cetus/sim-client";
import { useCanvasValues } from "../hooks/useCanvasValues";
import {
  buildCanvasLayout,
  sameTarget,
  type CanvasTarget,
} from "../lib/layout";
import { CanvasFallback } from "./CanvasFallback";
import { NODE_TYPES, type CanvasNode } from "./CanvasNodes";

interface ModelCanvasProps {
  meta: ModelMeta;
  selected: CanvasTarget | null;
  onSelect: (target: CanvasTarget) => void;
  queryValue: (name: string) => Promise<number | null>;
}

export function ModelCanvas({
  meta,
  selected,
  onSelect,
  queryValue,
}: ModelCanvasProps): React.JSX.Element {
  const { t } = useTranslation();
  const layout = useMemo(() => buildCanvasLayout(meta), [meta]);
  const queryNames = useMemo(
    () =>
      layout.nodes
        .filter((node) => node.kind !== "boundary")
        .map((node) => node.name),
    [layout],
  );
  const values = useCanvasValues(queryNames, queryValue);

  const nodes = useMemo<CanvasNode[]>(
    () =>
      layout.nodes.map((spec) => {
        const isBoundary = spec.kind === "boundary";
        return {
          id: spec.id,
          type: spec.kind,
          position: spec.position,
          selected: sameTarget(spec.target, selected),
          selectable: spec.target !== null,
          draggable: false,
          connectable: false,
          deletable: false,
          data: {
            kind: spec.kind,
            name: spec.name,
            unit: spec.unit,
            value: isBoundary ? null : (values[spec.name] ?? null),
            initial: spec.initial,
            bounds: spec.bounds,
            target: spec.target,
          },
          ariaLabel: isBoundary
            ? t("canvas.outside")
            : t("canvas.nodeAria", {
                kind: t(`canvas.kind.${spec.kind}`),
                name: spec.name,
              }),
        };
      }),
    [layout, selected, t, values],
  );

  const nodeById = useMemo(
    () => new Map(layout.nodes.map((node) => [node.id, node] as const)),
    [layout],
  );

  const edges = useMemo<Edge[]>(
    () =>
      layout.edges.map((spec) => {
        const isFlow = spec.kind === "flow";
        const sourceNode = nodeById.get(spec.source);
        const targetNode = nodeById.get(spec.target);
        const vertical =
          sourceNode !== undefined &&
          targetNode !== undefined &&
          sourceNode.position.x === targetNode.position.x;
        const downward =
          sourceNode !== undefined &&
          targetNode !== undefined &&
          sourceNode.position.y <= targetNode.position.y;
        const base: Edge = {
          id: spec.id,
          source: spec.source,
          target: spec.target,
          sourceHandle: vertical
            ? downward
              ? "source-bottom"
              : "source-top"
            : "source-right",
          targetHandle: vertical
            ? downward
              ? "target-top"
              : "target-bottom"
            : "target-left",
          type: "smoothstep",
          ...(isFlow ? { label: spec.name } : {}),
          selected: isFlow && sameTarget(spec.selection, selected),
          selectable: isFlow,
          focusable: isFlow,
          data: { kind: spec.kind, name: spec.name },
          style: isFlow
            ? { stroke: "var(--accent)", strokeWidth: 1.8 }
            : {
                stroke: "var(--ink-faint)",
                strokeWidth: 1,
                strokeDasharray: "5 4",
                opacity: 0.65,
              },
          labelStyle: {
            fill: isFlow ? "var(--ink)" : "var(--ink-faint)",
            fontFamily: "var(--font-mono)",
            fontSize: 11,
          },
          labelBgStyle: {
            fill: "var(--bg-elevated)",
            fillOpacity: 0.88,
          },
          labelBgPadding: [6, 3],
          labelBgBorderRadius: 4,
          ariaLabel: isFlow
            ? t("canvas.flowAria", { name: spec.name })
            : t("canvas.infoAria", { name: spec.name }),
        };
        if (!isFlow) {
          return base;
        }
        return {
          ...base,
          markerEnd: {
            type: MarkerType.ArrowClosed,
            color: "var(--accent)",
            width: 16,
            height: 16,
          },
        };
      }),
    [layout, nodeById, selected, t],
  );

  const handleNodeClick = useCallback(
    (_event: ReactMouseEvent, node: CanvasNode): void => {
      if (node.data.target !== null) {
        onSelect(node.data.target);
      }
    },
    [onSelect],
  );

  const handleEdgeClick = useCallback(
    (_event: ReactMouseEvent, edge: Edge): void => {
      const spec = layout.edges.find((candidate) => candidate.id === edge.id);
      if (spec?.selection) {
        onSelect(spec.selection);
      }
    },
    [layout, onSelect],
  );

  const handleCanvasKeyDown = (
    event: React.KeyboardEvent<HTMLDivElement>,
  ): void => {
    if (event.key !== "Enter" && event.key !== " ") {
      return;
    }
    const focused =
      event.target instanceof Element
        ? event.target.closest(".react-flow__node, .react-flow__edge")
        : null;
    const id = focused?.getAttribute("data-id") ?? null;
    if (id === null) {
      return;
    }
    event.preventDefault();
    const nodeSpec = layout.nodes.find((node) => node.id === id);
    if (nodeSpec?.target) {
      onSelect(nodeSpec.target);
      return;
    }
    const edgeSpec = layout.edges.find((edge) => edge.id === id);
    if (edgeSpec?.selection) {
      onSelect(edgeSpec.selection);
    }
  };

  return (
    <div className="model-canvas">
      <div className="model-canvas__flow" onKeyDown={handleCanvasKeyDown}>
        <ReactFlow
          id={`canvas-${meta.id}`}
          nodes={nodes}
          edges={edges}
          nodeTypes={NODE_TYPES}
          onNodeClick={handleNodeClick}
          onEdgeClick={handleEdgeClick}
          fitView
          fitViewOptions={{ padding: 0.26 }}
          minZoom={0.35}
          maxZoom={1.6}
          nodesDraggable={false}
          nodesConnectable={false}
          nodesFocusable
          edgesFocusable
          elementsSelectable
          preventScrolling={false}
          zoomOnScroll={false}
          zoomOnPinch={false}
          panOnScroll={false}
          colorMode="dark"
          aria-label={t("canvas.ariaLabel")}
        >
          <Background
            variant={BackgroundVariant.Dots}
            gap={18}
            size={1}
            color="rgba(127, 212, 255, 0.18)"
          />
          <Controls showInteractive={false} />
        </ReactFlow>
      </div>

      <CanvasFallback layout={layout} selected={selected} onSelect={onSelect} />
    </div>
  );
}
