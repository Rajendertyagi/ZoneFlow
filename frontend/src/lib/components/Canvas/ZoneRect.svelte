<script lang="ts">
  import type { ZoneDto } from '$lib/api/client';
  import type { OuterResizeHandle, ResizeHandle, SharedBoundary } from './geometry';

  interface Props {
    zone: ZoneDto;
    index: number;
    workArea: { w: number; h: number };
    /** Grid dimensions for the grid-cell readout (12×8 fallback from ZoneCanvas). */
    gridCols: number;
    gridRows: number;
    selected: boolean;
    dropTarget: boolean;
    mergeSource: boolean;
    /** Shared boundaries this zone participates in (for resize handles). */
    boundaries: SharedBoundary[];
    /** Outer-edge handles for every zone; this component renders its own. */
    outerHandles: OuterResizeHandle[];
    /** Boundary-detection tolerance from settings (percent). */
    tolerance: number;
    /** Visual inset applied to the body when the gap preview is on (user units). */
    inset: number;
    tint: string;
    onBodyPointerDown: (zone: ZoneDto, event: PointerEvent) => void;
    onResizeStart: (handle: ResizeHandle, event: PointerEvent) => void;
    onZoneContextMenu: (zone: ZoneDto, event: MouseEvent) => void;
    onZoneDragOver: (zone: ZoneDto, event: DragEvent) => void;
    onZoneDragLeave: (zone: ZoneDto, event: DragEvent) => void;
    onZoneDrop: (zone: ZoneDto, event: DragEvent) => void;
  }

  let {
    zone,
    index,
    workArea,
    gridCols,
    gridRows,
    selected,
    dropTarget,
    mergeSource,
    boundaries,
    outerHandles,
    tolerance,
    inset,
    tint,
    onBodyPointerDown,
    onResizeStart,
    onZoneContextMenu,
    onZoneDragOver,
    onZoneDragLeave,
    onZoneDrop,
  }: Props = $props();

  // Percent -> SVG user units (the viewBox is the work area in pixels).
  // `inset` is a display-only gap; geometry/handles stay at true positions.
  const x = $derived((zone.x / 100) * workArea.w + inset);
  const y = $derived((zone.y / 100) * workArea.h + inset);
  const w = $derived(Math.max(0, (zone.w / 100) * workArea.w - inset * 2));
  const h = $derived(Math.max(0, (zone.h / 100) * workArea.h - inset * 2));

  const labelSize = $derived(Math.max(11, workArea.h * 0.026));
  const subSize = $derived(Math.max(9, workArea.h * 0.017));
  const badgeSize = $derived(Math.max(20, workArea.h * 0.032));
  const pad = $derived(Math.max(8, workArea.h * 0.012));
  const barH = $derived(Math.max(3, workArea.h * 0.006));
  // Wider grab strips improve discoverability; still display-only geometry.
  const handleSize = $derived(Math.max(12, workArea.w * 0.0075));

  // Dedicated 12px-step font for the zone info readout (best practice #3):
  // subSize (~9px) was too small and sat off the type scale.
  const infoFont = $derived(Math.max(12, workArea.h * 0.022));
  const infoLineH = $derived(infoFont * 1.3);
  const infoPad = 8; // 8pt internal padding (best practice #1)

  // Per-zone info readout (V3). Uses the authoritative zone percentages (not
  // the display-only `inset`) so the numbers reflect the real layout.
  const pxW = $derived((zone.w / 100) * workArea.w);
  const pxH = $derived((zone.h / 100) * workArea.h);
  const infoLines = $derived.by((): string[] => {
    const px = `${Math.round(pxW)} × ${Math.round(pxH)} px`;
    const pct = `${Math.round(zone.w * 10) / 10}% × ${Math.round(zone.h * 10) / 10}%`;
    const pos = `x ${Math.round(zone.x * 10) / 10} · y ${Math.round(zone.y * 10) / 10}`;
    const cells = `${Math.max(1, Math.round((zone.w / 100) * gridCols))} × ${Math.max(1, Math.round((zone.h / 100) * gridRows))}`;
    // Priority order: px size first. If vertical room is short, drop lines
    // from the bottom rather than wrapping.
    const all = [px, pct, pos, cells];
    const fit = Math.max(1, Math.floor((h - pad * 2 - infoFont) / infoLineH));
    return all.slice(0, Math.min(4, fit));
  });
  const showInfo = $derived(w >= 180 && h >= 140 && infoLines.length > 0);

  // Backing rect geometry: sized to the widest line and the visible line count,
  // with 8px internal padding. The last text baseline keeps a full line of
  // clearance from the zone edge so it is never clipped.
  const infoLastY = $derived(y + h - pad - infoFont);
  const infoFirstY = $derived(infoLastY - (infoLines.length - 1) * infoLineH);
  const infoBoxW = $derived(Math.max(0, ...infoLines.map((l) => l.length)) * infoFont * 0.6 + infoPad * 2);
  const infoBoxH = $derived((infoLines.length - 1) * infoLineH + infoFont * 1.25 + infoPad * 2);
  const infoBoxX = $derived(x + w - pad + infoPad - infoBoxW);
  const infoBoxY = $derived(infoFirstY - infoFont - infoPad);

  interface RenderHandle {
    handle: ResizeHandle;
    x: number;
    y: number;
    w: number;
    h: number;
  }

  /**
   * Shared dividers (amber, centred on the edge) plus this zone's outer edges
   * (blue, inset so the whole grip stays inside the work area). Both feed the
   * same pointer handler; ZoneCanvas dispatches on `handle.kind`.
   */
  const renderHandles = $derived.by((): RenderHandle[] => {
    const tol = tolerance;
    const out: RenderHandle[] = [];

    for (const b of boundaries) {
      if (b.axis === 'v') {
        if (Math.abs(zone.x - b.at) <= tol || Math.abs(zone.x + zone.w - b.at) <= tol) {
          out.push({
            handle: { kind: 'shared', axis: 'v', at: b.at, start: b.start, end: b.end, boundary: b },
            x: (b.at / 100) * workArea.w - handleSize / 2,
            y: (b.start / 100) * workArea.h,
            w: handleSize,
            h: ((b.end - b.start) / 100) * workArea.h,
          });
        }
      } else if (Math.abs(zone.y - b.at) <= tol || Math.abs(zone.y + zone.h - b.at) <= tol) {
        out.push({
          handle: { kind: 'shared', axis: 'h', at: b.at, start: b.start, end: b.end, boundary: b },
          x: (b.start / 100) * workArea.w,
          y: (b.at / 100) * workArea.h - handleSize / 2,
          w: ((b.end - b.start) / 100) * workArea.w,
          h: handleSize,
        });
      }
    }

    for (const h of outerHandles) {
      if (h.zoneId !== zone.id) continue;
      if (h.axis === 'v') {
        out.push({
          handle: h,
          x: h.edge === 'low' ? (h.at / 100) * workArea.w : (h.at / 100) * workArea.w - handleSize,
          y: (h.start / 100) * workArea.h,
          w: handleSize,
          h: ((h.end - h.start) / 100) * workArea.h,
        });
      } else {
        out.push({
          handle: h,
          x: (h.start / 100) * workArea.w,
          y: h.edge === 'low' ? (h.at / 100) * workArea.h : (h.at / 100) * workArea.h - handleSize,
          w: ((h.end - h.start) / 100) * workArea.w,
          h: handleSize,
        });
      }
    }
    return out;
  });

  const stroke = $derived(
    dropTarget
      ? 'var(--canvas-zone-drop)'
      : mergeSource
        ? 'var(--canvas-zone-merge)'
        : selected
          ? 'var(--canvas-zone-selected)'
          : 'var(--canvas-zone-stroke)',
  );
  // Active states carry a clearly heavier stroke than the resting outline so
  // selected / drop / merge stay legible at every zoom level.
  const strokeWidth = $derived(
    dropTarget || selected || mergeSource
      ? Math.max(3, workArea.w * 0.0022)
      : Math.max(1.5, workArea.w * 0.0009),
  );
