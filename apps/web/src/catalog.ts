/**
 * Model catalogue.
 *
 * Content YAML is the single source of truth (P1). Vite inlines every module
 * YAML as a raw string at build time; no model equation is ever duplicated in
 * TypeScript. The glob below is relative to this file (`apps/web/src` →
 * repository root).
 */

export interface CatalogEntry {
  /** Repository content path, e.g. `content/modules/bio/epidemic-sir/module.yaml`. */
  key: string;
  yaml: string;
}

const modules = import.meta.glob<string>(
  "../../../content/modules/**/module.yaml",
  { query: "?raw", import: "default", eager: true },
);

/** Deterministically ordered by content path; the picker reorders by model id. */
export const catalog: readonly CatalogEntry[] = Object.entries(modules)
  .map(([key, yaml]) => ({ key, yaml }))
  .sort((left, right) => left.key.localeCompare(right.key));
