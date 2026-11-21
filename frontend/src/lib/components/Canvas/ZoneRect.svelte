<script lang="ts">
  import type { ZoneDto } from '$lib/api/client';
  import type { SharedBoundary } from './geometry';

  interface Props {
    zone: ZoneDto;
    index: number;
    workArea: { w: number; h: number };
    selected: boolean;
    dropTarget: boolean;
    mergeSource: boolean;
    /** Shared boundaries this zone participates in (for resize handles). */
    boundaries: SharedBoundary[];
    /** Boundary-detection tolerance from settings (percent). */
    tolerance: number;
    /** Visual inset applied to the body when the gap preview is on (user units). */
    inset: number;
    tint: string;
    onBodyPointerDown: (zone: ZoneDto, event: PointerEvent) => void;
    onResizeStart: (boundary: SharedBoundary, event: PointerEvent) => void;
    onZoneContextMenu: (zone: ZoneDto, event: MouseEvent) => void;
    onZoneDragOver: (zone: ZoneDto, event: DragEvent) => void;
    onZoneDragLeave: (zone: ZoneDto, event: DragEvent) => void;
    onZoneDrop: (zone: ZoneDto, event: DragEvent) => void;
  }

  let {
    zone,
    index,
    workArea,
    selected,
    dropTarget,
    mergeSource,
    boundaries,
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
  const handleSize = $derived(Math.max(9, workArea.w * 0.006));

  /** Which of this zone's edges sit on a shared boundary. */
  const handles = $derived.by(() => {
    const tol = tolerance;
    const out: Array<{ boundary: SharedBoundary; x: number; y: number; w: number; h: number }> = [];
    for (const b of boundaries) {
      if (b.axis === 'v') {
        const by = (b.start / 100) * workArea.h;
        const bh = ((b.end - b.start) / 100) * workArea.h;
        if (Math.abs(zone.x - b.at) <= tol) {
          out.push({ boundary: b, x: (b.at / 100) * workArea.w - handleSize / 2, y: by, w: handleSize, h: bh });
        }
        if (Math.abs(zone.x + zone.w - b.at) <= tol) {
          out.push({ boundary: b, x: (b.at / 100) * workArea.w - handleSize / 2, y: by, w: handleSize, h: bh });
        }
      } else {
        const bx = (b.start / 100) * workArea.w;
        const bw = ((b.end - b.start) / 100) * workArea.w;
        if (Math.abs(zone.y - b.at) <= tol) {
          out.push({ boundary: b, x: bx, y: (b.at / 100) * workArea.h - handleSize / 2, w: bw, h: handleSize });
        }
        if (Math.abs(zone.y + zone.h - b.at) <= tol) {
          out.push({ boundary: b, x: bx, y: (b.at / 100) * workArea.h - handleSize / 2, w: bw, h: handleSize });
        }
      }
    }
    return out;
  });

  const stroke = $derived(dropTarget ? '#f59e0b' : mergeSource ? '#c792ea' : selected ? '#f59e0b' : '#33445f');
  const strokeWidth = $derived(dropTarget || selected || mergeSource ? Math.max(2, workArea.w * 0.0016) : 1);
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
    fill-opacity={selected || dropTarget ? 0.2 : 0.1}
    {stroke}
    stroke-width={strokeWidth}
    stroke-dasharray={dropTarget ? `${Math.max(6, workArea.w * 0.006)} ${Math.max(4, workArea.w * 0.004)}` : 'none'}
    rx={Math.max(2, workArea.w * 0.002)}
  />

  <!-- Accent bar along the top edge -->
  <rect {x} {y} width={w} height={barH} fill={tint} fill-opacity={0.95} rx={barH / 2} />

  <!-- Number badge -->
  <g class="pointer-events-none" transform={`translate(${x + pad}, ${y + pad})`}>
    <rect width={badgeSize} height={badgeSize} rx={badgeSize / 3} fill="#0b1322" fill-opacity={0.85} stroke={tint} stroke-width={1} />
    <text
      x={badgeSize / 2}
      y={badgeSize / 2}
      fill={tint}
      font-size={badgeSize * 0.5}
      font-weight="700"
      text-anchor="middle"
      dominant-baseline="central">{index + 1}</text
    >
  </g>

  <!-- Label + executable -->
  <g class="pointer-events-none" transform={`translate(${x + pad}, ${y + pad + badgeSize + pad * 0.6})`}>
    <text fill="#e6edf7" font-size={labelSize} font-weight="600">{zone.label || `Zone ${index + 1}`}</text>
    <text y={labelSize * 1.25} fill="#93a4bd" font-size={subSize} font-family="var(--font-mono)">
      {zone.exe || 'unassigned'}
    </text>
  </g>

  {#if mergeSource}
    <g class="pointer-events-none" transform={`translate(${x + w - pad}, ${y + pad})`}>
      <rect x={-badgeSize * 1.5} width={badgeSize * 1.5} height={badgeSize * 0.7} rx={badgeSize / 4} fill="#c792ea" />
      <text x={-badgeSize * 0.75} y={badgeSize * 0.35} fill="#1a1204" font-size={badgeSize * 0.34} font-weight="700" text-anchor="middle" dominant-baseline="central">SRC</text>
    </g>
  {/if}

  <!-- Resize handles on shared boundaries -->
  {#each handles as handle, i (i)}
    <rect
      x={handle.x}
      y={handle.y}
      width={handle.w}
      height={handle.h}
      fill="#f59e0b"
      fill-opacity={0.18}
      stroke="#f59e0b"
      stroke-opacity={0.55}
      stroke-width={1}
      style:cursor={handle.boundary.axis === 'v' ? 'col-resize' : 'row-resize'}
      role="presentation"
      onpointerdown={(event) => {
        event.stopPropagation();
        onResizeStart(handle.boundary, event);
      }}
    />
  {/each}
</g>
