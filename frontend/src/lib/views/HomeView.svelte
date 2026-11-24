<script lang="ts">
  import {
    ArrowRight,
    Copy,
    Eye,
    LayoutDashboard,
    Pencil,
    Play,
    Plus,
    Power,
    Rocket,
    Trash2,
    Type,
  } from '@lucide/svelte';
  import { api } from '$lib/api/client';
  import {
    createProfile,
    deleteProfile,
    doApply,
    doDisable,
    doEnable,
    doPreview,
    duplicateProfile,
    guardedProfileOp,
    renameProfile,
  } from '$lib/actions';
  import { getProfiles, isDirty, store } from '$lib/state/app.svelte';
  import { navigate, prompt } from '$lib/state/ui.svelte';
  import { actionEligibility } from '$lib/state/workflow';
  import Badge from '$lib/components/ui/badge.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import ApplyResults from '$lib/components/ApplyResults.svelte';

  const profiles = $derived(getProfiles());
  const applied = $derived(store.snapshot?.applied || 'Not applied');
  const selected = $derived(store.snapshot?.selected || '—');
  const dirty = $derived(isDirty());
  const enabled = $derived(store.snapshot?.enabled ?? false);
  const workArea = $derived(store.snapshot?.status.work_area ?? null);
  const eligibility = $derived(actionEligibility(dirty, enabled));

  async function newProfile(): Promise<void> {
    const name = await prompt({
      title: 'New profile',
      description: 'Each profile owns its own zones and application assignments.',
      label: 'Profile name',
      placeholder: 'e.g. Trading',
      confirmLabel: 'Create',
    });
    if (name) {
      if (await createProfile(name)) navigate('editor');
    }
  }

  async function rename(name: string): Promise<void> {
    const next = await prompt({
      title: 'Rename profile',
      label: 'New name',
      initialValue: name,
      confirmLabel: 'Rename',
    });
    if (next && next !== name) await renameProfile(name, next);
  }

  async function duplicate(name: string): Promise<void> {
    const next = await prompt({
      title: 'Duplicate profile',
      label: 'Copy name',
      initialValue: `${name} copy`,
      confirmLabel: 'Duplicate',
    });
    if (next) await duplicateProfile(name, next);
  }

  async function select(name: string): Promise<void> {
    // D6: Home selection only — never applies or launches.
    await guardedProfileOp((discard) => api.selectProfile(name, discard), {
      profile: name,
      action: `select ${name}`,
      successMessage: `Selected ${name} (no layout applied).`,
    });
  }

  async function applyProfile(name: string): Promise<void> {
    // D6: tray-style immediate apply when enabled, select-only when disabled.
    await guardedProfileOp((discard) => api.applyProfile(name, discard), {
      profile: name,
      action: `apply ${name}`,
      // No canned successMessage: the backend returns per-zone outcomes, which
      // guardedProfileOp now reports with an honest tone + results panel (B-1).
    });
  }

  async function edit(name: string): Promise<void> {
    // D6: rail semantics — load this profile's zones into the editor draft.
    const ok = await guardedProfileOp((discard) => api.loadProfile(name, discard), {
      profile: name,
      action: `open ${name} in the editor`,
    });
    if (ok) navigate('editor');
  }
</script>