</script>

<g
  role="button"
  tabindex="-1"
  aria-label={`Zone ${index + 1}: ${zone.label || 'unassigned'}`}
  onpointerdown={(event) => onBodyPointerDown(zone, event)}
  oncontextmenu={(event) => onZoneContextMenu(zone, event)}
  ondragover={(event) => onZoneDragOver(zone, event)}
  ondragenter={(event) => onZoneDragOver(zone, event)}
  ondragleave={(event) => onZoneDragLeave(zone, event)}
  ondrop={(event) => onZoneDrop(zone, event)}
  style="cursor: grab"
>
  <!-- Tinted body -->
  <rect
    {x}
    {y}
    width={w}
    height={h}
    fill={tint}
    fill-opacity={dropTarget ? 0.24 : selected ? 0.2 : mergeSource ? 0.16 : 0.08}
    {stroke}
    stroke-width={strokeWidth}
    stroke-dasharray={dropTarget ? `${Math.max(6, workArea.w * 0.006)} ${Math.max(4, workArea.w * 0.004)}` : 'none'}
    rx={Math.max(2, workArea.w * 0.002)}
  />

  <!-- Accent bar along the top edge -->
  <rect {x} {y} width={w} height={barH} fill={tint} fill-opacity={0.95} rx={barH / 2} />

  <!-- Number badge: opaque label background + high-contrast label foreground. -->
  <g class="pointer-events-none" transform={`translate(${x + pad}, ${y + pad})`}>
    <rect width={badgeSize} height={badgeSize} rx={badgeSize / 3} fill="var(--canvas-label-bg)" fill-opacity={0.95} stroke={tint} stroke-width={1} />
    <text
      x={badgeSize / 2}
      y={badgeSize / 2}
      fill="var(--canvas-label-fg)"
      font-size={badgeSize * 0.5}
      font-weight="700"
      text-anchor="middle"
      dominant-baseline="central">{index + 1}</text
    >
  </g>

  <!-- Label + executable. A thin canvas-label-bg halo keeps text readable over
       any zone tint without introducing a new colour or a visible box. -->
  <g class="pointer-events-none" transform={`translate(${x + pad}, ${y + pad + badgeSize + pad * 0.6})`}>
    <text
      fill="var(--canvas-label-fg)"
      font-size={labelSize}
      font-weight="600"
      stroke="var(--canvas-label-bg)"
      stroke-width={Math.max(2, labelSize * 0.18)}
      stroke-linejoin="round"
      paint-order="stroke">{zone.label || `Zone ${index + 1}`}</text
    >
    <text
      y={labelSize * 1.25}
      fill="var(--muted-foreground)"
      font-size={subSize}
      font-family="var(--font-mono)"
      stroke="var(--canvas-label-bg)"
      stroke-width={Math.max(1.5, subSize * 0.18)}
      stroke-linejoin="round"
      paint-order="stroke">
      {zone.exe || 'unassigned'}
    </text>
  </g>

  <!-- Per-zone info readout (V3): px size, percentage, x/y, grid cells.
       Bottom-right anchored on the 12px type step, tabular numerics, two
       weights, with a subtle translucent backing. pointer-events-none keeps
       hit-testing and drag/resize unchanged. Hidden on small zones. -->
  {#if showInfo}
    <g class="pointer-events-none" text-anchor="end">
      <!-- Backing rect first so the text renders on top of it. -->
      <rect
        x={infoBoxX}
        y={infoBoxY}
        width={infoBoxW}
        height={infoBoxH}
        rx={4}
        fill="var(--canvas-label-bg)"
        fill-opacity={0.7}
      />
      {#each infoLines as line, i (i)}
        <text
          x={x + w - pad}
          y={infoLastY - (infoLines.length - 1 - i) * infoLineH}
          fill={i === 0 ? 'var(--canvas-label-fg)' : 'var(--muted-foreground)'}
          font-size={infoFont}
          font-family="var(--font-mono)"
          style="font-variant-numeric: tabular-nums"
          font-weight={i === 0 ? 600 : 400}
          stroke="var(--canvas-label-bg)"
          stroke-width={Math.max(1.5, infoFont * 0.18)}
          stroke-linejoin="round"
          paint-order="stroke">{line}</text
        >
      {/each}
    </g>
  {/if}

  {#if mergeSource}
    <g class="pointer-events-none" transform={`translate(${x + w - pad}, ${y + pad})`}>
      <rect x={-badgeSize * 1.5} width={badgeSize * 1.5} height={badgeSize * 0.7} rx={badgeSize / 4} fill="var(--canvas-zone-merge)" />
      <text x={-badgeSize * 0.75} y={badgeSize * 0.35} fill="var(--primary-foreground)" font-size={badgeSize * 0.34} font-weight="700" text-anchor="middle" dominant-baseline="central">SRC</text>
    </g>
  {/if}

  <!-- Resize handles: amber = shared divider, blue = outer zone edge -->
  {#each renderHandles as rh, i (i)}
    <rect
      x={rh.x}
      y={rh.y}
      width={rh.w}
      height={rh.h}
      fill={rh.handle.kind === 'outer' ? 'var(--canvas-handle-outer)' : 'var(--canvas-handle-shared)'}
      fill-opacity={rh.handle.kind === 'outer' ? 0.2 : 0.24}
      stroke={rh.handle.kind === 'outer' ? 'var(--canvas-handle-outer)' : 'var(--canvas-handle-shared)'}
      stroke-opacity={0.85}
      stroke-width={1.5}
      style:cursor={rh.handle.axis === 'v' ? 'col-resize' : 'row-resize'}
      role="presentation"
      onpointerdown={(event) => {
        event.stopPropagation();
        onResizeStart(rh.handle, event);
      }}
    />
  {/each}
</g>
