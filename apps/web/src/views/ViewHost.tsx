import { useTranslation } from "react-i18next";
import type { Frame, ModelMeta } from "@cetus/sim-client";
import { ChartPanel } from "../components/ChartPanel";
import type { SeriesStore } from "../lib/series";
import { pickView } from "./registry";

export interface ViewProps {
  meta: ModelMeta;
  frame: Frame;
  revision: number;
  seriesRef: { current: SeriesStore };
}

export function ViewHost(props: ViewProps): React.JSX.Element {
  const { t } = useTranslation();
  const view = pickView(props.meta.views);

  if (view === "series") {
    return (
      <ChartPanel
        meta={props.meta}
        frame={props.frame}
        revision={props.revision}
        seriesRef={props.seriesRef}
      />
    );
  }

  return (
    <section className="panel" aria-labelledby="view-unavailable-title">
      <header className="panel__head">
        <div>
          <p className="eyebrow">{t("view.unavailable.section")}</p>
          <h2 id="view-unavailable-title">{t("view.unavailable.title")}</h2>
        </div>
      </header>
      <p className="panel__body">{t("view.unavailable.body")}</p>
    </section>
  );
}
