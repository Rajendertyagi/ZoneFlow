/**
 * Pure launcher validation helpers. These mirror the backend's
 * `engine_core.rs` rules so an invalid script is rejected in the picker before
 * it is persisted. No Svelte, no filesystem, no shell.
 */
import { describe, expect, test } from 'bun:test';
import {
  CMD_METACHARACTERS,
  canCommitLauncherPath,
  isCmdExecutable,
  isConsoleClass,
  isConhostPath,
  scriptDisplayName,
  scriptPathErrorMessage,
  validateScriptPath,
} from '../src/lib/state/launcher';

describe('validateScriptPath', () => {
  test('accepts .bat and .cmd case-insensitively with surrounding whitespace', () => {
    expect(validateScriptPath('C:\\Scripts\\run.bat')).toBeNull();
    expect(validateScriptPath('C:\\Scripts\\RUN.CMD')).toBeNull();
    expect(validateScriptPath('  C:\\Scripts\\run.cmd  ')).toBeNull();
  });

  test('rejects empty paths and non-script extensions', () => {
    expect(validateScriptPath('   ')).toEqual({ kind: 'empty' });
    expect(validateScriptPath('C:\\Scripts\\run.exe')).toEqual({ kind: 'bad_extension' });
    expect(validateScriptPath('C:\\Scripts\\run')).toEqual({ kind: 'bad_extension' });
  });

  test('rejects every CMD metacharacter with a clear message', () => {
    for (const c of CMD_METACHARACTERS) {
      const error = validateScriptPath(`C:\\Scripts\\run${c}me.bat`);
      expect(error).toEqual({ kind: 'metacharacter', char: c });
      expect(scriptPathErrorMessage(error!)).toContain(c);
    }
  });

  test('rejects a double quote that would break cmd.exe quoting', () => {
    const error = validateScriptPath('C:\\Scripts\\run"me.bat');
    expect(error).toEqual({ kind: 'unsafe_character', char: '"' });
    expect(scriptPathErrorMessage(error!)).toBe('path contains a double quote');
  });

  test('rejects control characters without embedding them in the message', () => {
    for (const c of ['\n', '\r', '\0', '\u001b']) {
      const error = validateScriptPath(`C:\\Scripts\\run${c}me.bat`);
      expect(error).toEqual({ kind: 'unsafe_character', char: c });
      const message = scriptPathErrorMessage(error!);
      expect(message).toContain('control character U+');
      expect(message).not.toContain(c);
    }
  });

  test('all error kinds have non-empty human-readable messages', () => {
    for (const error of [
      { kind: 'empty' } as const,
      { kind: 'bad_extension' } as const,
      { kind: 'metacharacter', char: '&' } as const,
      { kind: 'unsafe_character', char: '"' } as const,
      { kind: 'unsafe_character', char: '\n' } as const,
    ]) {
      expect(scriptPathErrorMessage(error).length).toBeGreaterThan(0);
    }
  });
});

describe('canCommitLauncherPath', () => {
  test('allows clearing the launcher and valid paths', () => {
    expect(canCommitLauncherPath('')).toBe(true);
    expect(canCommitLauncherPath('   ')).toBe(true);
    expect(canCommitLauncherPath('C:\\Scripts\\run.bat')).toBe(true);
    expect(canCommitLauncherPath('C:\\Scripts\\RUN.CMD')).toBe(true);
  });

  test('blocks every invalid path so the inspector keeps the last valid value', () => {
    expect(canCommitLauncherPath('C:\\Scripts\\run.exe')).toBe(false);
    expect(canCommitLauncherPath('C:\\Scripts\\run&me.bat')).toBe(false);
    expect(canCommitLauncherPath('C:\\Scripts\\run"me.bat')).toBe(false);
    expect(canCommitLauncherPath('C:\\Scripts\\run\nme.bat')).toBe(false);
  });
});

describe('command interpreter and console recognition', () => {
  test('isCmdExecutable checks the basename only', () => {
    expect(isCmdExecutable('C:\\Windows\\System32\\cmd.exe')).toBe(true);
    expect(isCmdExecutable('C:\\Windows\\System32\\CMD.EXE')).toBe(true);
    expect(isCmdExecutable('/usr/bin/cmd.exe')).toBe(true);
    expect(isCmdExecutable('C:\\Windows\\System32\\cmd.exe.bak')).toBe(false);
    expect(isCmdExecutable('C:\\Tools\\notcmd.exe')).toBe(false);
    expect(isCmdExecutable('')).toBe(false);
  });

  test('console class and conhost path are recognized case-insensitively', () => {
    expect(isConsoleClass('ConsoleWindowClass')).toBe(true);
    expect(isConsoleClass('consolewindowclass')).toBe(true);
    expect(isConsoleClass('Chrome_WidgetWin_1')).toBe(false);
    expect(isConhostPath('C:\\Windows\\System32\\conhost.exe')).toBe(true);
    expect(isConhostPath('C:\\Windows\\System32\\CONHOST.EXE')).toBe(true);
    expect(isConhostPath('C:\\Windows\\System32\\cmd.exe')).toBe(false);
  });

  test('scriptDisplayName returns the file name', () => {
    expect(scriptDisplayName('C:\\Scripts\\run.bat')).toBe('run.bat');
    expect(scriptDisplayName('D:\\A B\\launch.cmd')).toBe('launch.cmd');
  });
});
