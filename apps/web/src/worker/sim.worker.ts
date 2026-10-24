/**
 * Simulation worker (§10.2).
 *
 * Owns the wasm instance. The main thread only talks to it through Comlink.
 * Every method is coarse-grained: the worker parses wasm's JSON status strings
 * once and ships plain objects back across the thread boundary.
 */

import * as Comlink from "comlink";
import init, {
  compileModel,
  hello_digest,
  kernel_version,
} from "../wasm/generated/cetus.js";
import type { CompiledModel, Sim } from "../wasm/generated/cetus.js";
import type {
  CompileResult,
  Diagnostic,
  Frame,
  ModelMeta,
  ModelPatch,
  ParseResult,
  ProbeSeries,
  QueryResult,
  SimWorkerApi,
} from "@cetus/sim-client";

// `init` is idempotent in wasm-bindgen's web target: concurrent callers await
// the same instantiation. Failures are surfaced to the main thread instead of
// hanging the UI in a loading state.
const ready = init().catch((error: unknown) => {
  console.error("[cetus] wasm init failed:", errorText(error));
  throw error;
});

/** The current compiled model and its metadata. */
let compiled: CompiledModel | null = null;
let meta: ModelMeta | null = null;
/** The running simulation, if the current model has been started. */
let sim: Sim | null = null;

const FALLBACK_CODE = "E0000";

