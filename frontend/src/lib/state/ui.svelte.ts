/**
 * UI-only state: navigation, toasts and the two global modal services
 * (confirm / prompt). Kept separate from the authoritative snapshot mirror in
 * `app.svelte.ts` so backend state stays the single source of truth.
 *
 * Svelte 5 module rule: export one non-reassigned `$state` object and mutate its
 * properties; promise resolvers live in module-level variables (not proxied).
 */

export type View = 'home' | 'editor' | 'hotkeys' | 'settings';
export type ToastKind = 'success' | 'error' | 'info' | 'warning';

export interface Toast {
  id: number;
  kind: ToastKind;
  message: string;
}

export interface ConfirmOptions {
  title: string;
  description?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  destructive?: boolean;
}

export interface PromptOptions {
  title: string;
  description?: string;
  label?: string;
  initialValue?: string;
  confirmLabel?: string;
  placeholder?: string;
}

interface ConfirmRequest extends ConfirmOptions {
  id: number;
}
interface PromptRequest extends PromptOptions {
  id: number;
}

interface UiShape {
  view: View;
  canvasMode: 'layout' | 'draw';
  toasts: Toast[];
  confirmRequest: ConfirmRequest | null;
  promptRequest: PromptRequest | null;
}

export const ui = $state<UiShape>({
  view: 'editor',
  canvasMode: 'layout',
  toasts: [],
  confirmRequest: null,
  promptRequest: null,
});

let nextId = 1;
let confirmResolve: ((value: boolean) => void) | null = null;
let promptResolve: ((value: string | null) => void) | null = null;

/* ------------------------------- navigation -------------------------------- */

export function navigate(view: View): void {
  ui.view = view;
}

/* --------------------------------- toasts ---------------------------------- */

export function toast(kind: ToastKind, message: string): void {
  const id = nextId++;
  ui.toasts.push({ id, kind, message });
  if (typeof window !== 'undefined') {
    window.setTimeout(() => dismissToast(id), 4600);
  }
}

export function dismissToast(id: number): void {
  const index = ui.toasts.findIndex((t) => t.id === id);
  if (index >= 0) ui.toasts.splice(index, 1);
}

/* --------------------------------- confirm --------------------------------- */

/** Show the global shadcn-style confirm dialog. Resolves true on confirm. */
export function confirm(options: ConfirmOptions): Promise<boolean> {
  confirmResolve?.(false); // never strand a previous waiter
  ui.confirmRequest = { ...options, id: nextId++ };
  return new Promise<boolean>((resolve) => {
    confirmResolve = resolve;
  });
}

export function resolveConfirm(value: boolean): void {
  ui.confirmRequest = null;
  const resolve = confirmResolve;
  confirmResolve = null;
  resolve?.(value);
}

/* --------------------------------- prompt ---------------------------------- */

/** Show the global text-entry dialog. Resolves null on cancel. */
export function prompt(options: PromptOptions): Promise<string | null> {
  promptResolve?.(null);
  ui.promptRequest = { ...options, id: nextId++ };
  return new Promise<string | null>((resolve) => {
    promptResolve = resolve;
  });
}

export function resolvePrompt(value: string | null): void {
  ui.promptRequest = null;
  const resolve = promptResolve;
  promptResolve = null;
  resolve?.(value);
}
