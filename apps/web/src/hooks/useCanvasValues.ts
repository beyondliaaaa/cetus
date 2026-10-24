import { useEffect, useState } from "react";

export interface CanvasValues {
  [name: string]: number | null;
}

const VALUE_REFRESH_MS = 250;

/** Polls live display values at ~4 Hz, pausing while the document is hidden. */
export function useCanvasValues(
  names: readonly string[],
  queryValue: (name: string) => Promise<number | null>,
): CanvasValues {
  const [values, setValues] = useState<CanvasValues>({});
  const namesKey = names.join("|");

  useEffect(() => {
    if (namesKey === "") {
      return;
    }
    setValues({});
    let cancelled = false;
    let inFlight = false;

    const read = async (): Promise<void> => {
      if (
        cancelled ||
        inFlight ||
        (typeof document !== "undefined" &&
          document.visibilityState !== "visible")
      ) {
        return;
      }
      inFlight = true;
      try {
        const entries = await Promise.all(
          names.map(async (name) => {
            const value = await queryValue(name);
            return [name, value] as const;
          }),
        );
        if (!cancelled) {
          setValues(Object.fromEntries(entries));
        }
      } finally {
        inFlight = false;
      }
    };

    void read();
    const timer = window.setInterval(() => {
      void read();
    }, VALUE_REFRESH_MS);
    const onVisibilityChange = (): void => {
      if (document.visibilityState === "visible") {
        void read();
      }
    };
    document.addEventListener("visibilitychange", onVisibilityChange);

    return () => {
      cancelled = true;
      window.clearInterval(timer);
      document.removeEventListener("visibilitychange", onVisibilityChange);
    };
  }, [names, namesKey, queryValue]);

  return values;
}
