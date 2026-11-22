import type { AppEntryDto } from '../api/client';

export type AppSource = 'installed' | 'running' | 'browse';

/**
 * B-3: a native Browse pick must not switch the active tab. Mutating `source`
 * would re-trigger the tab's load effect and overwrite the picked entries with
 * the installed/running list. Returns the source to keep after a browse pick.
 */
export function sourceAfterBrowse<S extends AppSource>(current: S): S {
  return current;
}

/**
 * B-6: entering the Browse tab must show only an explicit file-picker result —
 * never the previously loaded installed/running list. Returns the apps to show
 * on tab entry, or `null` when the tab should load its list from the API.
 */
export function appsOnTabEnter(next: AppSource): AppEntryDto[] | null {
  return next === 'browse' ? [] : null;
}
