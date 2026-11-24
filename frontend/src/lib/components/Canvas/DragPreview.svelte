<script lang="ts">
  /**
   * Optimistic local preview. Rendered only from in-memory `$state` during a
   * pointer gesture — no HTTP request is issued until `pointerup` commits via
   * the typed client. This is what keeps dragging smooth and offline-safe.
   */
  import type { RectPct } from './geometry';

  interface Props {
    workArea: { w: number; h: number };
    /** Preview geometry overrides keyed by zone id (move / resize). */
    zones: Record<number, RectPct>;
    /** In-progress rectangle for draw mode. */
    draw: RectPct | null;
    tints: string[];
  }

  let { workArea, zones, draw, tints }: Props = $props();

  const entries = $derived(Object.entries(zones).map(([id, rect]) => ({ id: Number(id), rect })));
  const strokeW = $derived(Math.max(2.5, workArea.w * 0.0022));
  const labelSize = $derived(Math.max(10, workArea.h * 0.016));
  const labelPad = $derived(Math.max(6, labelSize * 0.4));

  /** Width of the coordinate readout's backdrop, sized from the text itself. */
  function labelWidth(text: string): number {
    return text.length * labelSize * 0.6 + labelPad * 2;
  }

  function px(rect: RectPct) {
    return {
      x: (rect.x / 100) * workArea.w,
      y: (rect.y / 100) * workArea.h,
      w: (rect.w / 100) * workArea.w,
      h: (rect.h / 100) * workArea.h,
    };
  }
</script>

<g class="pointer-events-none">
  {#each entries as entry (entry.id)}
    {@const r = px(entry.rect)}
    {@const coordText = `${entry.rect.x.toFixed(1)}, ${entry.rect.y.toFixed(1)} · ${entry.rect.w.toFixed(1)}×${entry.rect.h.toFixed(1)}`}
    {@const boxW = labelWidth(coordText)}
    {@const boxH = labelSize + labelPad * 1.2}
    <rect
      x={r.x}
      y={r.y}
      width={r.w}
      height={r.h}
      fill={tints[(entry.id - 1) % Math.max(1, tints.length)] ?? 'var(--canvas-preview)'}
      fill-opacity={0.2}
      stroke="var(--canvas-preview)"
      stroke-width={strokeW}
      stroke-dasharray={`${Math.max(7, workArea.w * 0.007)} ${Math.max(5, workArea.w * 0.005)}`}
      rx={Math.max(2, workArea.w * 0.002)}
    />
    <rect
      x={r.x + r.w / 2 - boxW / 2}
      y={r.y + r.h / 2 - boxH / 2}
      width={boxW}
      height={boxH}
      rx={boxH / 3}
      fill="var(--canvas-label-bg)"
      fill-opacity={0.88}
      stroke="var(--canvas-preview)"
      stroke-opacity={0.5}
      stroke-width={1}
    />
    <text x={r.x + r.w / 2} y={r.y + r.h / 2} fill="var(--canvas-label-fg)" font-size={labelSize} font-family="var(--font-mono)" text-anchor="middle" dominant-baseline="central">
      {coordText}
    </text>
  {/each}

  {#if draw}
    {@const r = px(draw)}
    {@const drawText = `${draw.w.toFixed(1)} × ${draw.h.toFixed(1)}%`}
    {@const boxW = labelWidth(drawText)}
    {@const boxH = labelSize + labelPad * 1.2}
    <rect
      x={r.x}
      y={r.y}
      width={r.w}
      height={r.h}
      fill="var(--canvas-preview)"
      fill-opacity={0.18}
      stroke="var(--canvas-preview)"
      stroke-width={strokeW}
      stroke-dasharray={`${Math.max(7, workArea.w * 0.007)} ${Math.max(5, workArea.w * 0.005)}`}
      rx={Math.max(2, workArea.w * 0.002)}
    />
    <rect
      x={r.x + r.w / 2 - boxW / 2}
      y={r.y + r.h / 2 - boxH / 2}
      width={boxW}
      height={boxH}
      rx={boxH / 3}
      fill="var(--canvas-label-bg)"
      fill-opacity={0.88}
      stroke="var(--canvas-preview)"
      stroke-opacity={0.5}
      stroke-width={1}
    />
    <text x={r.x + r.w / 2} y={r.y + r.h / 2} fill="var(--canvas-label-fg)" font-size={labelSize} font-family="var(--font-mono)" text-anchor="middle" dominant-baseline="central">
      {drawText}
    </text>
  {/if}
</g>