function errorText(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

function asError(error: unknown): Error {
  return error instanceof Error ? error : new Error(errorText(error));
}

/**
 * wasm-bindgen throws the diagnostics document as a string. Parse it
 * defensively: a malformed or non-JSON failure still becomes one readable
 * diagnostic instead of escaping as an opaque exception.
 */
function toDiagnostics(error: unknown): Diagnostic[] {
  const text = errorText(error);
  const candidates = [text];
  const start = text.indexOf('{"diagnostics"');
  const end = text.lastIndexOf("}");
  if (start >= 0 && end > start) {
    candidates.push(text.slice(start, end + 1));
  }
  for (const candidate of candidates) {
    try {
      const parsed: unknown = JSON.parse(candidate);
      if (isRecord(parsed) && Array.isArray(parsed["diagnostics"])) {
        const diagnostics = parsed["diagnostics"]
          .map(diagnosticFrom)
          .filter((item): item is Diagnostic => item !== null);
        if (diagnostics.length > 0) {
          return diagnostics;
        }
      }
    } catch {
      // Not JSON — fall through to the single-diagnostic fallback.
    }
  }
  return [
    {
      severity: "error",
      code: FALLBACK_CODE,
      message: text,
      path: null,
      start: null,
      end: null,
      help: null,
    },
  ];
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function optionalString(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

function optionalNumber(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function diagnosticFrom(value: unknown): Diagnostic | null {
  if (typeof value === "string") {
    return {
      severity: "error",
      code: FALLBACK_CODE,
      message: value,
      path: null,
      start: null,
      end: null,
      help: null,
    };
  }
  if (!isRecord(value)) {
    return null;
  }
  return {
    severity: value["severity"] === "warning" ? "warning" : "error",
    code: optionalString(value["code"]) ?? FALLBACK_CODE,
    message: optionalString(value["message"]) ?? "unknown wasm error",
    path: optionalString(value["path"]),
    start: optionalNumber(value["start"]),
    end: optionalNumber(value["end"]),
    help: optionalString(value["help"]),
  };
}

/**
 * Wire-boundary parse. The kernel owns these JSON documents and their shape is
 * fixed by `crates/cetus-wasm`; the UI never re-validates trusted worker output.
 */
function readJson<T>(text: string): T {
  return JSON.parse(text) as T;
}

function freeSim(): void {
  if (sim) {
    sim.free();
    sim = null;
  }
}

function freeCompiled(): void {
  if (compiled) {
    compiled.free();
    compiled = null;
    meta = null;
  }
}

function requireCompiled(): CompiledModel {
  if (!compiled) {
    throw new Error("no model loaded");
  }
  return compiled;
}

function requireSim(): Sim {
  if (!sim) {
    throw new Error("no simulation running");
  }
  return sim;
}

function frameOf(running: Sim): Frame {
  return readJson<Frame>(running.statusJson());
}

function registerProbes(running: Sim, model: ModelMeta): void {
  for (const observable of model.observables) {
    running.addProbe(observable.name);
  }
}

const api: SimWorkerApi = {
  async helloDigest() {
    await ready;
    return hello_digest();
  },

  async kernelVersion() {
    await ready;
    return kernel_version();
  },

  async inspect(yaml: string): Promise<CompileResult> {
    await ready;
    let model: CompiledModel | null = null;
    try {
      model = compileModel(yaml);
      return { ok: true, meta: readJson<ModelMeta>(model.metaJson()) };
    } catch (error) {
      return { ok: false, diagnostics: toDiagnostics(error) };
    } finally {
      model?.free();
    }
  },

  async load(yaml: string): Promise<CompileResult> {
    await ready;
    let next: CompiledModel | null = null;
    try {
      next = compileModel(yaml);
      const nextMeta = readJson<ModelMeta>(next.metaJson());
      // Only replace the current model once the new one compiled cleanly.
      freeSim();
      freeCompiled();
      compiled = next;
      meta = nextMeta;
      next = null;
      return { ok: true, meta: nextMeta };
    } catch (error) {
      return { ok: false, diagnostics: toDiagnostics(error) };
    } finally {
      next?.free();
    }
  },

  async start(seed = 1): Promise<Frame> {
    await ready;
    const model = requireCompiled();
    if (!meta) {
      throw new Error("no model loaded");
    }
    let next: Sim | null = null;
    try {
      next = model.start(seed);
      registerProbes(next, meta);
    } catch (error) {
      next?.free();
      throw asError(error);
    }
    freeSim();
    sim = next;
    return frameOf(next);
  },

  async frame(): Promise<Frame> {
    await ready;
    return frameOf(requireSim());
  },

  async step(n: number): Promise<Frame> {
    await ready;
    const running = requireSim();
    try {
      return readJson<Frame>(running.step(n));
    } catch (error) {
      throw asError(error);
    }
  },

  async runUntil(displayTime: number): Promise<Frame> {
    await ready;
    const running = requireSim();
    try {
      return readJson<Frame>(running.runUntil(displayTime));
    } catch (error) {
      throw asError(error);
    }
  },

  async setParam(name: string, value: number, unit?: string): Promise<Frame> {
    await ready;
    const running = requireSim();
    try {
      return readJson<Frame>(running.setParam(name, value, unit ?? null));
    } catch (error) {
      throw asError(error);
    }
  },

  async probes(names: string[], sinceTick: number): Promise<ProbeSeries[]> {
    await ready;
    const running = requireSim();
    try {
      return names.map((name) => ({
        name,
        points: running.readProbe(name, sinceTick),
      }));
    } catch (error) {
      throw asError(error);
    }
  },

  async snapshot(): Promise<Uint8Array> {
    await ready;
    try {
      return requireSim().snapshot();
    } catch (error) {
      throw asError(error);
    }
  },

  async restore(bytes: Uint8Array): Promise<Frame> {
    await ready;
    const model = requireCompiled();
    if (!meta) {
      throw new Error("no model loaded");
    }
    let next: Sim | null = null;
    try {
      next = model.restore(bytes);
      // Snapshots already carry probe registrations; addProbe is idempotent,
      // so this also covers snapshots produced by future kernels that omit it.
      registerProbes(next, meta);
    } catch (error) {
      next?.free();
      throw asError(error);
    }
    freeSim();
    sim = next;
    return frameOf(next);
  },

  async parseExpr(source: string, expectedUnit?: string): Promise<ParseResult> {
    await ready;
    const model = requireCompiled();
    try {
      return readJson<ParseResult>(
        model.parseExpression(source, expectedUnit ?? null),
      );
    } catch (error) {
      throw asError(error);
    }
  },

  async query(expr: string): Promise<QueryResult> {
    await ready;
    try {
      return readJson<QueryResult>(requireSim().query(expr));
    } catch (error) {
      throw asError(error);
    }
  },

  async editModel(patch: ModelPatch) {
    await ready;
    try {
      const yaml = requireCompiled().editModel(JSON.stringify(patch));
      return { ok: true, yaml };
    } catch (error) {
      return { ok: false, message: errorText(error) };
    }
  },

  async stateHash(): Promise<string> {
    await ready;
    return requireSim().stateHash();
  },
};

Comlink.expose(api);
