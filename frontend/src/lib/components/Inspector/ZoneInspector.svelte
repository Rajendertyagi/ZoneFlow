<script lang="ts">
  import { Ban, Terminal, X } from '@lucide/svelte';
  import { api, type ZoneDto } from '$lib/api/client';
  import { refreshState } from '$lib/api/events';
  import { unassignApp } from '$lib/actions';
  import { store } from '$lib/state/app.svelte';
  import { toast } from '$lib/state/ui.svelte';
  import { errorMessage, stripAppIdentity } from '$lib/state/workflow';
  import Badge from '$lib/components/ui/badge.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import Label from '$lib/components/ui/label.svelte';
  import Select from '$lib/components/ui/select.svelte';
  import Switch from '$lib/components/ui/switch.svelte';
  import Tabs from '$lib/components/ui/tabs.svelte';

  interface Props {
    zone: ZoneDto | null;
    onclose: () => void;
  }
  let { zone, onclose }: Props = $props();

  let tab = $state('process');
  let busy = $state(false);

  const logs = $derived(store.snapshot?.logs ?? []);

  async function commit(patch: Partial<ZoneDto>): Promise<void> {
    if (!zone || busy) return;
    busy = true;
    try {
      // App identity is display-only here; the guarded picker owns exe/path
      // so D3's confirmation and rule reset can never be bypassed (F4).
      await api.updateZone(zone.id, { ...zone, ...stripAppIdentity(patch) });
      await refreshState();
    } catch (error) {
      toast('error', `Update failed: ${errorMessage(error)}`);
      await refreshState();
    } finally {
      busy = false;
    }
  }

  function textValue(event: Event): string {
    return (event.currentTarget as HTMLInputElement).value;
  }
</script>

