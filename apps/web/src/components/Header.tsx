import { useTranslation } from "react-i18next";
import type { LoadedModel } from "../hooks/useSimWorkbench";
import type { Locale } from "../i18n/messages";

interface HeaderProps {
  models: readonly LoadedModel[];
  selectedKey: string | null;
  onSelect: (key: string) => void;
  locale: Locale;
  onToggleLocale: () => void;
  kernel: { version: string; digest: string } | null;
  ready: boolean;
}

export function modelTitle(model: LoadedModel, locale: Locale): string {
  return model.meta.title[locale] ?? model.meta.id;
}

function shortDigest(digest: string): string {
  return digest.length > 12 ? `${digest.slice(0, 12)}…` : digest;
}

export function Header({
  models,
  selectedKey,
  onSelect,
  locale,
  onToggleLocale,
  kernel,
  ready,
}: HeaderProps) {
  const { t } = useTranslation();

  return (
    <header className="site-header">
      <div className="brand">
        <svg
          className="brand__glyph"
          viewBox="0 0 32 32"
          aria-hidden="true"
          focusable="false"
        >
          <circle cx="16" cy="16" r="12.5" fill="none" stroke="currentColor" strokeWidth="1.2" />
          <ellipse cx="16" cy="16" rx="5.2" ry="12.5" fill="none" stroke="currentColor" strokeWidth="1" />
          <path d="M3.8 16h24.4M7 9.2h18M7 22.8h18" fill="none" stroke="currentColor" strokeWidth="0.8" />
          <circle cx="16" cy="16" r="1.8" fill="currentColor" />
        </svg>
        <div>
          <p className="brand__word">CETUS</p>
          <h1 className="brand__title">{t("app.title")}</h1>
          <p className="brand__sub">{t("app.subtitle")}</p>
        </div>
      </div>

      <div className="site-header__tools">
        <label className="model-picker">
          <span className="eyebrow">{t("header.model")}</span>
          <select
            value={selectedKey ?? ""}
            onChange={(event) => {
              onSelect(event.target.value);
            }}
            disabled={!ready || models.length === 0}
          >
            {models.length === 0 && <option value="">—</option>}
            {models.map((model) => (
              <option key={model.key} value={model.key}>
                {modelTitle(model, locale)}
              </option>
            ))}
          </select>
        </label>
        <button
          type="button"
          className="ghost-button"
          aria-label={t("header.locale")}
          onClick={onToggleLocale}
        >
          {t("header.localeSwitch")}
        </button>
      </div>

      {kernel && (
        <p className="kernel-stamp">
          <span>{t("header.kernel")}</span>
          <code>{t("header.kernelVersion", { version: kernel.version })}</code>
          <span aria-hidden="true">·</span>
          <code title={kernel.digest}>{t("header.digest", { digest: shortDigest(kernel.digest) })}</code>
        </p>
      )}
    </header>
  );
}