<div class="h-full overflow-auto px-6 py-5">
  <div class="mx-auto flex max-w-6xl flex-col gap-5">
    <!-- Status hero — deliberately not a generic card grid -->
    <section class="relative overflow-hidden rounded-lg bg-zf-surface shadow-zf-1">
      <div class="absolute inset-y-0 left-0 w-1 bg-zf-accent"></div>
      <div class="flex flex-wrap items-end justify-between gap-6 px-6 py-5">
        <div>
          <p class="text-[0.7rem] uppercase tracking-[0.18em] text-zf-faint">Applied profile</p>
          <h2 class="mt-1 text-3xl font-semibold tracking-tight text-zf-text">{applied}</h2>
          <p class="mt-1 text-xs text-zf-muted">
            Selected: <span class="font-mono text-zf-text">{selected}</span>
            <span class="mx-2 text-zf-faint">·</span>
            Work area: <span class="font-mono text-zf-text">{workArea ? `${workArea.w}×${workArea.h}` : '—'}</span>
          </p>
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <Badge variant={enabled ? 'success' : 'error'}>{enabled ? 'Layout enabled' : 'Layout disabled'}</Badge>
          {#if dirty}<Badge variant="warning">Unsaved draft</Badge>{/if}
          <Button size="sm" onclick={() => navigate('editor')}>
            <LayoutDashboard size={14} /> Open editor <ArrowRight size={13} />
          </Button>
        </div>
      </div>

      <div class="flex flex-wrap items-center gap-2 border-t border-zf-border bg-zf-bg/40 px-6 py-3">
        <Button size="sm" variant="outline" disabled={!eligibility.allowed} title={eligibility.reason ?? 'Arrange running apps'} onclick={() => void doApply(false)}>
          <Play size={14} /> Apply
        </Button>
        <Button size="sm" variant="outline" disabled={!eligibility.allowed} title={eligibility.reason ?? 'Launch missing apps and arrange'} onclick={() => void doApply(true)}>
          <Rocket size={14} /> Launch &amp; Arrange
        </Button>
        <Button size="sm" variant="ghost" onclick={() => void doPreview()}>
          <Eye size={14} /> Preview
        </Button>
        <span class="mx-1 h-5 w-px bg-zf-border"></span>
        {#if enabled}
          <Button size="sm" variant="ghost" onclick={() => void doDisable()}>
            <Power size={14} /> Disable layout
          </Button>
        {:else}
          <Button size="sm" variant="default" onclick={() => void doEnable()}>
            <Power size={14} /> Enable layout
          </Button>
        {/if}
        {#if !eligibility.allowed}
          <span class="ml-1 text-xs text-zf-warn">{eligibility.reason}</span>
        {/if}
      </div>
    </section>

    <!-- Per-zone results from the last Apply / Launch & Arrange (Finding 3) -->
    <ApplyResults />

    <!-- Profile management -->
    <section>
      <div class="mb-2 flex items-center justify-between">
        <div>
          <h3 class="text-sm font-semibold text-zf-text">Saved profiles</h3>
          <p class="text-[0.7rem] text-zf-faint">Select changes only the selection · Apply arranges now</p>
        </div>
        <Button size="sm" variant="outline" onclick={() => void newProfile()}>
          <Plus size={14} /> New profile
        </Button>
      </div>

      <div class="overflow-hidden rounded-lg bg-zf-surface shadow-zf-1">
        {#each profiles as profile (profile.name)}
          <div class="flex flex-wrap items-center gap-3 border-b border-[color:var(--zf-hairline)] px-4 py-3 transition-colors last:border-0 hover:bg-zf-raised motion-reduce:transition-none {profile.selected ? 'bg-zf-accent/10' : ''}">
            <button type="button" class="flex min-w-0 flex-1 items-center gap-3 text-left" onclick={() => void select(profile.name)} title="Select this profile without applying it">
              <span class="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-zf-raised font-mono text-xs text-zf-accent">
                {profile.name.slice(0, 2).toUpperCase()}
              </span>
              <span class="min-w-0">
                <span class="block truncate text-sm font-medium text-zf-text">{profile.name}</span>
                <span class="block text-[0.7rem] text-zf-faint">{profile.zone_count} zone{profile.zone_count === 1 ? '' : 's'}</span>
              </span>
            </button>
            <div class="flex items-center gap-1.5">
              {#if profile.applied}<Badge variant="success" class="!text-[0.6rem]">applied</Badge>{/if}
              {#if profile.selected}<Badge variant="outline" class="!text-[0.6rem]">selected</Badge>{/if}
            </div>
            <div class="flex items-center gap-1">
              <Button size="sm" variant="outline" title="Apply this profile" onclick={() => void applyProfile(profile.name)}>
                <Play size={13} /> Apply
              </Button>
              <Button size="sm" variant="ghost" title="Edit in the layout editor" onclick={() => void edit(profile.name)}>
                <Pencil size={13} /> Edit
              </Button>
              <Button size="icon" variant="ghost" title="Duplicate" onclick={() => void duplicate(profile.name)}>
                <Copy size={13} />
              </Button>
              <Button size="sm" variant="ghost" title="Rename" onclick={() => void rename(profile.name)}>
                <Type size={13} />
              </Button>
              <Button size="sm" variant="ghost" title="Delete" onclick={() => void deleteProfile(profile.name)}>
                <Trash2 size={13} />
              </Button>
            </div>
          </div>
        {:else}
          <p class="px-4 py-6 text-center text-sm text-zf-faint">No profiles yet. Create one to get started.</p>
        {/each}
      </div>
    </section>
  </div>
</div>
