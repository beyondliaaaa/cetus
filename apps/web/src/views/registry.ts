import type { ViewKind } from "@cetus/sim-client";

/** View kinds the front-end can actually render (K12 registry, §5.6.3). */
export const IMPLEMENTED_VIEWS = ["series"] as const satisfies readonly ViewKind[];

export type ImplementedView = (typeof IMPLEMENTED_VIEWS)[number];

/**
 * Picks the first declared view this build can render.
 *
 * Paradigms may declare views the front-end has not implemented yet (`phase` is
 * declared by SD but arrives with a later milestone), so this walks the
 * paradigm's declaration order and ignores unknown kinds instead of failing.
 */
export function pickView(views: readonly ViewKind[]): ImplementedView | null {
  for (const view of views) {
    const implemented = IMPLEMENTED_VIEWS.find((candidate) => candidate === view);
    if (implemented !== undefined) {
      return implemented;
    }
  }
  return null;
}
