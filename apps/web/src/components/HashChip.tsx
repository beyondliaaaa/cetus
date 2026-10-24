import { useTranslation } from "react-i18next";
import { useCopy } from "../hooks/useCopy";

interface HashChipProps {
  hash: string;
  full?: boolean;
}

export function HashChip({ hash, full = false }: HashChipProps) {
  const { t } = useTranslation();
  const { state, copy } = useCopy();
  const short = hash.slice(0, 10);
  const stateLabel =
    state === "copied"
      ? t("hash.copied")
      : state === "failed"
        ? t("hash.copyFailed")
        : null;

  return (
    <button
      type="button"
      className={full ? "hash-chip hash-chip--full" : "hash-chip"}
      onClick={() => {
        void copy(hash);
      }}
      disabled={hash.length === 0}
      aria-label={t("hash.copy")}
      title={hash}
    >
      <span className="hash-chip__label">{t("hash.label")}</span>
      <code>{full ? hash : short.length > 0 ? short : "—"}</code>
      {stateLabel && <span className="hash-chip__state">{stateLabel}</span>}
    </button>
  );
}
