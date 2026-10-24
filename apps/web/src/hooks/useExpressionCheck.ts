import { useCallback, useEffect, useRef, useState } from "react";
import type { ParseResult } from "@cetus/sim-client";

const VALIDATION_DELAY_MS = 250;

export interface ExpressionCheck {
  checking: boolean;
  parsed: ParseResult | null;
  clearCheck: () => void;
}

/** Debounced `parseExpr` check that ignores responses from stale drafts. */
export function useExpressionCheck(
  source: string,
  expectedUnit: string | null,
  parseExpression: (
    source: string,
    expectedUnit?: string,
  ) => Promise<ParseResult>,
): ExpressionCheck {
  const [checking, setChecking] = useState(false);
  const [parsed, setParsed] = useState<ParseResult | null>(null);
  const requestRef = useRef(0);

  const clearCheck = useCallback((): void => {
    requestRef.current += 1;
    setParsed(null);
    setChecking(false);
  }, []);

  useEffect(() => {
    requestRef.current += 1;
    const request = requestRef.current;
    setParsed(null);
    setChecking(false);
    if (source.trim() === "" || expectedUnit === null) {
      return;
    }
    let cancelled = false;
    const timer = window.setTimeout(() => {
      if (cancelled) {
        return;
      }
      setChecking(true);
      void parseExpression(source, expectedUnit)
        .then((result) => {
          if (cancelled || requestRef.current !== request) {
            return;
          }
          setParsed(result);
          setChecking(false);
        })
        .catch((error: unknown) => {
          if (cancelled || requestRef.current !== request) {
            return;
          }
          setParsed({
            ok: false,
            code: "E1901",
            message: error instanceof Error ? error.message : String(error),
            start: 0,
            end: 0,
          });
          setChecking(false);
        });
    }, VALIDATION_DELAY_MS);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [source, expectedUnit, parseExpression]);

  return { checking, parsed, clearCheck };
}
