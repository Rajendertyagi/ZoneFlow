/**
 * High-level UI actions. Every function here talks to the real API, surfaces
 * the backend's own result (never a fabricated success) and refreshes the
 * authoritative snapshot. D1–D6 guards live here so every view behaves the same.
 */

import {
  api,
  ApiError,
  type ApplyResultDto,
  type AssignDto,
  type InactiveResult,
  type ZoneDto,
} from './api/client';
import { refreshState } from './api/events';
import { isDirty } from './state/app.svelte';
import { confirm, toast } from './state/ui.svelte';
import {
  errorMessage,
  isDirtyError,
  isInactiveResult,
  needsReplaceConfirmation,
} from './state/workflow';

/* ------------------------------ profile ops (D2/D6) ------------------------ */

export interface GuardedOpOptions {
  profile: string;
  /** Gerund used in the Discard/Cancel copy, e.g. "switch to". */
  action: string;
  successMessage?: string;
}

/**
 * Run a profile operation that the backend dirty-guards (load / select / apply).
 * On a 409 `dirty_state`, ask for an explicit Discard/Cancel and retry once with
 * `discard_dirty=true`. No state is changed on Cancel (D2).
 */
export async function guardedProfileOp(
  op: (discardDirty: boolean) => Promise<unknown>,
  options: GuardedOpOptions,
): Promise<boolean> {
  try {
    const result = await op(false);
    await refreshState();
    const inactive = isInactiveResult(result);
    reportInactive(result, options.action);
    // Never show a success toast for a disabled/inactive result (D4).
    if (!inactive && options.successMessage) toast('success', options.successMessage);
    return true;
  } catch (error) {
    if (!isDirtyError(error)) {
      toast('error', `${options.action} failed: ${errorMessage(error)}`);
      return false;
    }
    const discard = await confirm({
      title: 'Unsaved changes',
      description: `Unsaved changes in ${options.profile}. Discard them and ${options.action}?`,
      confirmLabel: 'Discard & continue',
      cancelLabel: 'Cancel',
      destructive: true,
    });
    if (!discard) {
      toast('info', 'Cancelled — your draft is unchanged.');
      return false;
    }
    try {
      const result = await op(true);
      await refreshState();
      reportInactive(result, options.action);
      toast('info', `Discarded unsaved changes in ${options.profile}.`);
      return true;
    } catch (retryError) {
      toast('error', `${options.action} failed: ${errorMessage(retryError)}`);
      return false;
    }
  }
}

function reportInactive(result: unknown, action: string): void {
  if (isInactiveResult(result)) {
    toast('info', `Layout is disabled — ${action} changed selection only.`);
  }
}

/* -------------------------------- assignment (D3) -------------------------- */

/** Assign or replace an app, confirming before resetting non-default rules. */
export async function assignAppGuarded(zone: ZoneDto, spec: AssignDto): Promise<boolean> {
  if (needsReplaceConfirmation(zone, spec)) {
    const replace = await confirm({
      title: 'Replace application',
      description: `${zone.label || zone.exe} has custom rules (launch mode, URL, title, tab behavior or macro). Replacing it resets those rules to defaults. The application stays in your library.`,
      confirmLabel: 'Replace & reset rules',
      cancelLabel: 'Cancel',
      destructive: true,
    });
    if (!replace) {
      toast('info', 'Assignment cancelled — the zone is unchanged.');
      return false;
    }
  }
  try {
    await api.assignZone(zone.id, spec);
    await refreshState();
    toast('success', `Assigned ${spec.label || spec.exe} to zone ${zone.id}.`);
    return true;
  } catch (error) {
    toast('error', `Assign failed: ${errorMessage(error)}`);
    return false;
  }
}

export async function unassignApp(zone: ZoneDto): Promise<void> {
  try {
    await api.unassignZone(zone.id);
    await refreshState();
    toast('success', `Cleared the assignment on zone ${zone.id}.`);
  } catch (error) {
    toast('error', `Unassign failed: ${errorMessage(error)}`);
  }
}

/* ------------------------------- core actions ------------------------------ */

export async function doSave(): Promise<boolean> {
  try {
    await api.save();
    await refreshState();
    toast('success', 'Saved the draft to this profile. Windows were not repositioned.');
    return true;
  } catch (error) {
    // Keep the draft and dirty badge intact — nothing is silently lost.
    toast('error', `Save failed: ${errorMessage(error)}`);
    return false;
  }
}

export async function doRevert(): Promise<void> {
  const ok = await confirm({
    title: 'Revert changes?',
    description: 'Discard all unsaved edits and reload this profile from disk?',
    confirmLabel: 'Revert',
    cancelLabel: 'Cancel',
    destructive: true,
  });
  if (!ok) return;
  try {
    await api.revert();
    await refreshState();
    toast('info', 'Reverted the draft to the saved profile.');
  } catch (error) {
    toast('error', `Revert failed: ${errorMessage(error)}`);
  }
}

