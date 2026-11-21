<script lang="ts">
  import { onMount } from 'svelte';
  import Badge from '$lib/components/ui/badge.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import {
    bootstrap,
    canRetryBootstrap,
    getBootstrapMessage,
    retryBootstrap,
    setSessionExpiredHandler,
  } from '$lib/api/client';
  import { refreshState, startEvents, stopEvents } from '$lib/api/events';
  import {
    getApplied,
    getProfiles,
    getSelected,
    isDirty,
    isEnabled,
    markSessionExpired,
    store,
  } from '$lib/state/app.svelte';

  type Phase = 'loading' | 'ready' | 'error';

  let phase = $state<Phase>('loading');
  let bootstrapMessage = $state('');
  let retryAvailable = $state(false);

  onMount(() => {
    setSessionExpiredHandler(markSessionExpired);
    void initialize();
    return () => stopEvents();
  });

  async function initialize(): Promise<void> {
    if (!(await bootstrap())) {
      showBootstrapError();
      return;
    }
    phase = 'ready';
    await refreshState();
    startEvents();
  }

  async function retry(): Promise<void> {
    phase = 'loading';
    if (!(await retryBootstrap())) {
      showBootstrapError();
      return;
    }
    phase = 'ready';
    await refreshState();
    startEvents();
  }

  function showBootstrapError(): void {
    phase = 'error';
    bootstrapMessage = getBootstrapMessage();
    retryAvailable = canRetryBootstrap();
  }

  const connectionLabel = $derived(
    phase === 'error'
      ? 'auth error'
      : store.connection === 'connected'
        ? 'connected'
        : store.connection === 'lost'
          ? 'connection lost'
          : store.connection === 'reconnecting'
            ? 'reconnecting'
            : 'connecting',
  );

  const connectionVariant = $derived(
    phase === 'error' || store.connection === 'lost'
      ? 'error'
      : store.connection === 'connected'
        ? 'success'
        : 'warning',
  );
</script>

<svelte:head>
  <title>ZoneFlow Studio</title>
</svelte:head>

<div class="mx-auto flex min-h-screen max-w-3xl flex-col gap-6 p-8">
  <header class="flex items-start justify-between gap-4">
    <div>
      <h1 class="text-xl font-semibold tracking-tight text-zf-text">ZoneFlow Studio</h1>
      <p class="mt-0.5 text-xs text-zf-muted">Frontend foundation — Phase 2 smoke test</p>
    </div>
    <Badge variant={connectionVariant}>{connectionLabel}</Badge>
  </header>

  {#if phase === 'error'}
    <div
      role="alert"
      class="rounded-md border border-zf-error/40 bg-zf-error/10 p-4 text-sm text-zf-error"
    >
      <p class="font-medium">Session unavailable</p>
      <p class="mt-1 text-zf-text/80">{bootstrapMessage}</p>
      {#if retryAvailable}
        <div class="mt-3">
          <Button size="sm" onclick={retry}>Retry</Button>
        </div>
      {/if}
    </div>
  {:else if phase === 'loading'}
    <div class="rounded-md border border-zf-border bg-zf-panel p-4 text-sm text-zf-muted">
      Opening session…
    </div>
  {:else}
    {#if store.authError}
      <div
        role="alert"
        class="rounded-md border border-zf-error/40 bg-zf-error/10 p-4 text-sm text-zf-error"
      >
        <p class="font-medium">Connection problem</p>
        <p class="mt-1 text-zf-text/80">{store.authError}</p>
      </div>
    {/if}

    {#if store.connection === 'lost'}
      <div
        role="alert"
        class="rounded-md border border-zf-warn/40 bg-zf-warn/10 p-4 text-sm text-zf-warn"
      >
        Connection lost — showing the last known state.
      </div>
    {/if}

    <section class="rounded-lg border border-zf-border bg-zf-panel">
      <div class="border-b border-zf-border px-4 py-3">
        <h2 class="text-sm font-semibold text-zf-text">State snapshot</h2>
      </div>
      <div class="grid gap-3 p-4 sm:grid-cols-2">
        <div><span class="text-zf-muted">Applied:</span> {getApplied() || '—'}</div>
        <div><span class="text-zf-muted">Selected:</span> {getSelected() || '—'}</div>
        <div><span class="text-zf-muted">Enabled:</span> {isEnabled() ? 'yes' : 'no'}</div>
        <div><span class="text-zf-muted">Dirty:</span> {isDirty() ? 'yes' : 'no'}</div>
        <div><span class="text-zf-muted">Zones:</span> {store.snapshot?.zones.length ?? 0}</div>
        <div>
          <span class="text-zf-muted">Last event:</span>
          {store.lastEventAt ? new Date(store.lastEventAt).toLocaleTimeString() : '—'}
        </div>
      </div>
    </section>

    <section class="rounded-lg border border-zf-border bg-zf-panel">
      <div class="border-b border-zf-border px-4 py-3">
        <h2 class="text-sm font-semibold text-zf-text">Profiles</h2>
      </div>
      <ul class="divide-y divide-zf-border">
        {#each getProfiles() as profile (profile.name)}
          <li class="flex items-center justify-between gap-3 px-4 py-2.5">
            <span class="text-zf-text">{profile.name}</span>
            <span class="flex items-center gap-2">
              <span class="text-xs text-zf-muted">{profile.zone_count} zones</span>
              {#if profile.applied}<Badge variant="success">applied</Badge>{/if}
              {#if profile.selected}<Badge variant="outline">selected</Badge>{/if}
            </span>
          </li>
        {:else}
          <li class="px-4 py-3 text-sm text-zf-muted">No profiles loaded yet.</li>
        {/each}
      </ul>
    </section>
  {/if}
</div>
