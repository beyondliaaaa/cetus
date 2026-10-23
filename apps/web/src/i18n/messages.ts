/**
 * Minimal message catalogue.
 *
 * P12: every player-facing string goes through a key from day one. M2 replaces
 * this toy loader with i18next + ICU message format without changing call
 * sites (`t("key")`).
 */

export const zhCN = {
  "app.title": "鲸鱼娘历险记",
  "app.tagline": "用可运行的科学模型，亲手重建一个世界。",
  "app.m0.title": "M0 基础设施",
  "app.m0.body":
    "页面、Worker 与 WASM 内核已经连通。下面是内核在不同平台上算出的同一个指纹——它必须完全一致。",
  "app.digest.label": "跨平台指纹",
  "app.digest.loading": "正在唤醒内核……",
  "app.digest.error": "内核没有启动。请查看控制台，或重新构建 WASM 包。",
  "app.version.label": "内核版本",
  "app.next.title": "下一步",
  "app.next.body":
    "M1 会带来量纲、表达式与第一个系统动力学模型：太阳能板与蓄电池。届时，这里将不再是占位页面，而是一台可以摆弄的实验台。",
  "lang.switch": "English",
} as const;

export const en = {
  "app.title": "Cetus",
  "app.tagline": "Rebuild a world out of runnable scientific models.",
  "app.m0.title": "M0 — Foundation",
  "app.m0.body":
    "Page, worker and wasm kernel are wired together. Below is the same fingerprint computed on different platforms — it must match exactly.",
  "app.digest.label": "Cross-platform digest",
  "app.digest.loading": "Waking the kernel…",
  "app.digest.error": "The kernel did not start. Check the console or rebuild the wasm bundle.",
  "app.version.label": "Kernel version",
  "app.next.title": "Next",
  "app.next.body":
    "M1 brings dimensions, expressions and the first system-dynamics model: a solar panel and a battery. This page becomes a workbench you can tinker with.",
  "lang.switch": "中文",
} as const;

export type MessageKey = keyof typeof zhCN;
export type Locale = "zh-CN" | "en";

export const catalogs: Record<Locale, Record<MessageKey, string>> = {
  "zh-CN": zhCN,
  en,
};

export function translate(locale: Locale, key: MessageKey): string {
  return catalogs[locale][key];
}
