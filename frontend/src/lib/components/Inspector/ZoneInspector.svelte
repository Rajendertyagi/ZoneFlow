<script lang="ts">
  import { Ban, MousePointer2, Terminal, X } from '@lucide/svelte';
  import { api, type ZoneDto } from '$lib/api/client';
  import { refreshState } from '$lib/api/events';
  import { unassignApp } from '$lib/actions';
  import { store } from '$lib/state/app.svelte';
  import { toast } from '$lib/state/ui.svelte';
  import { errorMessage, stripAppIdentity } from '$lib/state/workflow';
  import {
    canCommitLauncherPath,
    scriptPathErrorMessage,
    validateScriptPath,
  } from '$lib/state/launcher';
  import { Badge } from '$lib/components/ui/badge/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { Label } from '$lib/components/ui/label/index.js';
  import * as NativeSelect from '$lib/components/ui/native-select/index.js';
  import { Switch } from '$lib/components/ui/switch/index.js';
  import * as Tabs from '$lib/components/ui/tabs/index.js';

  const focusRing = 'focus-visible:outline-2 focus-visible:outline focus-visible:outline-offset-2 focus-visible:outline-primary focus-visible:ring-0 motion-reduce:transition-none';

  interface Props {
    zone: ZoneDto | null;
    onclose: () => void;
  }
  let { zone, onclose }: Props = $props();

  let tab = $state('process');
  let busy = $state(false);

  const logs = $derived(store.snapshot?.logs ?? []);

  // An invalid edit is held locally so the error can be shown WITHOUT writing
  // it to the zone. The last valid persisted value stays authoritative. The
  // draft is keyed to the zone id so switching zones never leaks stale text.
  let launcherDraft = $state<{ zoneId: number; text: string } | null>(null);
  const launcherValue = $derived(
    launcherDraft !== null && launcherDraft.zoneId === zone?.id
      ? launcherDraft.text
      : (zone?.launcher_path ?? ''),
  );
  const launcherError = $derived(
    launcherValue.trim() ? validateScriptPath(launcherValue) : null,
  );

  function onLauncherChange(event: Event): void {
    if (!zone) return;
    const next = textValue(event);
    if (!canCommitLauncherPath(next)) {
      // Keep the previous valid value; show the error from the local draft.
      launcherDraft = { zoneId: zone.id, text: next };
      return;
    }
    launcherDraft = null;
    void commit({ launcher_path: next });
  }

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

