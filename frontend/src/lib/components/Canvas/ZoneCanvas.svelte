<script lang="ts">
  /**
   * Canvas-primary SVG zone editor.
   *
   * The viewBox is the work area in pixels; zones are normalized percentages.
   * All pointer gestures are resolved locally into `$state` previews and only
   * committed through the typed client on `pointerup` — never per pointermove.
   */
  import { onMount } from 'svelte';
  import ContextMenu from '$lib/components/ui/context-menu.svelte';
  import {
    registerCanvasCommands,
    unregisterCanvasCommands,
    type CanvasCommands,
  } from '$lib/state/canvas-commands';
  import {
    api,
    type AssignDto,
    type ZoneDto,
    type ZoneGeometryUpdateDto,
  } from '$lib/api/client';
  import { refreshState } from '$lib/api/events';
  import { getSnapshot, patchZoneLocal, store } from '$lib/state/app.svelte';
  import { reconcileSelection } from '$lib/state/selection';
  import { canRemoveZone } from '$lib/state/workflow';
  import DragPreview from './DragPreview.svelte';
  import SnapGuides from './SnapGuides.svelte';
  import ZoneRect from './ZoneRect.svelte';
  import {
    blankZone,
    clampSplitRatio,
    findSharedBoundaries,
    firstFreeCell,
    moveZone,
    rectFromPoints,
    resizeBoundary,
    resizeZoneEdge,
    snapPercent,
    snapRectToGrid,
    zoneEdgeHandles,
    type RectPct,
    type ResizeHandle,
    type SharedBoundary,
  } from './geometry';

  interface Props {
    mode: 'layout' | 'draw';
    showGap: boolean;
    splitRatio: number;
    onnotice?: (message: string) => void;
    /**
     * Parent-owned assignment — REQUIRED so every assignment goes through the
     * D3 confirmation + rule-reset guard (`assignAppGuarded`). Never add a
     * direct `api.assignZone` fallback here: that would silently bypass D3.
     */
    onassign: (zoneId: number, spec: AssignDto) => void | Promise<void>;
    /** Open the application picker for a zone (context-menu action). */
    onpick?: (zoneId: number) => void;
  }

  let { mode, showGap, splitRatio, onnotice, onassign, onpick }: Props = $props();

  interface CanvasSettings {
    snap_points: number[];
    grid_cols: number;
    grid_rows: number;
    min_zone_pct: number;
    boundary_tolerance_pct: number;
    zone_tints: string[];
  }

  const FALLBACK: CanvasSettings = {
    snap_points: [25, 50, 75],
    grid_cols: 12,
    grid_rows: 8,
    min_zone_pct: 15,
    boundary_tolerance_pct: 1.5,
    zone_tints: ['#F5B544', '#6EA8FE', '#5BD6A0', '#C792EA', '#FF8A65', '#4DD0E1'],
  };

  interface PreviewState {
    zones: Record<number, RectPct>;
    draw: RectPct | null;
    snap: { axis: 'v' | 'h'; at: number; snapped: number | null } | null;
  }

  type Interaction =
    | { kind: 'move'; pointerId: number; origZone: ZoneDto; start: { x: number; y: number } }
    | { kind: 'resize'; pointerId: number; boundary: SharedBoundary; origZones: ZoneDto[] }
    | {
        kind: 'edge-resize';
        pointerId: number;
        zone: ZoneDto;
        axis: 'v' | 'h';
        edge: 'low' | 'high';
      }
    | { kind: 'draw'; pointerId: number; start: { x: number; y: number } };

  let svgEl: SVGSVGElement;
  let selectedId = $state<number | null>(null);
  let mergeMode = $state(false);
  let mergeSourceId = $state<number | null>(null);
  let contextZoneId = $state<number | null>(null);
  let dropTargetId = $state<number | null>(null);
  let preview = $state<PreviewState | null>(null);
  let interaction: Interaction | null = null;

  const snapshot = $derived(store.snapshot);
  const zones = $derived(snapshot?.zones ?? []);
  const settings = $derived<CanvasSettings>(snapshot?.settings ?? FALLBACK);
  const workArea = $derived(snapshot?.status.work_area ?? { x: 0, y: 0, w: 1920, h: 1080 });
  const boundaries = $derived(findSharedBoundaries(zones, settings.boundary_tolerance_pct));
  const outerHandles = $derived(zoneEdgeHandles(zones, boundaries, settings.boundary_tolerance_pct));
  const gapWidth = $derived(Math.max(2, snapshot?.draft.gap ?? 8));

  // A profile switch (create/load) must drop any stale local selection so the
  // blank canvas never reports a zone from the previous profile. Key off the
  // active profile name, NOT zones.length: a newly created zone is selected
  // before its authoritative refresh lands, and zones would still be empty in
  // that window — clearing on length would lose the fresh selection.
  let lastProfile: string | null = null;
  $effect(() => {
    const active = snapshot?.draft.active_profile ?? null;
    if (lastProfile !== null && active !== lastProfile) selectedId = null;
    lastProfile = active;
  });

  /* --------------------------- coordinate conversion ----------------------- */

  function clientToPct(event: { clientX: number; clientY: number }): { x: number; y: number } {
    const ctm = svgEl?.getScreenCTM();
    if (!ctm) return { x: 0, y: 0 };
    const point = new DOMPoint(event.clientX, event.clientY).matrixTransform(ctm.inverse());
    return {
      x: Math.min(100, Math.max(0, (point.x / workArea.w) * 100)),
      y: Math.min(100, Math.max(0, (point.y / workArea.h) * 100)),
    };
  }

  function notice(message: string): void {
    onnotice?.(message);
  }

  function errorMessage(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }

  /* ------------------------------ interactions ----------------------------- */

  function onBodyPointerDown(zone: ZoneDto, event: PointerEvent): void {
    if (interaction) return;
    if (mode === 'draw') {
      // Drawing works anywhere on the grid, including over existing zones.
      onBackgroundPointerDown(event);
      return;
    }
    if (mergeMode) {
      pickMerge(zone.id);
      event.preventDefault();
      return;
    }
    selectedId = zone.id;
    interaction = {
      kind: 'move',
      pointerId: event.pointerId,
      origZone: { ...zone },
      start: clientToPct(event),
    };
    svgEl.setPointerCapture(event.pointerId);
    event.preventDefault();
  }

  function onResizeStart(handle: ResizeHandle, event: PointerEvent): void {
    if (mode === 'draw') {
      onBackgroundPointerDown(event);
      return;
    }
    if (interaction) return;
    if (handle.kind === 'shared') {
      interaction = {
        kind: 'resize',
        pointerId: event.pointerId,
        boundary: handle.boundary,
        origZones: zones.map((z) => ({ ...z })),
      };
    } else {
      const zone = zones.find((z) => z.id === handle.zoneId);
      if (!zone) return;
      interaction = {
        kind: 'edge-resize',
        pointerId: event.pointerId,
        zone: { ...zone },
        axis: handle.axis,
        edge: handle.edge,
      };
    }
    svgEl.setPointerCapture(event.pointerId);
    event.preventDefault();
  }

  function onBackgroundPointerDown(event: PointerEvent): void {
    if (interaction) return;
    if (mode === 'draw') {
      const start = clientToPct(event);
      interaction = { kind: 'draw', pointerId: event.pointerId, start };
      preview = { zones: {}, draw: { ...start, w: 0, h: 0 }, snap: null };
      svgEl.setPointerCapture(event.pointerId);
      event.preventDefault();
    } else {
      selectedId = null;
      if (mergeMode) {
        mergeMode = false;
        mergeSourceId = null;
      }
    }
  }

  function onPointerMove(event: PointerEvent): void {
    if (!interaction || event.pointerId !== interaction.pointerId) return;
    const point = clientToPct(event);

    if (interaction.kind === 'move') {
      const moved = moveZone(interaction.origZone, point.x - interaction.start.x, point.y - interaction.start.y);
      preview = { zones: { [interaction.origZone.id]: moved }, draw: null, snap: null };
      return;
    }

    if (interaction.kind === 'resize') {
      const boundary = interaction.boundary;
      const rawAt = boundary.axis === 'v' ? point.x : point.y;
      const snapped = snapPercent(rawAt, settings.snap_points, settings.boundary_tolerance_pct);
      const result = resizeBoundary(
        interaction.origZones,
        boundary,
        snapped,
        settings.min_zone_pct,
        settings.boundary_tolerance_pct,
      );
      if (result) {
        preview = {
          zones: result.changes,
          draw: null,
          snap: {
            axis: boundary.axis,
            at: result.at,
            snapped: result.at === snapped && snapped !== rawAt ? snapped : null,
          },
        };
      }
      return;
    }

    if (interaction.kind === 'edge-resize') {
      const rawAt = interaction.axis === 'v' ? point.x : point.y;
      const geometry = resizeZoneEdge(
        interaction.zone,
        interaction.axis,
        interaction.edge,
        rawAt,
        settings.min_zone_pct,
        settings.snap_points,
        settings.boundary_tolerance_pct,
      );
      if (geometry) {
        const at =
          interaction.axis === 'v'
            ? interaction.edge === 'low'
              ? geometry.x
              : geometry.x + geometry.w
            : interaction.edge === 'low'
              ? geometry.y
              : geometry.y + geometry.h;
        const snapped = snapPercent(rawAt, settings.snap_points, settings.boundary_tolerance_pct);
        preview = {
          zones: { [interaction.zone.id]: geometry },
          draw: null,
          snap: {
            axis: interaction.axis,
            at,
            snapped: at === snapped && snapped !== rawAt ? snapped : null,
          },
        };
      }
      return;
    }

    // draw
    const rect = rectFromPoints(interaction.start, point);
    preview = {
      zones: {},
      draw: snapRectToGrid(rect, settings.grid_cols, settings.grid_rows, settings.min_zone_pct),
      snap: null,
    };
  }

  async function onPointerUp(event: PointerEvent): Promise<void> {
    if (!interaction || event.pointerId !== interaction.pointerId) return;
    const current = interaction;
    const result = preview;
    interaction = null;
    preview = null;
    if (svgEl.hasPointerCapture(event.pointerId)) svgEl.releasePointerCapture(event.pointerId);

    try {
      if (current.kind === 'move' && result?.zones[current.origZone.id]) {
        const geometry = result.zones[current.origZone.id];
        const original = current.origZone;
        if (Math.abs(geometry.x - original.x) > 0.01 || Math.abs(geometry.y - original.y) > 0.01) {
          await api.updateZone(original.id, { ...original, ...geometry });
          patchZoneLocal(original.id, geometry);
          await refreshState();
        }
      } else if (current.kind === 'resize' && result) {
        // A divider drag moves both sides. Commit every changed zone in ONE
        // request so the backend records a single undo point for the gesture.
        const updates: ZoneGeometryUpdateDto[] = [];
        for (const [idString, geometry] of Object.entries(result.zones)) {
          const id = Number(idString);
          const original = current.origZones.find((z) => z.id === id);
          if (!original) continue;
          const same =
            Math.abs(geometry.x - original.x) < 0.01 &&
            Math.abs(geometry.y - original.y) < 0.01 &&
            Math.abs(geometry.w - original.w) < 0.01 &&
            Math.abs(geometry.h - original.h) < 0.01;
          if (same) continue;
          updates.push({ id, x: geometry.x, y: geometry.y, w: geometry.w, h: geometry.h });
        }
        if (updates.length > 0) {
          await api.updateZonesBatch(updates);
          // Keep the optimistic local patch so the zones don't snap back while
          // the authoritative SSE refresh is in flight.
          for (const u of updates) {
            patchZoneLocal(u.id, { x: u.x, y: u.y, w: u.w, h: u.h });
          }
          await refreshState();
        }
      } else if (current.kind === 'edge-resize' && result?.zones[current.zone.id]) {
        // Only this zone moved. A single updateZone gives one undo point; no
        // batch is needed (and no neighbour should be dragged along).
        const geometry = result.zones[current.zone.id];
        const original = current.zone;
        if (
          Math.abs(geometry.x - original.x) > 0.01 ||
          Math.abs(geometry.y - original.y) > 0.01 ||
          Math.abs(geometry.w - original.w) > 0.01 ||
          Math.abs(geometry.h - original.h) > 0.01
        ) {
          await api.updateZone(original.id, { ...original, ...geometry });
          patchZoneLocal(original.id, geometry);
          await refreshState();
        }
      } else if (current.kind === 'draw' && result?.draw) {
        const rect = result.draw;
        if (rect.w >= settings.min_zone_pct - 0.01 && rect.h >= settings.min_zone_pct - 0.01) {
          const created = await api.createZone(blankZone(rect));
          selectedId = created.id;
          await refreshState();
        }
      }
    } catch (error) {
      notice(`Canvas action failed: ${errorMessage(error)}`);
      await refreshState();
    }
  }

  function onPointerCancel(event: PointerEvent): void {
    if (!interaction || event.pointerId !== interaction.pointerId) return;
    interaction = null;
    preview = null;
    if (svgEl.hasPointerCapture(event.pointerId)) svgEl.releasePointerCapture(event.pointerId);
  }

  /* -------------------------------- commands ------------------------------- */

  async function split(id: number, axis: 'x' | 'y'): Promise<void> {
    const zone = zones.find((z) => z.id === id);
    if (!zone) return;
    const requested = Number.isFinite(splitRatio) ? splitRatio : 0.5;
    const ratio = clampSplitRatio(zone, axis, requested, settings.min_zone_pct);
    try {
      await api.splitZone(id, { axis, ratio });
      await refreshState();
    } catch (error) {
      notice(`Split failed: ${errorMessage(error)}`);
    }
  }

  async function remove(id: number): Promise<void> {
    try {
      await api.deleteZone(id);
      if (selectedId === id) selectedId = null;
      if (mergeSourceId === id) mergeSourceId = null;
      await refreshState();
    } catch (error) {
      notice(`Remove failed: ${errorMessage(error)}`);
    }
  }

  async function commitMerge(source: number, target: number): Promise<void> {
    try {
      await api.mergeZones({ source_id: source, target_id: target });
      selectedId = target;
      await refreshState();
    } catch (error) {
      notice(`Merge failed: ${errorMessage(error)}`);
    }
  }

  function pickMerge(id: number): void {
    if (mergeSourceId === null) {
      mergeSourceId = id;
      selectedId = id;
      return;
    }
    if (mergeSourceId === id) {
      mergeSourceId = null;
      return;
    }
    const source = mergeSourceId;
    mergeSourceId = null;
    mergeMode = false;
    void commitMerge(source, id);
  }

  async function unassign(id: number): Promise<void> {
    try {
      await api.unassignZone(id);
      await refreshState();
    } catch (error) {
      notice(`Unassign failed: ${errorMessage(error)}`);
    }
  }

  async function addZone(): Promise<void> {
    const rect = firstFreeCell(zones, settings.grid_cols, settings.grid_rows, settings.min_zone_pct);
    try {
      const created = await api.createZone(blankZone(rect));
      selectedId = created.id;
      await refreshState();
    } catch (error) {
      notice(`Add zone failed: ${errorMessage(error)}`);
    }
  }

  async function undo(): Promise<void> {
    try {
      await api.undo();
      await refreshState();
      selectedId = reconcileSelection(getSnapshot(), selectedId);
    } catch (error) {
      notice(`Undo failed: ${errorMessage(error)}`);
    }
  }

  async function redo(): Promise<void> {
    try {
      await api.redo();
      await refreshState();
      selectedId = reconcileSelection(getSnapshot(), selectedId);
    } catch (error) {
      notice(`Redo failed: ${errorMessage(error)}`);
    }
  }

  /* --------------------------- exposed to toolbar -------------------------- */

  export function splitSelected(axis: 'x' | 'y'): void {
    if (selectedId !== null) void split(selectedId, axis);
    else notice('Select a zone first, then split.');
  }

  export function startMerge(): void {
    mergeMode = !mergeMode;
    mergeSourceId = null;
    if (mergeMode) notice('Merge: click a source zone, then a target zone.');
  }

  export function removeSelected(): void {
    if (selectedId !== null) void remove(selectedId);
    else notice('Select a zone first, then remove.');
  }

  export { addZone, undo, redo };

  // Make undo/redo reachable by the global keyboard handler in App.svelte.
  // The identity-checked unregister keeps a stale teardown from clearing a
  // newer registration if the editor canvas is ever remounted.
  const commands: CanvasCommands = { undo, redo };
  onMount(() => {
    registerCanvasCommands(commands);
    return () => unregisterCanvasCommands(commands);
  });

  export function getSelectedId(): number | null {
    return selectedId;
  }

  /* ------------------------------- drag & drop ----------------------------- */

  function onZoneDragOver(zone: ZoneDto, event: DragEvent): void {
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = 'copy';
    dropTargetId = zone.id;
  }

  function onZoneDragLeave(zone: ZoneDto, event: DragEvent): void {
    event.preventDefault();
    // dragleave also fires when crossing child elements; ignore those.
    const next = event.relatedTarget as Node | null;
    if (next && (event.currentTarget as Node).contains(next)) return;
    if (dropTargetId === zone.id) dropTargetId = null;
  }

  function onZoneDrop(zone: ZoneDto, event: DragEvent): void {
    event.preventDefault();
    dropTargetId = null;
    const raw =
      event.dataTransfer?.getData('application/x-zoneflow-app') ??
      event.dataTransfer?.getData('text/plain') ??
      '';
    if (!raw) return;
    try {
      const spec = JSON.parse(raw) as AssignDto;
      if (typeof spec.label === 'string' && typeof spec.exe === 'string') {
        selectedId = zone.id;
        // Always route through the parent-owned guard (D3); there is no
        // unguarded `api.assignZone` fallback in this component.
        void onassign(zone.id, spec);
      }
    } catch {
      notice('Dropped app payload was not valid.');
    }
  }

  /* ------------------------------ context menu ----------------------------- */

  const menuItems = $derived.by(() => {
    const zone = contextZoneId !== null ? zones.find((z) => z.id === contextZoneId) : null;
    const items: Array<{ label: string; onselect?: () => void; disabled?: boolean }> = [];
    if (zone) {
      items.push({
        label: `Split left / right (${Math.round(splitRatio * 100)}%)`,
        onselect: () => void split(zone.id, 'x'),
      });
      items.push({
        label: `Split top / bottom (${Math.round(splitRatio * 100)}%)`,
        onselect: () => void split(zone.id, 'y'),
      });
      items.push({
        label: 'Set as merge source',
        onselect: () => {
          mergeMode = true;
          mergeSourceId = zone.id;
          selectedId = zone.id;
        },
      });
      items.push({
        label: 'Merge into selected',
        disabled: selectedId === null || selectedId === zone.id,
        onselect: () => {
          if (selectedId !== null) void commitMerge(zone.id, selectedId);
        },
      });
      items.push({
        label: 'Assign app…',
        onselect: () => {
          if (onpick) onpick(zone.id);
          else notice('Open the Apps panel to assign an application.');
        },
      });
      items.push({
        label: 'Unassign app',
        disabled: !zone.exe,
        onselect: () => void unassign(zone.id),
      });
      items.push({
        label: 'Remove zone',
        // The backend allows deleting the last zone; both delete surfaces
        // may empty a profile (F3).
        disabled: !canRemoveZone(zones.length),
        onselect: () => void remove(zone.id),
      });
    } else {
      items.push({ label: 'Add zone', onselect: () => void addZone() });
      items.push({ label: 'Undo', disabled: !snapshot?.can_undo, onselect: () => void undo() });
      items.push({ label: 'Redo', disabled: !snapshot?.can_redo, onselect: () => void redo() });
    }
    return items;
  });
