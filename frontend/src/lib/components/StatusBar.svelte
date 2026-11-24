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
  const assignments = $derived(zones.filter((z) => z.exe.trim() !== ''));
</script>

<footer class="flex flex-wrap items-center gap-x-5 gap-y-2 border-t border-zf-border bg-zf-surface px-5 py-2 text-xs text-zf-muted">
  <span><span class="text-zf-faint">zones</span> <span class="font-mono text-zf-text">{zones.length}</span></span>
  <span><span class="text-zf-faint">work area</span> <span class="font-mono text-zf-text">{workArea ? `${workArea.w}×${workArea.h}` : '—'}</span></span>
  <span><span class="text-zf-faint">selected</span> <span class="font-mono text-zf-text">{selectedId ?? '—'}</span></span>
  <span><span class="text-zf-faint">layout</span> <span class={enabled ? 'text-zf-ok' : 'text-zf-error'}>{enabled ? 'enabled' : 'disabled'}</span></span>

  {#if duplicates.length}
    <span class="flex items-center gap-1 text-zf-warn" title="Only one window can be placed per app; the last zone wins.">
      <AlertTriangle size={13} />
      {duplicates.length} duplicate app assignment{duplicates.length > 1 ? 's' : ''}
    </span>
  {/if}

  <span class="ml-auto flex min-w-0 flex-wrap items-center gap-1.5">
    <span class="text-zf-faint">apps in this workspace</span>
    {#each assignments as zone (zone.id)}
      <span class="inline-flex items-center rounded-full bg-zf-raised px-1.5 py-0.5 text-[0.7rem] text-zf-text">
        {zone.label || zone.exe} <span class="text-zf-faint">→ {zone.id}</span>
      </span>
    {:else}
      <span class="text-zf-faint">none assigned</span>
    {/each}
  </span>
</footer>
