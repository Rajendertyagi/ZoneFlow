/**
 * Pure validation helpers for `.bat`/`.cmd` launcher scripts.
 *
 * These mirror `src/engine_core.rs` exactly so the picker can reject an invalid
 * selection before it is persisted. Kept free of Svelte runes so `bun test`
 * can import them directly. No filesystem access and no shell invocation here.
 */

/** CMD metacharacters rejected in a launcher path (mirrors the backend). */
export const CMD_METACHARACTERS = ['%', '!', '&', '|', '^', '<', '>'] as const;

export type ScriptPathError =
  | { kind: 'empty' }
  | { kind: 'bad_extension' }
  | { kind: 'metacharacter'; char: string }
  /** A double quote (breaks cmd.exe quoting) or a control character. */
  | { kind: 'unsafe_character'; char: string };

/** Human-readable reason, matching the backend's outcome message wording. */
export function scriptPathErrorMessage(error: ScriptPathError): string {
  switch (error.kind) {
    case 'empty':
      return 'path is empty';
    case 'bad_extension':
      return 'path must end in .bat or .cmd';
    case 'metacharacter':
      return `path contains CMD metacharacter '${error.char}'`;
    case 'unsafe_character':
      // Never embed a raw control character in a UI message.
      return error.char === '"'
        ? 'path contains a double quote'
        : `path contains control character U+${(error.char.codePointAt(0) ?? 0)
            .toString(16)
            .toUpperCase()
            .padStart(4, '0')}`;
  }
}

/** Whether a character is unsafe in a quoted `cmd.exe /c "<script>"` argument. */
function isUnsafePathChar(c: string): boolean {
  return (
    (CMD_METACHARACTERS as readonly string[]).includes(c) ||
    c === '"' ||
    /[\u0000-\u001f\u007f-\u009f]/.test(c)
  );
}

/** Validate a launcher path: non-empty, .bat/.cmd extension, no unsafe chars. */
export function validateScriptPath(path: string): ScriptPathError | null {
  const trimmed = path.trim();
  if (!trimmed) return { kind: 'empty' };
  const lower = trimmed.toLowerCase();
  if (!lower.endsWith('.bat') && !lower.endsWith('.cmd')) {
    return { kind: 'bad_extension' };
  }
  const bad = [...trimmed].find(isUnsafePathChar);
  if (bad === undefined) return null;
  return (CMD_METACHARACTERS as readonly string[]).includes(bad)
    ? { kind: 'metacharacter', char: bad }
    : { kind: 'unsafe_character', char: bad };
}

/**
 * Pure decision for the inspector's launcher-path edit: an empty path clears
 * the launcher, a valid path may be persisted, an invalid one must be kept out
 * of the draft so the previous valid value stays authoritative.
 */
export function canCommitLauncherPath(path: string): boolean {
  const trimmed = path.trim();
  return trimmed === '' || validateScriptPath(trimmed) === null;
}

/** Whether a path names cmd.exe (basename, case-insensitive). */
export function isCmdExecutable(path: string): boolean {
  const name = path.split(/[\\/]/).pop() ?? '';
  return name.toLowerCase() === 'cmd.exe';
}

/** Whether a window class is the console window class. */
export function isConsoleClass(className: string): boolean {
  return className.toLowerCase() === 'consolewindowclass';
}

/** Whether a process image path is conhost.exe. */
export function isConhostPath(path: string): boolean {
  const name = path.split(/[\\/]/).pop() ?? '';
  return name.toLowerCase() === 'conhost.exe';
}

/** Short display name for a launcher path (the file name). */
export function scriptDisplayName(path: string): string {
  return path.trim().split(/[\\/]/).pop() || path.trim();
}
