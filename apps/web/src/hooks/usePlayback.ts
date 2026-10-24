/**
 * requestAnimationFrame playback loop.
 *
 * The loop advances a *desired* display time by `speed × wallDelta` and asks
 * the worker to catch up. Calls are serialized with an in-flight guard so a
 * slow run never queues dozens of runUntil requests; React state is throttled
 * separately by the workbench hook.
 */

import { useEffect, useRef } from "react";

export interface PlaybackOptions {
  playing: boolean;
  /** Display units per real second. */
  speed: number;
  horizon: number | null;
  currentTime: number;
  advance: (targetDisplayTime: number) => Promise<void>;
}

const MAX_WALL_DELTA_SECONDS = 0.1;
const ADVANCE_INTERVAL_MS = 33;

export function usePlayback({
  playing,
  speed,
  horizon,
  currentTime,
  advance,
}: PlaybackOptions): void {
  const advanceRef = useRef(advance);
  const currentTimeRef = useRef(currentTime);

  useEffect(() => {
    advanceRef.current = advance;
  }, [advance]);

  useEffect(() => {
    currentTimeRef.current = currentTime;
  }, [currentTime]);

  useEffect(() => {
    if (!playing) {
      return;
    }

    const limit = horizon ?? Number.POSITIVE_INFINITY;
    let desired = currentTimeRef.current;
    let last = performance.now();
    let lastAdvance = 0;
    let inFlight = false;
    let frameId = 0;
    let cancelled = false;

    const tick = (now: number): void => {
      if (cancelled) {
        return;
      }
      const wallDelta = Math.min(
        MAX_WALL_DELTA_SECONDS,
        Math.max(0, (now - last) / 1000),
      );
      last = now;
      desired = Math.min(limit, desired + speed * wallDelta);

      if (
        !inFlight &&
        desired > currentTimeRef.current &&
        now - lastAdvance >= ADVANCE_INTERVAL_MS
      ) {
        inFlight = true;
        lastAdvance = now;
        const target = desired;
        void advanceRef.current(target).finally(() => {
          inFlight = false;
        });
      }

      frameId = requestAnimationFrame(tick);
    };

    frameId = requestAnimationFrame(tick);
    return () => {
      cancelled = true;
      cancelAnimationFrame(frameId);
    };
  }, [playing, speed, horizon]);
}