<aside class="flex h-full min-h-0 w-full flex-col bg-zf-surface">
  <div class="flex items-start justify-between shadow-zf-topbar px-4 py-3">
    <div class="min-w-0">
      <div class="flex items-center gap-2">
        <h3 class="text-sm font-semibold text-zf-text">Inspector</h3>
        {#if zone}
          <Badge class="bg-transparent border border-input text-muted-foreground !px-1.5 !py-0 !text-xs">zone {zone.id}</Badge>
        {/if}
      </div>
      {#if zone}
        <p class="mt-0.5 truncate text-xs text-zf-muted">
          {zone.label || 'Unnamed zone'}
        </p>
      {/if}
    </div>
    <button
      type="button"
      aria-label="Close inspector"
      class="inline-flex h-6 w-6 items-center justify-center rounded p-1 text-zf-faint hover:bg-zf-raised hover:text-zf-text"
      onclick={onclose}
    >
      <X size={15} />
    </button>
  </div>

  {#if !zone}
    <div class="flex flex-1 items-center justify-center p-4 shadow-zf-inset-ring">
      <div class="flex max-w-xs flex-col items-center gap-2 text-center">
        <MousePointer2 size={20} class="text-zf-faint" />
        <h4 class="text-base font-semibold text-zf-text">No zone selected</h4>
        <p class="text-sm leading-relaxed text-zf-muted">
          Click a zone on the canvas to edit its process, browser and macro rules. Changes stay in the draft until you Save.
        </p>
      </div>
    </div>
  {:else}
    <Tabs.Root bind:value={tab} class="flex min-h-0 flex-1 flex-col">
      <Tabs.List aria-label="Inspector sections" class="flex w-full shrink-0 gap-1 border-b border-border bg-transparent p-0">
        <Tabs.Trigger value="process" class={`px-3 py-2 ${focusRing}`}>Process</Tabs.Trigger>
        <Tabs.Trigger value="browser" class={`px-3 py-2 ${focusRing}`}>Browser</Tabs.Trigger>
        <Tabs.Trigger value="macro" class={`px-3 py-2 ${focusRing}`}>Macro</Tabs.Trigger>
        <Tabs.Trigger value="console" class={`px-3 py-2 ${focusRing}`}>Console</Tabs.Trigger>
      </Tabs.List>
      <Tabs.Content value="process" tabindex={-1} class="min-h-0 flex-1 overflow-auto px-4 pb-4 pt-4">
            <div class="flex flex-col gap-4">
              <div class="flex flex-col gap-1.5">
                <Label class="text-xs text-muted-foreground" for="insp-label">Zone label</Label>
                <Input
                  id="insp-label"
                  value={zone.label}
                  placeholder="e.g. Editor"
                  class="border border-border focus-visible:border-primary"
                  onchange={(e) => void commit({ label: textValue(e) })}
                />
              </div>
              <div class="flex flex-col gap-1.5">
                <span class="text-xs font-medium text-zf-text">Executable</span>
                <div
                  id="insp-exe"
                  class="rounded-md bg-zf-bg px-3 py-2 font-mono text-xs text-zf-muted"
                >
                  {zone.exe || '—'}
                </div>
              </div>
              <div class="flex flex-col gap-1.5">
                <span class="text-xs font-medium text-zf-text">Full path (optional)</span>
                <div
                  id="insp-path"
                  class="break-all rounded-md bg-zf-bg px-3 py-2 font-mono text-xs text-zf-muted"
                >
                  {zone.path || '—'}
                </div>
              </div>
              <p class="text-xs leading-relaxed text-zf-muted">
                App identity is read-only here. Use <span class="text-zf-muted">Assign app…</span> to change it — reassigning resets this zone's rules after confirmation.
              </p>

              <!-- Launcher script: when set, it runs instead of the EXE above. -->
              <div class="flex flex-col gap-1.5 rounded-md bg-zf-bg p-3 shadow-zf-1">
                <span class="text-xs font-medium text-zf-text">Launcher script (.bat/.cmd)</span>
                <p class="text-xs leading-relaxed text-zf-muted">
                  When set, this script is executed instead of the executable. The full path above
                  becomes an optional window-identification override only — it is never launched.
                </p>
                <Input
                  id="insp-launcher"
                  value={launcherValue}
                  placeholder="C:\Scripts\launch.bat (empty = launch the EXE)"
                  class="border border-border font-mono text-xs focus-visible:border-primary"
                  onchange={onLauncherChange}
                />
                {#if launcherError}
                  <p class="text-xs text-zf-error">
                    Invalid launcher: {scriptPathErrorMessage(launcherError)}
                  </p>
                {/if}
                <div class="flex items-center justify-between pt-1">
                  <span class="text-xs text-zf-muted">Show console when running script</span>
                  <Switch
                    checked={zone.launcher_show_console}
                    aria-label="Show console when running script"
                    class={focusRing}
                    onCheckedChange={(v) => void commit({ launcher_show_console: v })}
                  />
                </div>
              </div>

              <div class="flex flex-col gap-1.5">
                <Label class="text-xs text-muted-foreground" for="insp-mode">Launch mode</Label>
                <NativeSelect.Root
                  id="insp-mode"
                  value={zone.launch_mode}
                  aria-label="Launch mode"
                  class="h-9 border border-border focus-visible:border-primary"
                  onchange={(e) => void commit({ launch_mode: (e.currentTarget as HTMLSelectElement).value as ZoneDto['launch_mode'] })}
                >
                  <NativeSelect.Option value="launch">Launch — start the app if missing</NativeSelect.Option>
                  <NativeSelect.Option value="attach">Attach — only arrange if already running</NativeSelect.Option>
                </NativeSelect.Root>
                <p class="text-xs leading-relaxed text-zf-muted">
                  Attach mode never launches an application. Apply only arranges running windows.
                </p>
              </div>
              <div class="flex flex-col gap-1.5">
                <Label class="text-xs text-muted-foreground" for="insp-title">Window title match</Label>
                <Input
                  id="insp-title"
                  value={zone.title_match}
                  placeholder="Optional title substring"
                  class="border border-border focus-visible:border-primary"
                  onchange={(e) => void commit({ title_match: textValue(e) })}
                />
              </div>
              <div class="flex flex-col gap-1.5">
                <Label class="text-xs text-muted-foreground" for="insp-notes">Notes</Label>
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
                class={`h-7 text-xs ${focusRing}`}
                onclick={() => void unassignApp(zone)}
              >
                <Ban size={14} /> Remove assignment
              </Button>
            </div>
      </Tabs.Content>
      <Tabs.Content value="browser" tabindex={-1} class="min-h-0 flex-1 overflow-auto px-4 pb-4 pt-4">
            <div class="flex flex-col gap-4">
              <div class="flex flex-col gap-1.5">
                <Label class="text-xs text-muted-foreground" for="insp-url">URL</Label>
                <Input
                  id="insp-url"
                  value={zone.url}
                  placeholder="https://…"
                  class="border border-border font-mono text-xs focus-visible:border-primary"
                  onchange={(e) => void commit({ url: textValue(e) })}
                />
              </div>
              <div class="flex flex-col gap-1.5">
                <Label class="text-xs text-muted-foreground" for="insp-tab">Tab behavior</Label>
                <NativeSelect.Root
                  id="insp-tab"
                  value={zone.tab_behavior}
                  aria-label="Tab behavior"
                  class="h-9 border border-border focus-visible:border-primary"
                  onchange={(e) => void commit({ tab_behavior: (e.currentTarget as HTMLSelectElement).value as ZoneDto['tab_behavior'] })}
                >
                  <NativeSelect.Option value="focus_or_new">Focus existing tab, or open a new one</NativeSelect.Option>
                  <NativeSelect.Option value="reuse_blank">Reuse a blank tab</NativeSelect.Option>
                  <NativeSelect.Option value="ignore">Ignore tab rules</NativeSelect.Option>
                </NativeSelect.Root>
              </div>
              <p class="rounded-md bg-zf-bg px-3 py-2 text-xs leading-relaxed text-zf-muted">
                Browser rules are zone-scoped, so they belong to this profile only. Some browsers control their own windows and may ignore placement.
              </p>
            </div>
      </Tabs.Content>
      <Tabs.Content value="macro" tabindex={-1} class="min-h-0 flex-1 overflow-auto px-4 pb-4 pt-4">
            <div class="flex flex-col gap-4">
              <div class="flex items-center justify-between rounded-md bg-zf-bg px-3 py-2">
                <span class="text-xs text-zf-muted">Enable macro</span>
                <Switch
                  checked={zone.macro_enabled}
                  aria-label="Enable macro"
                  class={focusRing}
                  onCheckedChange={(v) => void commit({ macro_enabled: v })}
                />
              </div>
              <div class="flex flex-col gap-1.5">
                <Label class="text-xs text-muted-foreground" for="insp-trigger">Trigger window title</Label>
                <Input
                  id="insp-trigger"
                  value={zone.trigger_title}
                  placeholder="Title that activates the macro"
                  class="border border-border focus-visible:border-primary"
                  onchange={(e) => void commit({ trigger_title: textValue(e) })}
                />
              </div>
              <div class="flex flex-col gap-1.5">
                <Label class="text-xs text-muted-foreground" for="insp-keys">Keys (AHK syntax)</Label>
                <Input
                  id="insp-keys"
                  value={zone.macro_text}
                  placeholder="^t"
                  class="border border-border font-mono text-xs focus-visible:border-primary"
                  onchange={(e) => void commit({ macro_text: textValue(e) })}
                />
              </div>
              <div class="flex flex-col gap-1.5">
                <Label class="text-xs text-muted-foreground" for="insp-delay">Key delay (ms)</Label>
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
      </Tabs.Content>
      <Tabs.Content value="console" tabindex={-1} class="min-h-0 flex-1 overflow-auto px-4 pb-4 pt-4">
        <div class="flex h-full flex-col gap-2">
              <div class="flex items-center justify-between">
                <span class="text-xs text-zf-faint">{logs.length} recent lines</span>
                <Button
                  variant="ghost"
                  size="sm"
                  class={`h-7 text-xs ${focusRing}`}
                  onclick={() => void refreshState()}
                  title="Refresh from backend"
                >
                  <Terminal size={13} /> Refresh
                </Button>
              </div>
              <div class="min-h-0 flex-1 overflow-auto rounded-md bg-zf-bg p-2 font-mono text-xs leading-relaxed shadow-zf-1">
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
              <p class="text-xs text-zf-muted">Live output arrives over SSE as actions run.</p>
            </div>
      </Tabs.Content>
    </Tabs.Root>
  {/if}
</aside>
