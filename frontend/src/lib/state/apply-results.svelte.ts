/**
 * UI store for the last apply / launch-arrange result (Finding 3).
 *
 * Thin rune wrapper around the pure projection in `apply-results.ts`. The
 * aggregate toast stays in `actions.ts`; this store only feeds the dismissible
 * per-zone results panel. Svelte 5 module rule: export one non-reassigned
 * `$state` object and mutate its properties.
 */

import type { ApplyIntent, ApplyResultDto } from '../api/client';
import { toOutcomeRows, type OutcomeRow } from './apply-results';

interface ApplyResultsShape {
  visible: boolean;
  profile: string;
  intent: ApplyIntent | null;
  rows: OutcomeRow[];
}

export const applyResults = $state<ApplyResultsShape>({
  visible: false,
  profile: '',
  intent: null,
  rows: [],
});

/** Replace the displayed rows with this operation's per-zone outcomes. */
export function showApplyResults(result: ApplyResultDto): void {
  applyResults.rows = toOutcomeRows(result);
  applyResults.profile = result.profile;
  applyResults.intent = result.intent;
  applyResults.visible = applyResults.rows.length > 0;
}

/** Hide the panel without clearing the rows (cheap dismiss). */
export function dismissApplyResults(): void {
  applyResults.visible = false;
}
