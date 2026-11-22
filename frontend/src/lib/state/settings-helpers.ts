/**
 * Pure helpers shared by the Settings view and the top bar.
 *
 * No Svelte runes and no DOM globals, so bun can exercise these directly with
 * plain values — no component-test framework needed.
 */

/** Accepted hex color forms: `#RGB` and `#RRGGBB` (case-insensitive). */
const HEX_COLOR = /^#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/;

/**
 * Validate a zone tint before writing it into settings. An invalid value must
 * leave the existing tint untouched rather than corrupting the palette.
 */
export function isValidHexColor(value: string): boolean {
  return HEX_COLOR.test(value);
}

export type CloseDecision = 'dialog' | 'minimize' | 'exit';

/**
 * Map the persisted close action to what the editor's close button should do.
 * `ask` (or any missing/unknown value) keeps the confirmation dialog; the two
 * remembered choices bypass it entirely.
 */
export function resolveCloseAction(action: string | null | undefined): CloseDecision {
  if (action === 'minimize') return 'minimize';
  if (action === 'exit') return 'exit';
  return 'dialog';
}
