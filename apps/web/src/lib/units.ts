/**
 * Display-unit conversions.
 *
 * The kernel stores every value in SI. The UI shows `(si - offset) / factor`
 * and sends `display * factor + offset` back. Observable and parameter metadata
 * both carry the factor/offset pair, so this module never guesses a unit.
 */

/** Converts a kernel (SI) value into the model's display unit. */
export function toDisplayValue(
  siValue: number,
  unitFactor: number,
  offset = 0,
): number {
  return (siValue - offset) / unitFactor;
}

/** Converts a display-unit value into the kernel's SI value. */
export function toSiValue(
  displayValue: number,
  unitFactor: number,
  offset = 0,
): number {
  return displayValue * unitFactor + offset;
}

/** Converts a display-unit range to an SI range (same affine mapping). */
export function rangeToSi(
  range: readonly [number, number],
  unitFactor: number,
  offset = 0,
): [number, number] {
  return [toSiValue(range[0], unitFactor, offset), toSiValue(range[1], unitFactor, offset)];
}
