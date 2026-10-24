/**
 * The workbench session: worker lifecycle, model catalogue, probe accumulation,
 * playback, snapshots and diagnostics.
 *
 * All simulation work happens in the worker (P4 / §10.2). This hook only owns
 * UI state and mutable chart buffers; React state updates are throttled to
 * ~15 Hz so a long `runUntil` never re-renders per animation frame.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import {
  createSimWorker,
  type CompileResult,
  type Diagnostic,
  type Frame,
  type ModelMeta,
  type ModelPatch,
  type ParseResult,
  type ProbeSeries,
  type SimClient,
} from "@cetus/sim-client";
import { catalog, type CatalogEntry } from "../catalog";
import {
  appendProbePoints,
  createSeriesStore,
  type SeriesStore,
} from "../lib/series";
import { usePlayback } from "./usePlayback";

export interface LoadedModel {
  key: string;
  yaml: string;
  meta: ModelMeta;
}

export type BootState =
  | { status: "booting"; done: number; total: number }
  | { status: "ready" }
  | { status: "error"; diagnostics: Diagnostic[] };

export interface SnapshotRecord {
  bytes: Uint8Array;
  tick: number;
  displayTime: number;
}

export type SnapshotNotice =
  | { kind: "working" }
  | { kind: "saved"; tick: number; time: number }
  | { kind: "downloaded" }
  | { kind: "restoredMemory" }
  | { kind: "restoredFile" }
  | { kind: "error"; message: string };

interface ActiveModel {
  key: string;
  meta: ModelMeta;
}

interface InspectOutcome {
  entry: CatalogEntry;
  result: CompileResult;
}

const PREFERRED_MODEL_IDS = ["bio.epidemic-sir", "energy.solar-battery"];
const UI_UPDATE_INTERVAL_MS = 70;
const DEFAULT_SPEED = 10;

function modelRank(id: string): number {
  const index = PREFERRED_MODEL_IDS.indexOf(id);
  return index === -1 ? PREFERRED_MODEL_IDS.length : index;
}

function compareModels(left: LoadedModel, right: LoadedModel): number {
  const rank = modelRank(left.meta.id) - modelRank(right.meta.id);
  return rank !== 0 ? rank : left.meta.id.localeCompare(right.meta.id);
}

export function errorMessage(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

export function runtimeDiagnostic(error: unknown): Diagnostic {
  return {
    severity: "error",
    code: "RUNTIME",
    message: errorMessage(error),
    path: null,
    start: null,
    end: null,
    help: null,
  };
}

function editDiagnostic(message: string): Diagnostic {
  return {
    severity: "error",
    code: "E0003",
    message,
    path: null,
    start: null,
    end: null,
    help: null,
  };
}

function noModelParseResult(): ParseResult {
  return {
    ok: false,
    code: "E1901",
    message: "no model loaded",
    start: 0,
    end: 0,
  };
}

function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  if (target.isContentEditable) {
    return true;
  }
  const tag = target.tagName;
  return (
    tag === "INPUT" ||
    tag === "TEXTAREA" ||
    tag === "SELECT" ||
    tag === "BUTTON" ||
    target.getAttribute("role") === "button"
  );
}

function downloadBytes(bytes: Uint8Array, filename: string): void {
  const buffer = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(buffer).set(bytes);
  const blob = new Blob([buffer], { type: "application/octet-stream" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.rel = "noopener";
  anchor.click();
  window.setTimeout(() => {
    URL.revokeObjectURL(url);
  }, 1000);
}

export function useSimWorkbench() {
  const [boot, setBoot] = useState<BootState>({
    status: "booting",
    done: 0,
    total: catalog.length,
  });
  const [models, setModels] = useState<readonly LoadedModel[]>([]);
  const [failedDiagnostics, setFailedDiagnostics] = useState<
    Record<string, Diagnostic[]>
  >({});
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [active, setActive] = useState<ActiveModel | null>(null);
  const [frame, setFrame] = useState<Frame | null>(null);
  const [sessionLoading, setSessionLoading] = useState(false);
  const [diagnostics, setDiagnostics] = useState<Diagnostic[]>([]);
  const [runtimeError, setRuntimeError] = useState<Diagnostic | null>(null);
  const [kernel, setKernel] = useState<{ version: string; digest: string } | null>(
    null,
  );
  const [paramValues, setParamValues] = useState<Record<string, number>>({});
  const [playing, setPlaying] = useState(false);
  const [speed, setSpeed] = useState(DEFAULT_SPEED);
  const [revision, setRevision] = useState(0);
  const [snapshotRecord, setSnapshotRecord] = useState<SnapshotRecord | null>(null);
  const [snapshotNotice, setSnapshotNotice] = useState<SnapshotNotice | null>(null);

  const clientRef = useRef<SimClient | null>(null);
  const activeRef = useRef<ActiveModel | null>(null);
  const seriesRef = useRef<SeriesStore>(createSeriesStore([]));
  const lastProbeTickRef = useRef(0);
  const latestFrameRef = useRef<Frame | null>(null);
  const paramValuesRef = useRef<Record<string, number>>({});
  const lastFlushRef = useRef(0);
  const advanceInFlightRef = useRef(false);
  const switchTokenRef = useRef(0);

  const flush = useCallback((force = false) => {
    const next = latestFrameRef.current;
    if (!next) {
      return;
    }
    const now = performance.now();
    if (!force && now - lastFlushRef.current < UI_UPDATE_INTERVAL_MS) {
      return;
    }
    lastFlushRef.current = now;
    setFrame(next);
    setRevision((value) => value + 1);
    if (next.done) {
      setPlaying(false);
    }
  }, []);

  const fail = useCallback((error: unknown) => {
    setRuntimeError(runtimeDiagnostic(error));
    setPlaying(false);
  }, []);

  const resetSeries = useCallback((meta: ModelMeta) => {
    seriesRef.current = createSeriesStore(meta.observables.map((item) => item.name));
    lastProbeTickRef.current = 0;
    setRevision((value) => value + 1);
  }, []);

  const applyProbes = useCallback(
    (probes: ProbeSeries[], meta: ModelMeta) => {
      const specs = new Map(meta.observables.map((item) => [item.name, item]));
      const store = seriesRef.current;
      let maxTick = lastProbeTickRef.current;
      for (const probe of probes) {
        const spec = specs.get(probe.name);
        const buffer = store[probe.name];
        if (!spec || !buffer) {
          continue;
        }
        appendProbePoints(buffer, probe.points, meta.time, spec.unitFactor, 0);
        for (let index = 0; index + 1 < probe.points.length; index += 2) {
          const tick = probe.points[index];
          if (tick !== undefined && Number.isFinite(tick) && tick > maxTick) {
            maxTick = tick;
          }
        }
      }
      lastProbeTickRef.current = maxTick;
    },
    [],
  );

  const pullProbes = useCallback(
    async (client: SimClient, meta: ModelMeta) => {
      const names = meta.observables.map((item) => item.name);
      if (names.length === 0) {
        return;
      }
      const probes = await client.api.probes(names, lastProbeTickRef.current);
      applyProbes(probes, meta);
    },
    [applyProbes],
  );

  const applySession = useCallback(
    (key: string, meta: ModelMeta, nextFrame: Frame) => {
      const values: Record<string, number> = {};
      for (const parameter of meta.parameters) {
        values[parameter.name] = parameter.value;
      }
      activeRef.current = { key, meta };
      setActive({ key, meta });
      latestFrameRef.current = nextFrame;
      setFrame(nextFrame);
      paramValuesRef.current = values;
      setParamValues(values);
      resetSeries(meta);
      setDiagnostics([]);
      setRuntimeError(null);
      setSessionLoading(false);
      lastFlushRef.current = performance.now();
      setRevision((value) => value + 1);
    },
    [resetSeries],
  );

  useEffect(() => {
    const client = createSimWorker(
      () =>
        new Worker(new URL("../worker/sim.worker.ts", import.meta.url), {
          type: "module",
        }),
    );
    clientRef.current = client;
    let cancelled = false;

    const bumpProgress = (): void => {
      setBoot((current) =>
        current.status === "booting"
          ? { ...current, done: current.done + 1 }
          : current,
      );
    };

    void (async () => {
      try {
        const [digest, version] = await Promise.all([
          client.api.helloDigest(),
          client.api.kernelVersion(),
        ]);
        if (!cancelled) {
          setKernel({ version, digest });
        }
      } catch {
        // The kernel banner is de-emphasised; a failure surfaces through the
        // model inspection below.
      }

      try {
        const outcomes = await Promise.all(
          catalog.map(async (entry): Promise<InspectOutcome> => {
            try {
              const result = await client.api.inspect(entry.yaml);
              if (!cancelled) {
                bumpProgress();
              }
              return { entry, result };
            } catch (error) {
              if (!cancelled) {
                bumpProgress();
              }
              return {
                entry,
                result: { ok: false, diagnostics: [runtimeDiagnostic(error)] },
              };
            }
          }),
        );
        if (cancelled) {
          return;
        }

        const loaded: LoadedModel[] = [];
        const failures: Record<string, Diagnostic[]> = {};
        for (const outcome of outcomes) {
          if (outcome.result.ok) {
            loaded.push({
              key: outcome.entry.key,
              yaml: outcome.entry.yaml,
              meta: outcome.result.meta,
            });
          } else {
            failures[outcome.entry.key] = outcome.result.diagnostics;
          }
        }
        loaded.sort(compareModels);
        setModels(loaded);
        setFailedDiagnostics(failures);

        if (loaded.length === 0) {
          const all = outcomes.flatMap((outcome) =>
            outcome.result.ok ? [] : outcome.result.diagnostics,
          );
          setBoot({ status: "error", diagnostics: all });
          return;
        }

        setBoot({ status: "ready" });
        const preferred =
          loaded.find((model) => model.meta.id === "bio.epidemic-sir") ?? loaded[0];
        if (!preferred) {
          return;
        }
        // A user may switch models as soon as the picker appears; never let the
        // bootstrap load overwrite that newer choice.
        const bootToken = switchTokenRef.current;
        setSelectedKey(preferred.key);
        setSessionLoading(true);
        const loadedModel = await client.api.load(preferred.yaml);
        if (cancelled || switchTokenRef.current !== bootToken) {
          return;
        }
        if (!loadedModel.ok) {
          setDiagnostics(loadedModel.diagnostics);
          setSessionLoading(false);
          return;
        }
        const firstFrame = await client.api.start(1);
        if (!cancelled && switchTokenRef.current === bootToken) {
          applySession(preferred.key, loadedModel.meta, firstFrame);
        }
      } catch (error) {
        if (!cancelled) {
          setBoot({ status: "error", diagnostics: [runtimeDiagnostic(error)] });
          setSessionLoading(false);
        }
      }
    })();

    return () => {
      cancelled = true;
      clientRef.current = null;
      client.dispose();
    };
  }, [applySession]);

  const selectModel = useCallback(
    (key: string) => {
      void (async () => {
        const client = clientRef.current;
        if (!client) {
          return;
        }
        const loadedModel = models.find((model) => model.key === key);
        const token = switchTokenRef.current + 1;
        switchTokenRef.current = token;

        setSelectedKey(key);
        setPlaying(false);
        setSessionLoading(true);
        setDiagnostics([]);
        setRuntimeError(null);
        setActive(null);
        activeRef.current = null;
        setFrame(null);
        latestFrameRef.current = null;
        paramValuesRef.current = {};
        setParamValues({});
        seriesRef.current = createSeriesStore([]);
        lastProbeTickRef.current = 0;
        setRevision((value) => value + 1);
        setSnapshotRecord(null);
        setSnapshotNotice(null);

        if (!loadedModel) {
          setDiagnostics(failedDiagnostics[key] ?? []);
          setSessionLoading(false);
          return;
        }

        try {
          const result = await client.api.load(loadedModel.yaml);
          if (token !== switchTokenRef.current) {
            return;
          }
          if (!result.ok) {
            setDiagnostics(result.diagnostics);
            setSessionLoading(false);
            return;
          }
          const firstFrame = await client.api.start(1);
          if (token !== switchTokenRef.current) {
            return;
          }
          applySession(key, result.meta, firstFrame);
        } catch (error) {
          if (token === switchTokenRef.current) {
            setDiagnostics([runtimeDiagnostic(error)]);
            setSessionLoading(false);
          }
        }
      })();
    },
    [applySession, failedDiagnostics, models],
  );

  const advanceTo = useCallback(
    async (target: number) => {
      const client = clientRef.current;
      const current = activeRef.current;
      if (!client || !current || advanceInFlightRef.current) {
        return;
      }
      advanceInFlightRef.current = true;
      try {
        const next = await client.api.runUntil(target);
        if (activeRef.current !== current) {
          return;
        }
        latestFrameRef.current = next;
        await pullProbes(client, current.meta);
        if (activeRef.current !== current) {
          return;
        }
        flush(next.done);
      } catch (error) {
        fail(error);
      } finally {
        advanceInFlightRef.current = false;
      }
    },
    [fail, flush, pullProbes],
  );

  usePlayback({
    playing: playing && active !== null && frame !== null,
    speed,
    horizon: active?.meta.time.horizon ?? null,
    currentTime: frame?.displayTime ?? 0,
    advance: advanceTo,
  });

  const togglePlaying = useCallback(() => {
    setPlaying((current) => {
      if (current) {
        return false;
      }
      const latest = latestFrameRef.current;
      return latest !== null && !latest.done;
    });
  }, []);

  const stepOnce = useCallback(() => {
    void (async () => {
      const client = clientRef.current;
      const current = activeRef.current;
      if (!client || !current) {
        return;
      }
      setPlaying(false);
      try {
        const next = await client.api.step(1);
        if (activeRef.current !== current) {
          return;
        }
        latestFrameRef.current = next;
        await pullProbes(client, current.meta);
        if (activeRef.current === current) {
          flush(true);
        }
      } catch (error) {
        fail(error);
      }
    })();
  }, [fail, flush, pullProbes]);

  const resetSession = useCallback(() => {
    void (async () => {
      const client = clientRef.current;
      const current = activeRef.current;
      if (!client || !current) {
        return;
      }
      setPlaying(false);
      try {
        const next = await client.api.start(1);
        if (activeRef.current !== current) {
          return;
        }
        applySession(current.key, current.meta, next);
        setSnapshotNotice(null);
      } catch (error) {
        fail(error);
      }
    })();
  }, [applySession, fail]);

  const setParamValue = useCallback(
    (name: string, value: number, unit: string) => {
      void (async () => {
        const client = clientRef.current;
        const current = activeRef.current;
        if (!client || !current) {
          return;
        }
        const previous = paramValuesRef.current[name] ?? value;
        const optimistic = { ...paramValuesRef.current, [name]: value };
        paramValuesRef.current = optimistic;
        setParamValues(optimistic);
        try {
          const next = await client.api.setParam(name, value, unit);
          if (activeRef.current !== current) {
            return;
          }
          latestFrameRef.current = next;
          setFrame(next);
          setRuntimeError(null);
        } catch (error) {
          if (activeRef.current !== current) {
            return;
          }
          const reverted = { ...paramValuesRef.current, [name]: previous };
          paramValuesRef.current = reverted;
          setParamValues(reverted);
          fail(error);
        }
      })();
    },
    [fail],
  );

  /**
   * Type-checks a formula for the equation editor. A missing session is a
   * normal result (the editor may render during a model switch), never a throw.
   */
  const parseExpression = useCallback(
    async (
      source: string,
      expectedUnit?: string,
    ): Promise<ParseResult> => {
      const client = clientRef.current;
      if (!client || activeRef.current === null) {
        return noModelParseResult();
      }
      try {
        return await client.api.parseExpr(source, expectedUnit);
      } catch (error) {
        return {
          ok: false,
          code: "E1901",
          message: errorMessage(error),
          start: 0,
          end: 0,
        };
      }
    },
    [],
  );

  /**
   * Reads a live value. `query` converts SI into the declared display unit, so
   * the returned number is ready to render. Failures become `null` (a stale
   * value is never fabricated).
   */
  const queryValue = useCallback(async (name: string): Promise<number | null> => {
    const client = clientRef.current;
    if (!client || activeRef.current === null) {
      return null;
    }
    try {
      const result = await client.api.query(name);
      return result.ok ? result.value : null;
    } catch {
      return null;
    }
  }, []);

  /**
   * Applies a model patch, recompiles through the worker, and starts a fresh
   * simulation on success. The worker only swaps its model after a successful
   * compile, so a rejected edit leaves the previous simulation running; the
   * caller receives the diagnostics and can keep its draft.
   */
  const applyEdit = useCallback(
    async (
      patch: ModelPatch,
    ): Promise<{ ok: boolean; diagnostics?: Diagnostic[] }> => {
      const client = clientRef.current;
      const current = activeRef.current;
      if (!client || !current) {
        const diagnostic = editDiagnostic("no model loaded");
        setDiagnostics([diagnostic]);
        return { ok: false, diagnostics: [diagnostic] };
      }

      setPlaying(false);
      setSessionLoading(true);
      try {
        const edited = await client.api.editModel(patch);
        if (!edited.ok) {
          const diagnostic = editDiagnostic(edited.message);
          if (activeRef.current === current) {
            setDiagnostics([diagnostic]);
            setRuntimeError(null);
            setSessionLoading(false);
          }
          return { ok: false, diagnostics: [diagnostic] };
        }

        const loadedModel = await client.api.load(edited.yaml);
        if (activeRef.current !== current) {
          return { ok: false };
        }
        if (!loadedModel.ok) {
          setDiagnostics(loadedModel.diagnostics);
          setRuntimeError(null);
          setSessionLoading(false);
          return { ok: false, diagnostics: loadedModel.diagnostics };
        }

        const firstFrame = await client.api.start(1);
        if (activeRef.current !== current) {
          return { ok: false };
        }
        setModels((existing) =>
          existing.map((model) =>
            model.key === current.key
              ? { ...model, yaml: edited.yaml, meta: loadedModel.meta }
              : model,
          ),
        );
        applySession(current.key, loadedModel.meta, firstFrame);
        // The content hash changed: snapshots recorded against the old model
        // no longer describe the running plan.
        setSnapshotRecord(null);
        setSnapshotNotice(null);
        return { ok: true };
      } catch (error) {
        const diagnostic = runtimeDiagnostic(error);
        if (activeRef.current === current) {
          setDiagnostics([diagnostic]);
          setRuntimeError(null);
          setSessionLoading(false);
        }
        return { ok: false, diagnostics: [diagnostic] };
      }
    },
    [applySession],
  );

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent): void => {
      if (
        event.defaultPrevented ||
        event.repeat ||
        event.metaKey ||
        event.ctrlKey ||
        event.altKey
      ) {
        return;
      }
      if (isEditableTarget(event.target)) {
        return;
      }
      if (event.code === "Space") {
        event.preventDefault();
        togglePlaying();
      } else if (event.code === "ArrowRight") {
        event.preventDefault();
        stepOnce();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [stepOnce, togglePlaying]);

  const saveToMemory = useCallback(() => {
    void (async () => {
      const client = clientRef.current;
      const current = activeRef.current;
      const latest = latestFrameRef.current;
      if (!client || !current || !latest) {
        return;
      }
      setSnapshotNotice({ kind: "working" });
      try {
        const bytes = await client.api.snapshot();
        if (activeRef.current !== current) {
          return;
        }
        setSnapshotRecord({
          bytes,
          tick: latest.tick,
          displayTime: latest.displayTime,
        });
        setSnapshotNotice({
          kind: "saved",
          tick: latest.tick,
          time: latest.displayTime,
        });
      } catch (error) {
        setSnapshotNotice({ kind: "error", message: errorMessage(error) });
      }
    })();
  }, []);

  const downloadSnapshot = useCallback(() => {
    void (async () => {
      const client = clientRef.current;
      const current = activeRef.current;
      const latest = latestFrameRef.current;
      if (!client || !current || !latest) {
        return;
      }
      setSnapshotNotice({ kind: "working" });
      try {
        const bytes = await client.api.snapshot();
        if (activeRef.current !== current) {
          return;
        }
        downloadBytes(bytes, `cetus-${current.meta.id}-${latest.tick}.cetussnap`);
        setSnapshotNotice({ kind: "downloaded" });
      } catch (error) {
        setSnapshotNotice({ kind: "error", message: errorMessage(error) });
      }
    })();
  }, []);

  const restoreFromBytes = useCallback(
    (bytes: Uint8Array, source: "memory" | "file") => {
      void (async () => {
        const client = clientRef.current;
        const current = activeRef.current;
        if (!client || !current) {
          return;
        }
        setPlaying(false);
        setSnapshotNotice({ kind: "working" });
        try {
          const next = await client.api.restore(bytes);
          if (activeRef.current !== current) {
            return;
          }
          latestFrameRef.current = next;
          setFrame(next);
          resetSeries(current.meta);
          const names = current.meta.observables.map((item) => item.name);
          if (names.length > 0) {
            const probes = await client.api.probes(names, 0);
            if (activeRef.current !== current) {
              return;
            }
            applyProbes(probes, current.meta);
          }
          setSnapshotRecord({
            bytes,
            tick: next.tick,
            displayTime: next.displayTime,
          });
          flush(true);
          setSnapshotNotice({
            kind: source === "memory" ? "restoredMemory" : "restoredFile",
          });
          setDiagnostics([]);
          setRuntimeError(null);
        } catch (error) {
          setSnapshotNotice({ kind: "error", message: errorMessage(error) });
        }
      })();
    },
    [applyProbes, flush, resetSeries],
  );

  const restoreFromMemory = useCallback(() => {
    if (snapshotRecord) {
      restoreFromBytes(snapshotRecord.bytes, "memory");
    }
  }, [restoreFromBytes, snapshotRecord]);

  const restoreFromFile = useCallback(
    (file: File) => {
      void (async () => {
        try {
          const bytes = new Uint8Array(await file.arrayBuffer());
          restoreFromBytes(bytes, "file");
        } catch (error) {
          setSnapshotNotice({ kind: "error", message: errorMessage(error) });
        }
      })();
    },
    [restoreFromBytes],
  );

  return {
    boot,
    models,
    selectedKey,
    active,
    frame,
    sessionLoading,
    diagnostics,
    runtimeError,
    kernel,
    paramValues,
    playing,
    speed,
    revision,
    seriesRef,
    snapshotRecord,
    snapshotNotice,
    stateHash: frame?.hash ?? "",
    selectModel,
    togglePlaying,
    setSpeed,
    stepOnce,
    resetSession,
    setParamValue,
    parseExpression,
    queryValue,
    applyEdit,
    saveToMemory,
    downloadSnapshot,
    restoreFromMemory,
    restoreFromFile,
  };
}

export type SimWorkbench = ReturnType<typeof useSimWorkbench>;
