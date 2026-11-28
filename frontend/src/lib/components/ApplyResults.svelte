<script lang="ts">
  /** Dismissible per-zone results panel (Finding 3). Aggregate toast is unchanged. */
  import { X } from '@lucide/svelte';
  import { applyResults, dismissApplyResults } from '$lib/state/apply-results.svelte';
  import { Badge } from '$lib/components/ui/badge/index.js';

  // Official Badge has no success/warning/error variants; keep the tinted pills.
  const toneClass: Record<'success' | 'warning' | 'error', string> = {
    success: 'bg-success/15 text-success',
    warning: 'bg-warning/15 text-warning',
    error: 'bg-destructive/15 text-destructive',
  };
</script>

{#if applyResults.visible}
  <section
    class="border-t border-zf-border bg-zf-raised px-4 py-2"
    aria-label="Per-zone apply results"
  >
    <div class="mb-1.5 flex items-center justify-between gap-2">
      <p class="text-xs font-semibold uppercase tracking-wider text-zf-muted">
        Zone results · {applyResults.profile}
        {#if applyResults.intent === 'launch_arrange'}
          <span class="font-normal normal-case tracking-normal text-zf-faint">(Launch &amp; Arrange)</span>
        {/if}
      </p>
      <button
        type="button"
        aria-label="Dismiss zone results"
        class="inline-flex h-6 w-6 items-center justify-center rounded p-0.5 text-zf-faint hover:bg-zf-raised hover:text-zf-text"
        onclick={dismissApplyResults}
      >
        <X size={13} />
      </button>
    </div>
    <div class="flex flex-col gap-1">
      {#each applyResults.rows as row (row.zoneId)}
        <div class="flex flex-wrap items-start gap-x-2 gap-y-0.5 text-xs">
          <span class="min-w-0 font-medium text-zf-text">{row.name}</span>
          <Badge class={`rounded-full text-xs ${toneClass[row.tone]}`}>{row.outcome}</Badge>
          {#if row.reason}
            <span class="min-w-0 flex-1 text-zf-muted">{row.reason}</span>
          {/if}
        </div>
      {/each}
    </div>
  </section>
{/if}