<aside class="flex h-full min-h-0 w-full flex-col border-l border-zf-border bg-zf-surface">
  <div class="flex items-start justify-between border-b border-zf-border px-4 py-3">
    <div class="min-w-0">
      <div class="flex items-center gap-2">
        <h3 class="text-sm font-semibold text-zf-text">Inspector</h3>
        {#if zone}
          <Badge variant="outline" class="!px-1.5 !py-0 !text-[0.6rem]">zone {zone.id}</Badge>
        {/if}
      </div>
      <p class="mt-0.5 truncate text-[0.7rem] text-zf-faint">
        {zone ? zone.label || 'Unnamed zone' : 'Select a zone on the canvas'}
      </p>
    </div>
    <button
      type="button"
      aria-label="Close inspector"
      class="rounded p-1 text-zf-faint hover:bg-zf-raised hover:text-zf-text"
      onclick={onclose}
    >
      <X size={15} />
    </button>
  </div>

  {#if !zone}
    <div class="flex flex-1 items-center justify-center px-6 text-center">
      <p class="text-xs leading-relaxed text-zf-faint">
        Click a zone to edit its process, browser and macro rules here. Changes are live but stay in the draft until you Save.
      </p>
    </div>
  {:else}
    <Tabs
      bind:value={tab}
      items={[
        { value: 'process', label: 'Process' },
        { value: 'browser', label: 'Browser' },
        { value: 'macro', label: 'Macro' },
        { value: 'console', label: 'Console' },
      ]}
      class="min-h-0 flex-1"
    >
      {#snippet children(value)}
        <div class="h-full overflow-auto px-4 pb-4">
          {#if value === 'process'}
            <div class="flex flex-col gap-4">
              <div class="flex flex-col gap-1.5">
                <Label for="insp-label">Zone label</Label>
                <Input
                  id="insp-label"
                  value={zone.label}
                  placeholder="e.g. Editor"
                  onchange={(e) => void commit({ label: textValue(e) })}
                />
              </div>
              <div class="flex flex-col gap-1.5">
                <span class="text-xs font-medium text-zf-text">Executable</span>
                <div
                  id="insp-exe"
                  class="rounded-md border border-zf-border bg-zf-bg px-3 py-2 font-mono text-xs text-zf-muted"
                >
                  {zone.exe || '—'}
                </div>
              </div>
              <div class="flex flex-col gap-1.5">
                <span class="text-xs font-medium text-zf-text">Full path (optional)</span>
                <div
                  id="insp-path"
                  class="break-all rounded-md border border-zf-border bg-zf-bg px-3 py-2 font-mono text-xs text-zf-muted"
                >
                  {zone.path || '—'}
                </div>
              </div>
              <p class="text-[0.65rem] leading-relaxed text-zf-faint">
                App identity is read-only here. Use <span class="text-zf-muted">Assign app…</span> to change it — reassigning resets this zone's rules after confirmation.
              </p>
              <div class="flex flex-col gap-1.5">
                <Label for="insp-mode">Launch mode</Label>
                <Select
                  id="insp-mode"
                  value={zone.launch_mode}
                  options={[
                    { value: 'launch', label: 'Launch — start the app if missing' },
                    { value: 'attach', label: 'Attach — only arrange if already running' },
                  ]}
                  onchange={(v) => void commit({ launch_mode: v as ZoneDto['launch_mode'] })}
                />
                <p class="text-[0.65rem] leading-relaxed text-zf-faint">
                  Attach mode never launches an application. Apply only arranges running windows.
                </p>
              </div>
              <div class="flex flex-col gap-1.5">
                <Label for="insp-title">Window title match</Label>
                <Input
                  id="insp-title"
                  value={zone.title_match}
                  placeholder="Optional title substring"
                  onchange={(e) => void commit({ title_match: textValue(e) })}
                />
              </div>
              <div class="flex flex-col gap-1.5">
                <Label for="insp-notes">Notes</Label>
                <textarea
                  id="insp-notes"
                  value={zone.notes}
                  rows="2"
                  placeholder="Non-executable notes"
                  class="w-full rounded-md border border-zf-border bg-zf-surface px-3 py-2 text-xs text-zf-text placeholder:text-zf-faint focus-visible:border-zf-accent focus-visible:outline-none"
                  onchange={(e) => void commit({ notes: (e.currentTarget as HTMLTextAreaElement).value })}
                ></textarea>
              </div>
              <Button
                variant="outline"
                size="sm"
                disabled={!zone.exe}
                onclick={() => void unassignApp(zone)}
              >
                <Ban size={14} /> Remove assignment
              </Button>
            </div>
          {:else if value === 'browser'}
            <div class="flex flex-col gap-4">
              <div class="flex flex-col gap-1.5">
                <Label for="insp-url">URL</Label>
                <Input
                  id="insp-url"
                  value={zone.url}
                  placeholder="https://…"
                  class="font-mono text-xs"
                  onchange={(e) => void commit({ url: textValue(e) })}
                />
              </div>
              <div class="flex flex-col gap-1.5">
                <Label for="insp-tab">Tab behavior</Label>
                <Select
                  id="insp-tab"
                  value={zone.tab_behavior}
                  options={[
                    { value: 'focus_or_new', label: 'Focus existing tab, or open a new one' },
                    { value: 'reuse_blank', label: 'Reuse a blank tab' },
                    { value: 'ignore', label: 'Ignore tab rules' },
                  ]}
                  onchange={(v) => void commit({ tab_behavior: v as ZoneDto['tab_behavior'] })}
                />
              </div>
              <p class="rounded-md border border-zf-border bg-zf-bg px-3 py-2 text-[0.7rem] leading-relaxed text-zf-faint">
                Browser rules are zone-scoped, so they belong to this profile only. Some browsers control their own windows and may ignore placement.
              </p>
            </div>
          {:else if value === 'macro'}
            <div class="flex flex-col gap-4">
              <div class="flex items-center justify-between rounded-md border border-zf-border bg-zf-bg px-3 py-2">
                <span class="text-xs text-zf-muted">Enable macro</span>
                <Switch
                  checked={zone.macro_enabled}
                  label="Enable macro"
                  onchange={(v) => void commit({ macro_enabled: v })}
                />
              </div>
              <div class="flex flex-col gap-1.5">
                <Label for="insp-trigger">Trigger window title</Label>
                <Input
                  id="insp-trigger"
                  value={zone.trigger_title}
                  placeholder="Title that activates the macro"
                  onchange={(e) => void commit({ trigger_title: textValue(e) })}
                />
              </div>
              <div class="flex flex-col gap-1.5">
                <Label for="insp-keys">Keys (AHK syntax)</Label>
                <Input
                  id="insp-keys"
                  value={zone.macro_text}
                  placeholder="^t"
                  class="font-mono text-xs"
                  onchange={(e) => void commit({ macro_text: textValue(e) })}
                />
              </div>
              <div class="flex flex-col gap-1.5">
                <Label for="insp-delay">Key delay (ms)</Label>
                <input
                  id="insp-delay"
                  type="number"
                  min="0"
                  max="5000"
                  value={zone.key_delay}
                  class="h-9 w-full rounded-md border border-zf-border bg-zf-surface px-3 text-sm text-zf-text focus-visible:border-zf-accent focus-visible:outline-none"
                  onchange={(e) => void commit({ key_delay: Number((e.currentTarget as HTMLInputElement).value) || 0 })}
                />
              </div>
            </div>
          {:else}
            <div class="flex h-full flex-col gap-2">
              <div class="flex items-center justify-between">
                <span class="text-[0.7rem] text-zf-faint">{logs.length} recent lines</span>
                <Button
                  variant="ghost"
                  size="sm"
                  onclick={() => void refreshState()}
                  title="Refresh from backend"
                >
                  <Terminal size={13} /> Refresh
                </Button>
              </div>
              <div class="min-h-0 flex-1 overflow-auto rounded-md border border-zf-border bg-zf-bg p-2 font-mono text-[0.7rem] leading-relaxed">
                {#each logs as line (line.seq)}
                  <div class="flex gap-2">
                    <span class="shrink-0 text-zf-faint">{line.seq}</span>
                    <span class="shrink-0 {line.level === 'error' ? 'text-zf-error' : line.level === 'warn' ? 'text-zf-warn' : 'text-zf-ok'}">
                      {line.level}
                    </span>
                    <span class="min-w-0 break-words text-zf-muted">{line.message}</span>
                  </div>
                {:else}
                  <p class="text-zf-faint">No log lines yet.</p>
                {/each}
              </div>
              <p class="text-[0.65rem] text-zf-faint">Live output arrives over SSE as actions run.</p>
            </div>
          {/if}
        </div>
      {/snippet}
    </Tabs>
  {/if}
</aside>
