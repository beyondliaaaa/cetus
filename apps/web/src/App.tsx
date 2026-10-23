import { useEffect, useState } from "react";
import { createSimWorker } from "@cetus/sim-client";
import { translate, type Locale, type MessageKey } from "./i18n/messages";

type DigestState =
  | { status: "loading" }
  | { status: "ready"; digest: string; version: string }
  | { status: "error"; message: string };

export function App(): React.JSX.Element {
  const [locale, setLocale] = useState<Locale>("zh-CN");
  const t = (key: MessageKey): string => translate(locale, key);

  const [state, setState] = useState<DigestState>({ status: "loading" });

  useEffect(() => {
    const client = createSimWorker(
      () => new Worker(new URL("./worker/sim.worker.ts", import.meta.url), { type: "module" }),
    );
    let cancelled = false;
    void (async () => {
      try {
        const [digest, version] = await Promise.all([
          client.api.helloDigest(),
          client.api.kernelVersion(),
        ]);
        if (!cancelled) {
          setState({ status: "ready", digest, version });
        }
      } catch (error) {
        if (!cancelled) {
          setState({
            status: "error",
            message: error instanceof Error ? error.message : String(error),
          });
        }
      }
    })();
    return () => {
      cancelled = true;
      client.dispose();
    };
  }, []);

  return (
    <main className="shell">
      <div className="stars" aria-hidden="true" />

      <header className="header">
        <p className="eyebrow">CETUS · M0</p>
        <h1>{t("app.title")}</h1>
        <p className="tagline">{t("app.tagline")}</p>
        <button
          type="button"
          className="lang"
          onClick={() => {
            setLocale((current) => (current === "zh-CN" ? "en" : "zh-CN"));
          }}
        >
          {t("lang.switch")}
        </button>
      </header>

      <section className="card" aria-labelledby="m0-title">
        <h2 id="m0-title">{t("app.m0.title")}</h2>
        <p>{t("app.m0.body")}</p>

        <dl className="facts">
          <div>
            <dt>{t("app.digest.label")}</dt>
            <dd>
              {state.status === "loading" && <span className="muted">{t("app.digest.loading")}</span>}
              {state.status === "error" && (
                <span className="error" role="alert">
                  {t("app.digest.error")} <code>{state.message}</code>
                </span>
              )}
              {state.status === "ready" && <code className="digest">{state.digest}</code>}
            </dd>
          </div>
          {state.status === "ready" && (
            <div>
              <dt>{t("app.version.label")}</dt>
              <dd>
                <code>{state.version}</code>
              </dd>
            </div>
          )}
        </dl>
      </section>

      <section className="card card--quiet" aria-labelledby="next-title">
        <h2 id="next-title">{t("app.next.title")}</h2>
        <p>{t("app.next.body")}</p>
      </section>

      <footer className="footer">
        <span>Cetus · 鲸鱼娘历险记</span>
        <span>MIT OR Apache-2.0 · content CC BY-SA 4.0</span>
      </footer>
    </main>
  );
}