/** D1: Apply = arrange running apps only; Launch & Arrange = explicit launch. */
export async function doApply(launch: boolean): Promise<void> {
  try {
    const result = launch ? await api.launchArrange() : await api.apply();
    await refreshState();
    if (isInactiveResult(result)) {
      toast('info', 'Layout is disabled — nothing was applied or launched.');
      return;
    }
    const applied = result as ApplyResultDto;
    // Show the backend's own per-zone message verbatim — no simulated success.
    toast(
      'success',
      `${launch ? 'Launch & Arrange' : 'Apply'} · ${applied.profile}: ${applied.message}`,
    );
  } catch (error) {
    if (error instanceof ApiError && error.code === 'dirty_state') {
      toast('error', 'Save or revert your changes before applying.');
    } else {
      toast('error', `${launch ? 'Launch & Arrange' : 'Apply'} failed: ${errorMessage(error)}`);
    }
  }
}

/** D5: inspect the steps without running anything. */
export async function doPreview(): Promise<void> {
  try {
    const result = await api.preview();
    await refreshState();
    const lines = result.steps.length ? result.steps.join('\n') : '(no zones in this profile)';
    toast('info', `Preview · ${result.profile} (not executed)\n${lines}`);
  } catch (error) {
    toast('error', `Preview failed: ${errorMessage(error)}`);
  }
}

export async function doDisable(): Promise<void> {
  try {
    await api.disable();
    await refreshState();
    toast('info', 'Layout disabled — automation suspended. Profiles can still be selected.');
  } catch (error) {
    toast('error', `Disable failed: ${errorMessage(error)}`);
  }
}

export async function doEnable(): Promise<void> {
  const dirty = isDirty();
  try {
    await api.enable();
    await refreshState();
    // The backend deliberately skips re-apply when the draft is dirty (D2).
    toast(
      'success',
      dirty
        ? 'Layout enabled — re-apply was skipped because the draft has unsaved changes.'
        : 'Layout enabled — the selected profile was re-applied.',
    );
  } catch (error) {
    toast('error', `Enable failed: ${errorMessage(error)}`);
  }
}

/* ------------------------------ profile CRUD ------------------------------- */

/**
 * §2a: destructive profile operations must apply the D2 Discard/Cancel rule
 * when the draft is dirty. The backend does not guard these three operations,
 * so the guard lives here — otherwise deleting a profile could silently lose
 * its unsaved edits.
 */
async function confirmDiscardIfDirty(profile: string, action: string): Promise<boolean> {
  if (!isDirty()) return true;
  const discard = await confirm({
    title: 'Unsaved changes',
    description: `Unsaved changes in ${profile}. Discard them and ${action}?`,
    confirmLabel: 'Discard & continue',
    cancelLabel: 'Cancel',
    destructive: true,
  });
  if (!discard) {
    toast('info', 'Cancelled — your draft is unchanged.');
    return false;
  }
  try {
    await api.revert();
    await refreshState();
  } catch (error) {
    toast('error', `Could not discard the draft: ${errorMessage(error)}`);
    return false;
  }
  return true;
}

export async function createProfile(name: string): Promise<boolean> {
  try {
    await api.createProfile(name);
    await refreshState();
    toast('success', `Created profile ${name}.`);
    return true;
  } catch (error) {
    toast('error', `Create failed: ${errorMessage(error)}`);
    return false;
  }
}

export async function renameProfile(name: string, newName: string): Promise<boolean> {
  if (!(await confirmDiscardIfDirty(name, `rename it to ${newName}`))) return false;
  try {
    await api.renameProfile(name, newName);
    await refreshState();
    toast('success', `Renamed ${name} to ${newName}.`);
    return true;
  } catch (error) {
    toast('error', `Rename failed: ${errorMessage(error)}`);
    return false;
  }
}

export async function duplicateProfile(name: string, newName: string): Promise<boolean> {
  if (!(await confirmDiscardIfDirty(name, `duplicate it as ${newName}`))) return false;
  try {
    await api.duplicateProfile(name, newName);
    await refreshState();
    toast('success', `Duplicated ${name} as ${newName}.`);
    return true;
  } catch (error) {
    toast('error', `Duplicate failed: ${errorMessage(error)}`);
    return false;
  }
}

export async function deleteProfile(name: string): Promise<boolean> {
  if (!(await confirmDiscardIfDirty(name, `delete ${name}`))) return false;
  const ok = await confirm({
    title: 'Delete profile?',
    description: `Delete ${name} and all of its zones and assignments? This cannot be undone.`,
    confirmLabel: 'Delete',
    cancelLabel: 'Cancel',
    destructive: true,
  });
  if (!ok) return false;
  try {
    await api.deleteProfile(name);
    await refreshState();
    toast('success', `Deleted profile ${name}.`);
    return true;
  } catch (error) {
    toast('error', `Delete failed: ${errorMessage(error)}`);
    return false;
  }
}
