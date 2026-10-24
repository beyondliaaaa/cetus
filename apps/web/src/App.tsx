import { useTranslation } from "react-i18next";
import { DiagnosticsPanel } from "./components/DiagnosticsPanel";
import { Header, modelTitle } from "./components/Header";
import { ModelInspector } from "./components/ModelInspector";
import { ParameterRail } from "./components/ParameterRail";
import { RunControls } from "./components/RunControls";
import { SnapshotPanel } from "./components/SnapshotPanel";
import { StartupState } from "./components/StartupState";
import { useSimWorkbench } from "./hooks/useSimWorkbench";
import { toggleLocale } from "./i18n";
import { normalizeLocale } from "./i18n/messages";
import { ViewHost } from "./views/ViewHost";

export function App(): React.JSX.Element {
  const { t, i18n } = useTranslation();
  const workbench = useSimWorkbench();
  const locale = normalizeLocale(i18n.resolvedLanguage ?? i18n.language);
  const activeMeta = workbench.active?.meta ?? null;
  const frame = workbench.frame;
  const selectedModel =
    workbench.models.find((model) => model.key === workbench.selectedKey) ?? null;
  const visibleDiagnostics = workbench.runtimeError
    ? [workbench.runtimeError, ...workbench.diagnostics]
    : workbench.diagnostics;

  return (
    <div className="app-shell">
      <div className="stars" aria-hidden="true" />

      <Header
        models={workbench.models}
        selectedKey={workbench.selectedKey}
        onSelect={workbench.selectModel}
        locale={locale}
        onToggleLocale={toggleLocale}
        kernel={workbench.kernel}
        ready={workbench.boot.status === "ready"}
      />

      <main className="stage">
        {workbench.boot.status === "booting" && (
          <StartupState
            variant="boot"
            done={workbench.boot.done}
            total={workbench.boot.total}
          />
        )}

        {workbench.boot.status === "error" && (
          <div className="stack">
            <section className="panel">
              <p className="eyebrow">{t("startup.errorTitle")}</p>
              <h2>{t("startup.errorTitle")}</h2>
              <p className="panel__body">{t("startup.errorBody")}</p>
              {workbench.boot.diagnostics.length === 0 && (
                <p className="notice notice--error" role="alert">
                  {t("error.noModels")}
                </p>
              )}
            </section>
            <DiagnosticsPanel diagnostics={workbench.boot.diagnostics} hash="" />
          </div>
        )}

        {workbench.boot.status === "ready" &&
          (activeMeta && frame ? (
            <>
              <RunControls
                meta={activeMeta}
                frame={frame}
                playing={workbench.playing}
                speed={workbench.speed}
                onTogglePlay={workbench.togglePlaying}
                onStep={workbench.stepOnce}
                onReset={workbench.resetSession}
                onSpeedChange={workbench.setSpeed}
              />

              <div className="workbench-grid">
                <ViewHost
                  key={activeMeta.id}
                  meta={activeMeta}
                  frame={frame}
                  revision={workbench.revision}
                  seriesRef={workbench.seriesRef}
                />
                <ParameterRail
                  key={`params-${activeMeta.id}`}
                  meta={activeMeta}
                  values={workbench.paramValues}
                  disabled={workbench.sessionLoading}
                  onCommit={workbench.setParamValue}
                />
                <SnapshotPanel
                  meta={activeMeta}
                  record={workbench.snapshotRecord}
                  notice={workbench.snapshotNotice}
                  onSaveMemory={workbench.saveToMemory}
                  onDownload={workbench.downloadSnapshot}
                  onRestoreMemory={workbench.restoreFromMemory}
                  onRestoreFile={workbench.restoreFromFile}
                />
                <DiagnosticsPanel
                  diagnostics={visibleDiagnostics}
                  hash={frame.hash}
                />
                <ModelInspector
                  key={`inspector-${activeMeta.id}`}
                  meta={activeMeta}
                  sourceYaml={selectedModel?.yaml ?? ""}
                  disabled={workbench.sessionLoading}
                  parseExpression={workbench.parseExpression}
                  queryValue={workbench.queryValue}
                  applyEdit={workbench.applyEdit}
                />
              </div>
            </>
          ) : workbench.sessionLoading ? (
            <StartupState
              variant="session"
              modelName={
                selectedModel ? modelTitle(selectedModel, locale) : (workbench.selectedKey ?? "")
              }
            />
          ) : (
            <div className="stack">
              <section className="panel">
                <p className="eyebrow">{t("startup.diagnostics")}</p>
                <h2>{t("error.loadModel")}</h2>
                {workbench.diagnostics.length === 0 && (
                  <p className="notice notice--error" role="alert">
                    {t("error.loadModel")}
                  </p>
                )}
              </section>
              <DiagnosticsPanel diagnostics={visibleDiagnostics} hash="" />
            </div>
          ))}
      </main>

      <footer className="site-footer">
        <span>{t("footer.tagline")}</span>
        <span>{t("footer.license")}</span>
      </footer>
    </div>
  );
}
