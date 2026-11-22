/**
 * Tests for the canvas command registry that bridges App.svelte's global
 * keydown handler to ZoneCanvas.svelte's undo/redo functions.
 */
import { beforeEach, describe, expect, test } from 'bun:test';
import {
  getCanvasCommands,
  registerCanvasCommands,
  unregisterCanvasCommands,
} from '../src/lib/state/canvas-commands';

describe('canvas command registry', () => {
  beforeEach(() => registerCanvasCommands(null));

  test('starts empty', () => {
    expect(getCanvasCommands()).toBeNull();
  });

  test('registers and retrieves commands', () => {
    const commands = { undo: () => {}, redo: () => {} };
    registerCanvasCommands(commands);
    expect(getCanvasCommands()).toBe(commands);
  });

  test('unregister clears only the active registration', () => {
    const first = { undo: () => {}, redo: () => {} };
    const second = { undo: () => {}, redo: () => {} };
    registerCanvasCommands(first);
    registerCanvasCommands(second);
    // A stale teardown from the first instance must not wipe out the newer one.
    unregisterCanvasCommands(first);
    expect(getCanvasCommands()).toBe(second);
    unregisterCanvasCommands(second);
    expect(getCanvasCommands()).toBeNull();
  });

  test('registered commands are the ones actually invoked', () => {
    let undone = 0;
    let redone = 0;
    registerCanvasCommands({
      undo: () => {
        undone += 1;
      },
      redo: () => {
        redone += 1;
      },
    });
    getCanvasCommands()?.undo();
    getCanvasCommands()?.redo();
    expect(undone).toBe(1);
    expect(redone).toBe(1);
  });
});
