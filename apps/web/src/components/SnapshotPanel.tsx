import { useRef } from "react";
import { useTranslation } from "react-i18next";
import type { ModelMeta } from "@cetus/sim-client";
import type { SnapshotNotice, SnapshotRecord } from "../hooks/useSimWorkbench";
import { formatFixed, formatNumber, timeUnitKey } from "../lib/format";

interface SnapshotPanelProps {
  meta: ModelMeta;
  record: SnapshotRecord | null;
  notice: SnapshotNotice | null;
  onSaveMemory: () => void;
  onDownload: () => void;
  onRestoreMemory: () => void;
  onRestoreFile: (file: File) => void;
}

export function SnapshotPanel({
  meta,
  record,
  notice,
  onSaveMemory,
  onDownload,
  onRestoreMemory,
  onRestoreFile,
}: SnapshotPanelProps) {
  const { t, i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? i18n.language;
  const fileInput = useRef<HTMLInputElement | null>(null);
  const unitKey = timeUnitKey(meta.time.unit);
  const unitLabel = unitKey ? t(unitKey) : meta.time.unit;

  const noticeText = ((): { text: string; error: boolean } | null => {
    if (!notice) {
      return null;
    }
    switch (notice.kind) {
      case "working":
        return { text: t("snapshot.working"), error: false };
      case "saved":
        return {
          text: t("snapshot.saved", {
            tick: formatNumber(notice.tick, locale, 6),
            time: formatFixed(notice.time, locale),
            unit: unitLabel,
          }),
          error: false,
        };
      case "downloaded":
        return { text: t("snapshot.downloaded"), error: false };
      case "restoredMemory":
        return { text: t("snapshot.restored"), error: false };
      case "restoredFile":
        return { text: t("snapshot.restoredFile"), error: false };
      case "error":
        return { text: t("snapshot.error", { message: notice.message }), error: true };
    }
  })();

  return (
    <section className="panel snapshot" aria-labelledby="snapshot-title">
      <header className="panel__head">
        <div>
          <p className="eyebrow">{t("snapshot.section")}</p>
          <h2 id="snapshot-title">{t("snapshot.title")}</h2>
        </div>
      </header>

      <p className="mono-note">
        {record
          ? t("snapshot.inMemory", {
              tick: formatNumber(record.tick, locale, 6),
              time: formatFixed(record.displayTime, locale),
              unit: unitLabel,
            })
          : t("snapshot.empty")}
      </p>

      <div className="snapshot__actions">
        <button type="button" className="button" onClick={onSaveMemory}>
          {t("snapshot.saveMemory")}
        </button>
        <button type="button" className="button" onClick={onDownload}>
          {t("snapshot.download")}
        </button>
        <button
          type="button"
          className="button"
          onClick={onRestoreMemory}
          disabled={!record}
        >
          {t("snapshot.restoreMemory")}
        </button>
        <button
          type="button"
          className="button"
          onClick={() => {
            fileInput.current?.click();
          }}
        >
          {t("snapshot.restoreFile")}
        </button>
        <input
          ref={fileInput}
          type="file"
          accept=".cetussnap"
          className="visually-hidden"
          aria-label={t("snapshot.fileAria")}
          onChange={(event) => {
            const file = event.target.files?.[0];
            if (file) {
              onRestoreFile(file);
            }
            event.target.value = "";
          }}
        />
      </div>

      {noticeText && (
        <p
          className={noticeText.error ? "notice notice--error" : "notice"}
          role={noticeText.error ? "alert" : "status"}
        >
          {noticeText.text}
        </p>
      )}
    </section>
  );
}
