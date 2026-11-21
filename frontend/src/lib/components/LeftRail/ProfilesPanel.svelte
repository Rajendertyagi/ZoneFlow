<script lang="ts">
  import { Copy, Pencil, Plus, Trash2 } from '@lucide/svelte';
  import { api } from '$lib/api/client';
  import {
    createProfile,
    deleteProfile,
    duplicateProfile,
    guardedProfileOp,
    renameProfile,
  } from '$lib/actions';
  import { getProfiles, isDirty, store } from '$lib/state/app.svelte';
  import { prompt } from '$lib/state/ui.svelte';
  import Badge from '$lib/components/ui/badge.svelte';
  import Button from '$lib/components/ui/button.svelte';

  const profiles = $derived(getProfiles());
  const selected = $derived(store.snapshot?.selected ?? '');

  async function newProfile(): Promise<void> {
    const name = await prompt({
      title: 'New profile',
      description: 'Profiles keep their own zones and application assignments.',
      label: 'Profile name',
      placeholder: 'e.g. Trading',
      confirmLabel: 'Create',
    });
    if (name) await createProfile(name);
  }

  async function rename(name: string): Promise<void> {
    const next = await prompt({
      title: 'Rename profile',
      description: `Renaming ${name} keeps its zones and assignments.`,
      label: 'New name',
      initialValue: name,
      confirmLabel: 'Rename',
    });
    if (next && next !== name) await renameProfile(name, next);
  }

  async function duplicate(name: string): Promise<void> {
    const next = await prompt({
      title: 'Duplicate profile',
      description: `Create an independent copy of ${name}.`,
      label: 'Copy name',
      initialValue: `${name} copy`,
      confirmLabel: 'Duplicate',
    });
    if (next) await duplicateProfile(name, next);
  }

  async function load(name: string): Promise<void> {
    await guardedProfileOp((discard) => api.loadProfile(name, discard), {
      profile: name,
      action: `switch to ${name}`,
      successMessage: `Loaded ${name} into the editor draft.`,
    });
  }
</script>

<div class="flex h-full min-h-0 flex-col">
  <div class="flex items-center justify-between border-b border-zf-border px-3 py-2">
    <div>
      <h3 class="text-xs font-semibold uppercase tracking-wider text-zf-text">Profiles</h3>
      <p class="text-[0.65rem] text-zf-faint">Click to load a draft for editing</p>
    </div>
    <Button variant="ghost" size="icon" title="New profile" onclick={() => void newProfile()}>
      <Plus size={14} />
    </Button>
  </div>

  {#if isDirty()}
    <div class="mx-3 mt-2 rounded border border-zf-warn/30 bg-zf-warn/10 px-2 py-1 text-[0.65rem] text-zf-warn">
      Unsaved draft — switching will ask before discarding.
    </div>
  {/if}

  <div class="min-h-0 flex-1 overflow-auto p-2">
    {#each profiles as profile (profile.name)}
      <div
        class="group mb-1 rounded-md border px-2.5 py-2 transition-colors {profile.selected
          ? 'border-zf-accent/60 bg-zf-accent/10'
          : 'border-zf-border bg-zf-surface hover:border-zf-border-strong'}"
      >
        <button
          type="button"
          class="flex w-full items-center gap-2 text-left"
          onclick={() => void load(profile.name)}
          title="Load ${profile.name} into the draft"
        >
          <span class="min-w-0 flex-1 truncate text-sm font-medium text-zf-text">{profile.name}</span>
          <span class="shrink-0 font-mono text-[0.65rem] text-zf-faint">{profile.zone_count}z</span>
        </button>
        <div class="mt-1.5 flex items-center gap-1">
          {#if profile.applied}
            <Badge variant="success" class="!px-1.5 !py-0 !text-[0.6rem]">applied</Badge>
          {/if}
          {#if profile.selected}
            <Badge variant="outline" class="!px-1.5 !py-0 !text-[0.6rem]">selected</Badge>
          {/if}
          <span class="ml-auto flex gap-0.5 opacity-60 transition-opacity group-hover:opacity-100">
            <button type="button" title="Rename" class="rounded p-1 text-zf-faint hover:bg-zf-raised hover:text-zf-text" onclick={() => void rename(profile.name)}>
              <Pencil size={12} />
            </button>
            <button type="button" title="Duplicate" class="rounded p-1 text-zf-faint hover:bg-zf-raised hover:text-zf-text" onclick={() => void duplicate(profile.name)}>
              <Copy size={12} />
            </button>
            <button type="button" title="Delete" class="rounded p-1 text-zf-faint hover:bg-zf-error/15 hover:text-zf-error" onclick={() => void deleteProfile(profile.name)}>
              <Trash2 size={12} />
            </button>
          </span>
        </div>
      </div>
    {/each}
  </div>
</div>
