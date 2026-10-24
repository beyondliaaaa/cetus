/**
 * `@cetus/sim-client` — the only place the UI is allowed to talk to the
 * simulation worker.
 *
 * Threading model (§10.2): the main thread never calls into wasm directly.
 * It creates a `Worker`, wraps it with Comlink, and awaits coarse-grained
 * calls. The worker owns the wasm instance.
 *
 * This package is bundler-agnostic on purpose: it never imports Vite helpers
 * and the caller owns the `new Worker(new URL(...), { type: "module" })`
 * expression.
 */

import * as Comlink from "comlink";

/** View kinds a model may advertise. The canvas layer will consume these. */
export type ViewKind =
  | "series"
  | "phase"
  | "field2d"
  | "vector_field"
  | "network"
  | "particles"
  | "scene2d"
  | "tree"
  | "matrix"
  | "bloch_sphere"
  | "circuit"
  | "automaton"
  | "trace";

export interface Diagnostic {
  severity: "error" | "warning";
  code: string;
  message: string;
  path: string | null;
  start: number | null;
  end: number | null;
  help: string | null;
}

export interface Bounds {
  min: number | null;
  max: number | null;
}

export interface TimeMeta {
  unit: string;
  unitFactor: number;
  dt: number;
  horizon: number | null;
  integrator: string;
  kind: string;
  start: number;
  startTick: number;
}

export interface ParameterMeta {
  name: string;
  unit: string;
  unitFactor: number;
  offset: number;
  value: number;
  siValue: number;
  range: [number, number] | null;
  siRange: [number, number] | null;
  role: "fixed" | "control" | "input";
  source: string | null;
  dim: string;
}

export interface StockMeta {
  name: string;
  unit: string;
  unitFactor: number;
  initial: number;
  siInitial: number;
  bounds: Bounds;
  dim: string;
}

export interface AuxiliaryMeta {
  name: string;
  unit: string;
  unitFactor: number;
  eq: string;
  dim: string;
}

export interface FlowMeta {
  name: string;
  from: string | null;
  to: string | null;
  unit: string;
  unitFactor: number;
  eq: string;
  dim: string;
}

export interface ObservableMeta {
  name: string;
  unit: string;
  unitFactor: number;
  display: string | null;
  dim: string;
}

export interface InvariantMeta {
  id: string;
  kind: string;
}

export interface ModelMeta {
  id: string;
  version: string;
  title: Record<string, string>;
  tier: string;
  paradigms: string[];
  views: ViewKind[];
  contentHash: string;
  time: TimeMeta;
  parameters: ParameterMeta[];
  stocks: StockMeta[];
  auxiliaries: AuxiliaryMeta[];
  flows: FlowMeta[];
  observables: ObservableMeta[];
  invariants: InvariantMeta[];
}

export type CompileResult =
  | { ok: true; meta: ModelMeta }
  | { ok: false; diagnostics: Diagnostic[] };

export interface Frame {
  tick: number;
  time: number;
  displayTime: number;
  done: boolean;
  hash: string;
  observables: Record<string, number>;
}

/** Interleaved `[tick, valueSi, ...]` pairs recorded after `sinceTick`. */
export interface ProbeSeries {
  name: string;
  points: Float64Array;
}

export type ParseResult =
  | {
      ok: true;
      dim: string | null;
      unit: string | null;
      type: "scalar" | "bool" | "other";
      matchesExpected: boolean | null;
      expectedUnit: string | null;
    }
  | { ok: false; code: string; message: string; start: number; end: number };

export type QueryResult =
  | { ok: true; siValue: number; value: number; unit: string; dim: string }
  | { ok: false; code: string; message: string; start: number; end: number };

export type ModelPatch =
  | {
      op: "set_eq";
      section: "flows" | "auxiliaries" | "observables";
      name: string;
      eq: string;
    }
  | { op: "set_initial"; name: string; expr: string }
  | { op: "set_value"; name: string; value: number };

/** The coarse-grained API exposed by the simulation worker. */
export interface SimWorkerApi {
  /** Cross-target determinism witness (M0). */
  helloDigest(): Promise<string>;
  /** Kernel crate version, for cache busting. */
  kernelVersion(): Promise<string>;
  /** Compile a model, return its metadata, and do not keep it. */
  inspect(yaml: string): Promise<CompileResult>;
  /** Compile a model and keep it as the current model. */
  load(yaml: string): Promise<CompileResult>;
  /** Start a fresh simulation and probe every observable. */
  start(seed?: number): Promise<Frame>;
  frame(): Promise<Frame>;
  /** Advance one macro step. */
  step(n: number): Promise<Frame>;
  /** Advance until a display time (in the model's time unit). */
  runUntil(displayTime: number): Promise<Frame>;
  /** Set a parameter expressed in its display unit. */
  setParam(name: string, value: number, unit?: string): Promise<Frame>;
  /** Incremental probe reads as interleaved `[tick, valueSi, ...]` pairs. */
  probes(names: string[], sinceTick: number): Promise<ProbeSeries[]>;
  snapshot(): Promise<Uint8Array>;
  /** Restore a snapshot and re-register every observable probe. */
  restore(bytes: Uint8Array): Promise<Frame>;
  parseExpr(source: string, expectedUnit?: string): Promise<ParseResult>;
  query(expr: string): Promise<QueryResult>;
  editModel(
    patch: ModelPatch,
  ): Promise<{ ok: true; yaml: string } | { ok: false; message: string }>;
  stateHash(): Promise<string>;
}

/** Handle returned by {@link createSimWorker}. */
export interface SimClient {
  api: Comlink.Remote<SimWorkerApi>;
  /** Terminates the worker. Safe to call more than once. */
  dispose(): void;
}

/**
 * Spawn the simulation worker.
 *
 * @param createWorker - a factory so the caller (Vite app) owns the
 *   `new Worker(new URL(...), { type: "module" })` expression. This keeps the
 *   package bundler-agnostic.
 */
export function createSimWorker(createWorker: () => Worker): SimClient {
  const worker = createWorker();
  const api = Comlink.wrap<SimWorkerApi>(worker);
  let disposed = false;
  return {
    api,
    dispose() {
      if (disposed) {
        return;
      }
      disposed = true;
      worker.terminate();
    },
  };
}
