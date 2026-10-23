/**
 * `@cetus/sim-client` — the only place the UI is allowed to talk to the
 * simulation worker.
 *
 * Threading model (§10.2): the main thread never calls into wasm directly.
 * It creates a `Worker`, wraps it with Comlink, and awaits coarse-grained
 * calls. The worker owns the wasm instance.
 */

import * as Comlink from "comlink";

/** The coarse-grained API exposed by the simulation worker. */
export interface SimWorkerApi {
  /** Cross-target determinism witness (M0). */
  helloDigest(): Promise<string>;
  /** Kernel crate version, for cache busting. */
  kernelVersion(): Promise<string>;
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
  return {
    api,
    dispose() {
      worker.terminate();
    },
  };
}
