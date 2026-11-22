/**
 * Tests for the shared close-action logic in `close-action.ts`.
 *
 * Both helpers take injected dependencies, so the remembered close paths can be
 * verified without a component framework, a real API or a real browser window.
 */
import { describe, expect, test } from 'bun:test';
import {
  performRememberedClose,
  persistCloseChoice,
  type ClosePersistApi,
  type RememberedCloseDeps,
} from '../src/lib/state/close-action';

function fakeApi(overrides: Partial<ClosePersistApi> = {}) {
  const calls: Array<{ method: string; arg?: unknown }> = [];
  const client: ClosePersistApi = {
    setAppOptions: async (options) => {
      calls.push({ method: 'setAppOptions', arg: options });
    },
    save: async () => {
      calls.push({ method: 'save' });
    },
    ...overrides,
  };
  return { client, calls };
}

function fakeDeps(overrides: Partial<RememberedCloseDeps> = {}) {
  const events: string[] = [];
  // Wrap every dependency so calls are recorded even when a test overrides it.
  const deps: RememberedCloseDeps = {
    persist: async (action) => {
      events.push('persist');
      return overrides.persist ? overrides.persist(action) : true;
    },
    closeWindow: () => {
      events.push('closeWindow');
      overrides.closeWindow?.();
    },
    shutdown: async () => {
      events.push('shutdown');
      if (overrides.shutdown) await overrides.shutdown();
    },
  };
  return { deps, events };
}

describe('persistCloseChoice', () => {
  test('stores the remembered action and saves the whole draft', async () => {
    const { client, calls } = fakeApi();
    const ok = await persistCloseChoice(
      { auto_start: true, gap: 12 },
      'exit',
      true,
      { client },
    );
    expect(ok).toBe(true);
    expect(calls).toEqual([
      { method: 'setAppOptions', arg: { auto_start: true, close_action: 'exit', gap: 12 } },
      { method: 'save' },
    ]);
  });

  test('resets close_action to ask when the choice is not remembered', async () => {
    const { client, calls } = fakeApi();
    await persistCloseChoice({ auto_start: false, gap: 8 }, 'minimize', false, { client });
    expect(calls[0].arg).toEqual({ auto_start: false, close_action: 'ask', gap: 8 });
  });

  test('returns false and reports the error when persistence fails', async () => {
    let reported: unknown = null;
    const { client, calls } = fakeApi({
      setAppOptions: async () => {
        throw new Error('disk full');
      },
    });
    const ok = await persistCloseChoice(
      { auto_start: false, gap: 8 },
      'exit',
      true,
      { client, onError: (error) => { reported = error; } },
    );
    expect(ok).toBe(false);
    expect((reported as Error).message).toBe('disk full');
    expect(calls.some((c) => c.method === 'save')).toBe(false);
  });
});

describe('performRememberedClose', () => {
  test('ask opens the dialog without persisting, closing or shutting down', async () => {
    const { deps, events } = fakeDeps();
    expect(await performRememberedClose('dialog', deps)).toBe('dialog');
    expect(events).toEqual([]);
  });

  test('remembered minimize persists then closes the window, no shutdown', async () => {
    const { deps, events } = fakeDeps();
    expect(await performRememberedClose('minimize', deps)).toBe('minimize');
    expect(events).toEqual(['persist', 'closeWindow']);
  });

  test('remembered exit persists then shuts down, no window close', async () => {
    const { deps, events } = fakeDeps();
    expect(await performRememberedClose('exit', deps)).toBe('exit');
    expect(events).toEqual(['persist', 'shutdown']);
  });

  test('failed persist prevents both close and shutdown', async () => {
    const exit = fakeDeps({ persist: async () => false });
    expect(await performRememberedClose('exit', exit.deps)).toBe('persist_failed');
    expect(exit.events).toEqual(['persist']);

    const minimize = fakeDeps({ persist: async () => false });
    expect(await performRememberedClose('minimize', minimize.deps)).toBe('persist_failed');
    expect(minimize.events).toEqual(['persist']);
  });

  test('persist receives the chosen action', async () => {
    const seen: string[] = [];
    const { deps } = fakeDeps({
      persist: async (action) => {
        seen.push(action);
        return true;
      },
    });
    await performRememberedClose('exit', deps);
    await performRememberedClose('minimize', deps);
    expect(seen).toEqual(['exit', 'minimize']);
  });
});
