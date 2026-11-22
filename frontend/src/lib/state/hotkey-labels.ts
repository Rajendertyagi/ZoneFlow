/**
 * Pure, user-facing labels for config hotkey names (U-04). Kept free of Svelte
 * runes so it can be unit-tested directly with `bun test`.
 *
 * NOTE: `profile_N` is deliberately labelled as a zone launch, NOT a profile
 * apply. The native side maps it to `Action::LaunchApp(N-1)` (src/system.rs),
 * which launches/focuses zone N of the CURRENT profile. Do not reword this to
 * "Apply profile N" — that would describe behaviour the binding does not have.
 */

const FRIENDLY: Record<string, string> = {
  clear_layout: 'Clear layout',
  move_left: 'Move window left',
  move_right: 'Move window right',
  move_up: 'Move window up',
  move_down: 'Move window down',
  maximize: 'Maximize window',
  restore: 'Restore window',
  reload: 'Reload configuration',
};

export function hotkeyLabel(name: string): string {
  if (FRIENDLY[name]) return FRIENDLY[name];
  const match = /^profile_(\d)$/.exec(name);
  return match ? `Launch/focus zone ${match[1]} (current profile)` : name;
}
