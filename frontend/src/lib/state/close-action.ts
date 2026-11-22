/**
 * Shared close-action logic used by both the CloseDialog and the top bar's
 * remembered-close path. Rune-free and DOM-global-free so bun can exercise it
 * with injected dependencies — no component-test framework needed.
 *
 * Relative imports are deliberate: bun's test runner resolves them without
 * relying on the `$lib` alias from tsconfig.app.json.
 */
import { api, type AppOptionsDto } from '../api/client';
import type { CloseDecision } from './settings-helpers';

/** The slice of the API the persist step needs (injectable for tests). */
export interface ClosePersistApi {
  setAppOptions(options: AppOptionsDto): Promise<unknown>;
  save(): Promise<unknown>;
}

/** Draft values carried into the persisted AppOptions DTO. */
export interface CloseDraftValues {
  auto_start: boolean;
  gap: number;
}

export interface PersistCloseOptions {
  client?: ClosePersistApi;
  onError?: (error: unknown) => void;
}

/**
 * Persist the close choice and the whole draft to disk. When `remember` is
 * true the chosen action is stored; otherwise `close_action` is reset to
 * `ask`. Returns false on failure so callers never discard data on a failed
 * write — the optional `onError` hook lets them surface a message.
 */
export async function persistCloseChoice(
  draft: CloseDraftValues,
  action: 'minimize' | 'exit',
  remember: boolean,
  options: PersistCloseOptions = {},
): Promise<boolean> {
  const client = options.client ?? api;
  try {
    await client.setAppOptions({
      auto_start: draft.auto_start,
      close_action: remember ? action : 'ask',
      gap: draft.gap,
    });
    await client.save();
    return true;
  } catch (error) {
    options.onError?.(error);
    return false;
  }
}

export type RememberedCloseResult = 'dialog' | 'minimize' | 'exit' | 'persist_failed';

export interface RememberedCloseDeps {
  persist: (action: 'minimize' | 'exit') => Promise<boolean>;
  closeWindow: () => void;
  shutdown: () => Promise<void>;
}

/**
 * Execute the decision for the editor's close button. `dialog` is returned
 * untouched so the caller can open the confirmation dialog; the two remembered
 * choices persist first and only then close/shut down. A failed persist aborts
 * the action so a dirty draft is never lost.
 */
export async function performRememberedClose(
  action: CloseDecision,
  deps: RememberedCloseDeps,
): Promise<RememberedCloseResult> {
  if (action === 'dialog') return 'dialog';
  if (!(await deps.persist(action))) return 'persist_failed';
  if (action === 'minimize') {
    deps.closeWindow();
  } else {
    await deps.shutdown();
  }
  return action;
}
