<script lang="ts">
  import { AlertTriangle } from '@lucide/svelte';
  import { store } from '$lib/state/app.svelte';
  import { duplicateAssignments } from '$lib/state/workflow';

  interface Props {
    selectedId: number | null;
  }
  let { selectedId }: Props = $props();

  const zones = $derived(store.snapshot?.zones ?? []);
  const workArea = $derived(store.snapshot?.status.work_area ?? null);
  const enabled = $derived(store.snapshot?.enabled ?? false);
  const duplicates = $derived(duplicateAssignments(zones));
  // Same 12×8 fallback as ZoneCanvas.svelte:72-75 when settings are absent.
  const gridCols = $derived(store.snapshot?.settings?.grid_cols ?? 12);
  const gridRows = $derived(store.snapshot?.settings?.grid_rows ?? 8);
  const connectionLabel = $derived(
    store.connection === 'connected'
      ? 'connected'
      : store.connection === 'lost'
        ? 'connection lost'
        : store.connection === 'reconnecting'
          ? 'reconnecting'
          : 'connecting',
  );
  // Tinted status pill using the repo's semantic tokens (moved from TopBar).
  const connectionClass = $derived(
    store.connection === 'connected'
      ? 'bg-success/15 text-success'
      : store.connection === 'lost'
        ? 'bg-destructive/15 text-destructive'
        : 'bg-warning/15 text-warning',
  );
</script>

<footer class="flex flex-wrap items-center gap-x-5 gap-y-2 shadow-zf-status bg-zf-surface px-5 py-2 text-xs text-zf-muted">
  <span><span class="text-zf-faint">zones</span> <span class="font-mono text-zf-text">{zones.length}</span></span>
  <span><span class="text-zf-faint">work area</span> <span class="font-mono text-zf-text">{workArea ? `${workArea.w}×${workArea.h}` : '—'}</span></span>
  <span><span class="text-zf-faint">grid</span> <span class="font-mono text-zf-text">{gridCols}×{gridRows}</span></span>
  <span><span class="text-zf-faint">selected</span> <span class="font-mono text-zf-text">{selectedId ?? '—'}</span></span>
  <span><span class="text-zf-faint">layout</span> <span class={enabled ? 'text-zf-ok' : 'text-zf-error'}>{enabled ? 'enabled' : 'disabled'}</span></span>

  {#if duplicates.length}
    <span class="flex items-center gap-1 text-zf-warn" title="Only one window can be placed per app; the last zone wins.">
      <AlertTriangle size={13} />
      {duplicates.length} duplicate app assignment{duplicates.length > 1 ? 's' : ''}
    </span>
  {/if}

  <span class="ml-auto inline-flex items-center gap-1.5 rounded-full px-2 py-0.5 text-xs {connectionClass}">
    <span aria-hidden="true">●</span> {connectionLabel}
  </span>
</footer>
