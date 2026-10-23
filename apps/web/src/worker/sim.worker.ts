/**
 * Simulation worker (§10.2).
 *
 * Owns the wasm instance. The main thread only talks to it through Comlink.
 * In M0 it exposes the cross-target determinism witness; M1 adds `compile`,
 * `Sim`, probes and snapshots (§5.13).
 */

import * as Comlink from "comlink";
import init, { hello_digest, kernel_version } from "../wasm/generated/cetus.js";
import type { SimWorkerApi } from "@cetus/sim-client";

// `init` is idempotent in wasm-bindgen's web target: concurrent callers await
// the same instantiation. Failures are surfaced to the main thread instead of
// hanging the UI in a loading state.
const ready = init().catch((error: unknown) => {
  const message = error instanceof Error ? error.message : String(error);
  console.error("[cetus] wasm init failed:", message);
  throw error;
});

const api: SimWorkerApi = {
  async helloDigest() {
    await ready;
    return hello_digest();
  },
  async kernelVersion() {
    await ready;
    return kernel_version();
  },
};

Comlink.expose(api);