</script>

<ContextMenu items={menuItems} class="relative h-full w-full">
  <svg
    bind:this={svgEl}
    class="block h-full w-full select-none"
    viewBox={`0 0 ${workArea.w} ${workArea.h}`}
    preserveAspectRatio="xMidYMid meet"
    role="application"
    aria-label="Zone layout canvas"
    onpointermove={onPointerMove}
    onpointerup={onPointerUp}
    onpointercancel={onPointerCancel}
    oncontextmenu={(event) => {
      // Letterboxed area outside the viewBox: no zone under the cursor.
      if (event.target === svgEl) contextZoneId = null;
    }}
  >
    <defs>
      <pattern
        id="zf-grid"
        width={workArea.w / Math.max(1, settings.grid_cols)}
        height={workArea.h / Math.max(1, settings.grid_rows)}
        patternUnits="userSpaceOnUse"
      >
        <path
          d={`M ${workArea.w / Math.max(1, settings.grid_cols)} 0 L 0 0 0 ${workArea.h / Math.max(1, settings.grid_rows)}`}
          fill="none"
          stroke="#22304a"
          stroke-opacity={mode === 'draw' ? 0.55 : 0.22}
          stroke-width="1"
        />
      </pattern>
    </defs>

    <!-- Background / draw surface -->
    <rect
      x={0}
      y={0}
      width={workArea.w}
      height={workArea.h}
      fill="var(--color-zf-bg)"
      onpointerdown={onBackgroundPointerDown}
      oncontextmenu={() => (contextZoneId = null)}
      role="presentation"
    />
    <rect x={0} y={0} width={workArea.w} height={workArea.h} fill="url(#zf-grid)" class="pointer-events-none" />

    {#if zones.length === 0}
      <g class="pointer-events-none" text-anchor="middle">
        <text
          x={workArea.w / 2}
          y={workArea.h / 2 - 14}
          fill="#93a4bd"
          font-size={Math.max(16, workArea.h * 0.028)}
          font-weight="600">No zones yet</text
        >
        <text
          x={workArea.w / 2}
          y={workArea.h / 2 + 16}
          fill="#6b7c96"
          font-size={Math.max(12, workArea.h * 0.018)}>Create your first zone with Add or Draw</text
        >
      </g>
    {/if}

    <SnapGuides
      {workArea}
      points={settings.snap_points}
      active={preview?.snap ?? null}
      emphasized={mode === 'draw' || preview !== null}
    />

    {#each zones as zone, index (zone.id)}
      <ZoneRect
        {zone}
        {index}
        {workArea}
        selected={selectedId === zone.id}
        dropTarget={dropTargetId === zone.id}
        mergeSource={mergeSourceId === zone.id}
        {boundaries}
        {outerHandles}
        tolerance={settings.boundary_tolerance_pct}
        inset={showGap ? gapWidth / 2 : 0}
        tint={settings.zone_tints[index % Math.max(1, settings.zone_tints.length)] ?? '#f59e0b'}
        onBodyPointerDown={onBodyPointerDown}
        onResizeStart={onResizeStart}
        onZoneContextMenu={(z, event) => {
          event.preventDefault();
          contextZoneId = z.id;
          // Do not overwrite `selectedId` here: "Merge into selected" needs the
          // previously selected zone to remain the merge target. All other
          // context actions operate on `contextZoneId` directly.
        }}
        onZoneDragOver={onZoneDragOver}
        onZoneDragLeave={onZoneDragLeave}
        onZoneDrop={onZoneDrop}
      />
    {/each}

    {#if preview}
      <DragPreview
        {workArea}
        zones={preview.zones}
        draw={preview.draw}
        tints={settings.zone_tints}
      />
    {/if}
  </svg>
</ContextMenu>
