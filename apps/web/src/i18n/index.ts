/**
 * i18next bootstrap (P12).
 *
 * - ICU MessageFormat via `i18next-icu` (single-brace `{name}` interpolation).
 * - Flat keys (`keySeparator: false`) so the catalogue stays greppable.
 * - Detection: localStorage → navigator.language → zh-CN.
 * - `document.documentElement.lang` follows every language change so screen
 *   readers and spell-checkers agree with the visible copy.
 */

import i18next from "i18next";
import ICU from "i18next-icu";
import { initReactI18next } from "react-i18next";
import {
  normalizeLocale,
  resources,
  type Locale,
} from "./messages";

export const LOCALE_STORAGE_KEY = "cetus.locale";

function detectLocale(): Locale {
  if (typeof window === "undefined") {
    return "zh-CN";
  }
  try {
    const stored = window.localStorage.getItem(LOCALE_STORAGE_KEY);
    if (stored) {
      return normalizeLocale(stored);
    }
  } catch {
    // Storage can be unavailable in private modes; fall through to navigator.
  }
  return normalizeLocale(window.navigator.language);
}

export const i18n = i18next.createInstance();

void i18n
  .use(ICU)
  .use(initReactI18next)
  .init({
    resources,
    lng: detectLocale(),
    fallbackLng: "zh-CN",
    supportedLngs: ["zh-CN", "en"],
    defaultNS: "translation",
    keySeparator: false,
    nsSeparator: false,
    interpolation: { escapeValue: false },
    returnNull: false,
    // Resources are bundled, so init can complete synchronously and the first
    // React render already has translations (no flash of raw keys).
    initAsync: false,
  });

function applyDocumentLang(locale: string): void {
  if (typeof document === "undefined") {
    return;
  }
  const normalized = normalizeLocale(locale);
  document.documentElement.lang = normalized;
  document.title = i18n.t("app.documentTitle", { lng: normalized });
  document
    .querySelector('meta[name="description"]')
    ?.setAttribute("content", i18n.t("app.description", { lng: normalized }));
}

function persistLocale(locale: string): void {
  if (typeof window === "undefined") {
    return;
  }
  try {
    window.localStorage.setItem(LOCALE_STORAGE_KEY, normalizeLocale(locale));
  } catch {
    // Non-fatal: the session keeps the chosen language in memory.
  }
}

applyDocumentLang(i18n.language);
i18n.on("languageChanged", (language) => {
  applyDocumentLang(language);
  persistLocale(language);
});

export function currentLocale(): Locale {
  return normalizeLocale(i18n.resolvedLanguage ?? i18n.language);
}

export function setLocale(locale: Locale): void {
  void i18n.changeLanguage(locale);
}

export function toggleLocale(): void {
  setLocale(currentLocale() === "zh-CN" ? "en" : "zh-CN");
}
