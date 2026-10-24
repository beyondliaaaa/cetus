/**
 * Pure helpers shared by the inspector and the equation editor: which elements
 * are editable, where a target's current expression lives, and which model
 * patch applies to it.
 *
 * The compiled metadata carries equations for flows and auxiliaries. It omits
 * observables and stores a stock's initial value only numerically, so those are
 * read from the authored YAML the workbench already loaded. If the field cannot
 * be read, a stock falls back to a unit-typed numeric literal.
 */

import type {
  ModelMeta,
  ModelPatch,
  ParseResult,
} from "@cetus/sim-client";
import { formatInputNumber } from "./format";
import type { CanvasTarget } from "./layout";
import { readYamlEquation, readYamlScalar } from "./modelSource";

export interface TargetInfo {
  unit: string;
  compiled: string | null;
}

export type TargetKind = CanvasTarget["kind"];

export interface TargetGroup {
  kind: TargetKind;
  names: readonly string[];
}

function assertNever(value: never): never {
  throw new Error(`unhandled target kind: ${String(value)}`);
}

export function targetGroups(meta: ModelMeta): TargetGroup[] {
  return [
    { kind: "flow", names: meta.flows.map((item) => item.name) },
    {
      kind: "auxiliary",
      names: meta.auxiliaries.map((item) => item.name),
    },
    {
      kind: "observable",
      names: meta.observables.map((item) => item.name),
    },
    { kind: "stock", names: meta.stocks.map((item) => item.name) },
  ];
}

export function editableTargets(meta: ModelMeta): CanvasTarget[] {
  return targetGroups(meta).flatMap((group) =>
    group.names.map((name): CanvasTarget => ({ kind: group.kind, name })),
  );
}

export function defaultTarget(meta: ModelMeta): CanvasTarget | null {
  return editableTargets(meta)[0] ?? null;
}

export function targetKey(target: CanvasTarget): string {
  return `${target.kind}:${target.name}`;
}

export function resolveTarget(
  meta: ModelMeta,
  yaml: string,
  target: CanvasTarget,
): TargetInfo | null {
  switch (target.kind) {
    case "flow": {
      const flow = meta.flows.find((item) => item.name === target.name);
      return flow ? { unit: flow.unit, compiled: flow.eq } : null;
    }
    case "auxiliary": {
      const auxiliary = meta.auxiliaries.find((item) => item.name === target.name);
      return auxiliary
        ? { unit: auxiliary.unit, compiled: auxiliary.eq }
        : null;
    }
    case "observable": {
      const observable = meta.observables.find((item) => item.name === target.name);
      return observable
        ? {
            unit: observable.unit,
            compiled: readYamlEquation(yaml, "observables", target.name),
          }
        : null;
    }
    case "stock": {
      const stock = meta.stocks.find((item) => item.name === target.name);
      if (!stock) {
        return null;
      }
      const authored = readYamlScalar(yaml, "stocks", target.name, "initial");
      const numeric = formatInputNumber(stock.initial);
      return {
        unit: stock.unit,
        compiled:
          authored ??
          (stock.unit === "1" ? numeric : `${numeric}[${stock.unit}]`),
      };
    }
    default:
      return assertNever(target.kind);
  }
}

/**
 * Mirrors the compiler's stock-initial rule: a dimensionless initial is
 * interpreted in the stock's declared unit, so `990` is a valid `person`
 * initial. Other targets still require an exact dimension match.
 */
export function effectiveMatch(
  parsed: ParseResult | null,
  kind: TargetKind,
): ParseResult | null {
  if (
    parsed?.ok === true &&
    kind === "stock" &&
    parsed.dim === "1" &&
    parsed.matchesExpected === false
  ) {
    return { ...parsed, matchesExpected: true };
  }
  return parsed;
}

export function patchFor(target: CanvasTarget, expression: string): ModelPatch {
  switch (target.kind) {
    case "stock":
      return { op: "set_initial", name: target.name, expr: expression };
    case "flow":
      return {
        op: "set_eq",
        section: "flows",
        name: target.name,
        eq: expression,
      };
    case "auxiliary":
      return {
        op: "set_eq",
        section: "auxiliaries",
        name: target.name,
        eq: expression,
      };
    case "observable":
      return {
        op: "set_eq",
        section: "observables",
        name: target.name,
        eq: expression,
      };
    default:
      return assertNever(target.kind);
  }
}
