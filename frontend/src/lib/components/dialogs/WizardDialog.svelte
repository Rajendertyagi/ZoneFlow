<script lang="ts">
  import { PenTool, Sparkles } from '@lucide/svelte';
  import { api } from '$lib/api/client';
  import { refreshState } from '$lib/api/events';
  import { store } from '$lib/state/app.svelte';
  import { confirm, toast } from '$lib/state/ui.svelte';
  import { countAssignedZonesToRemove } from '$lib/state/wizard';
  import { errorMessage } from '$lib/state/workflow';
  import { blankZone } from '$lib/components/Canvas/geometry';
  import {
    TEMPLATE_OPTIONS,
    templateLabel,
    templateRects,
    type TemplateId,
  } from '$lib/components/Canvas/templates';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Label } from '$lib/components/ui/label/index.js';
  import { Switch } from '$lib/components/ui/switch/index.js';

  const focusRing = 'focus-visible:outline-2 focus-visible:outline focus-visible:outline-offset-2 focus-visible:outline-primary focus-visible:ring-0 motion-reduce:transition-none';

  interface Props {
    open: boolean;
    onclose: () => void;
    /** Switch the canvas into draw mode and close the wizard. */
    ondraw: () => void;
  }
  let { open, onclose, ondraw }: Props = $props();

  let template = $state<TemplateId>('duo');
  let cols = $state(2);
  let rows = $state(2);
  let replace = $state(true);
  let busy = $state(false);

  const tints = $derived(store.snapshot?.settings.zone_tints ?? ['#F5B544', '#6EA8FE', '#5BD6A0', '#C792EA']);
  const rects = $derived(templateRects(template, cols, rows));
  const templateName = $derived(TEMPLATE_OPTIONS.find((t) => t.id === template)?.label ?? 'Template');

  async function apply(): Promise<void> {
    if (!rects.length || busy) return;
    busy = true;
    try {
      const existing = store.snapshot?.zones ?? [];
      if (replace && existing.length > 0) {
        // Reuse the first zone (the backend refuses deleting the last one), then
        // remove the rest and create the remaining template zones.
        const keep = existing[0];
        // U-09: warn before silently dropping application assignments on the
        // zones that are about to be deleted. The kept zone keeps its exe/path.
        const assignedToRemove = countAssignedZonesToRemove(existing, keep.id);
        if (assignedToRemove > 0) {
          const ok = await confirm({
            title: 'Replace assigned zones?',
            description: `This template will delete ${assignedToRemove} assigned zone${assignedToRemove === 1 ? '' : 's'}, removing their application assignments. This cannot be undone. Continue?`,
            confirmLabel: 'Replace zones',
            cancelLabel: 'Cancel',
            destructive: true,
          });
          if (!ok) return;
        }
        await api.updateZone(keep.id, {
          ...keep,
          ...rects[0],
          label: templateLabel(template, 0),
        });
        for (const zone of existing.slice(1)) await api.deleteZone(zone.id);
        for (let i = 1; i < rects.length; i += 1) {
          await api.createZone(blankZone(rects[i], templateLabel(template, i)));
        }
      } else {
        for (let i = 0; i < rects.length; i += 1) {
          await api.createZone(blankZone(rects[i], templateLabel(template, i)));
        }
      }
      await refreshState();
      toast('success', `Applied the ${rects.length}-zone ${templateName} template.`);
      onclose();
    } catch (error) {
      toast('error', `Template failed: ${errorMessage(error)}`);
      await refreshState();
    } finally {
      busy = false;
    }
  }

  function draw(): void {
    onclose();
    ondraw();
  }
</script>

<Dialog.Root
  open={open}
  onOpenChange={(next) => {
    if (!next) onclose();
  }}
>
  <Dialog.Content
    showCloseButton={false}
    aria-label="Layout wizard"
    class="flex max-h-[90vh] max-w-3xl flex-col gap-0 overflow-hidden rounded-lg border border-zf-border-strong bg-zf-panel p-0 shadow-2xl ring-0 sm:max-w-3xl"
  >
    <Dialog.Header class="flex flex-row items-center gap-2 border-b border-zf-border px-6 py-4">
      <Sparkles size={18} class="text-zf-accent" />
      <div class="min-w-0">
        <Dialog.Title class="text-base font-semibold text-zf-text">Layout wizard</Dialog.Title>
        <Dialog.Description class="text-xs text-zf-muted">Start from a template, or draw zones yourself.</Dialog.Description>
      </div>
    </Dialog.Header>

      <div class="grid min-h-0 flex-1 gap-5 overflow-auto p-6 md:grid-cols-[1fr_260px]">
        <!-- Template picker -->
        <div class="flex flex-col gap-2">
          <Label class="text-xs text-muted-foreground">Templates</Label>
          <div class="grid grid-cols-2 gap-2 sm:grid-cols-3">
            {#each TEMPLATE_OPTIONS as option (option.id)}
              <button
                type="button"
                aria-pressed={template === option.id}
                onclick={() => (template = option.id)}
                class="rounded-md border px-3 py-2 text-left transition-colors aria-pressed:border-zf-accent aria-pressed:bg-zf-accent/10 border-zf-border bg-zf-surface hover:border-zf-border-strong"
              >
                <span class="block text-sm font-medium {template === option.id ? 'text-zf-accent' : 'text-zf-text'}">
                  {option.label}
                </span>
                <span class="mt-0.5 block text-xs text-zf-faint">{option.hint}</span>
              </button>
            {/each}
          </div>

          {#if template === 'grid'}
            <div class="mt-3 flex items-end gap-3">
              <div class="flex-1">
                <Label for="wizard-cols" class="text-xs text-muted-foreground">Columns</Label>
                <input
                  id="wizard-cols"
                  type="number"
                  min="1"
                  max="12"
                  bind:value={cols}
                  class="h-9 w-full rounded-md border border-zf-border bg-zf-surface px-3 text-sm text-zf-text focus-visible:border-zf-accent focus-visible:outline-none"
                />
              </div>
              <div class="flex-1">
                <Label for="wizard-rows" class="text-xs text-muted-foreground">Rows</Label>
                <input
                  id="wizard-rows"
                  type="number"
                  min="1"
                  max="12"
                  bind:value={rows}
                  class="h-9 w-full rounded-md border border-zf-border bg-zf-surface px-3 text-sm text-zf-text focus-visible:border-zf-accent focus-visible:outline-none"
                />
              </div>
            </div>
          {/if}

          <div class="mt-4 flex items-center justify-between rounded-md border border-zf-border bg-zf-surface px-3 py-2">
            <span class="text-xs text-zf-muted">Replace the current zones</span>
            <Switch bind:checked={replace} aria-label="Replace current zones" class={focusRing} />
          </div>
        </div>

        <!-- Live preview -->
        <div class="flex flex-col gap-2">
          <Label class="text-xs text-muted-foreground">Live preview · {rects.length} zones</Label>
          <div class="aspect-video w-full overflow-hidden rounded-md border border-zf-border bg-zf-bg">
            <svg viewBox="0 0 100 100" preserveAspectRatio="none" class="h-full w-full" role="img" aria-label="Template preview">
              {#each rects as rect, i (i)}
                <rect
                  x={rect.x}
                  y={rect.y}
                  width={rect.w}
                  height={rect.h}
                  fill={tints[i % Math.max(1, tints.length)] ?? 'var(--canvas-preview)'}
                  fill-opacity="0.22"
                  stroke={tints[i % Math.max(1, tints.length)] ?? 'var(--canvas-preview)'}
                  stroke-width="0.6"
                  vector-effect="non-scaling-stroke"
                />
                <text
                  x={rect.x + rect.w / 2}
                  y={rect.y + rect.h / 2}
                  fill="var(--canvas-label-fg)"
                  font-size="3.2"
                  text-anchor="middle"
                  dominant-baseline="middle"
                >
                  {i + 1}
                </text>
              {/each}
            </svg>
          </div>
          <p class="text-xs leading-relaxed text-zf-faint">
            Geometry is stored as percentages, so the layout adapts to any resolution.
          </p>
        </div>
      </div>

    <Dialog.Footer
      class="mx-0 mb-0 flex flex-wrap items-center justify-between gap-2 rounded-none border-zf-border bg-transparent p-0 px-6 py-4 sm:justify-between"
    >
      <Button variant="ghost" size="sm" class={`h-7 text-xs ${focusRing}`} onclick={draw}>
        <PenTool size={14} /> Draw manually instead
      </Button>
      <div class="flex gap-2">
        <Button variant="ghost" size="sm" class={`h-7 text-xs ${focusRing}`} onclick={onclose}>Cancel</Button>
        <Button size="sm" disabled={busy || rects.length === 0} class={`h-9 px-4 ${focusRing}`} onclick={() => void apply()}>
          Apply template
        </Button>
      </div>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
