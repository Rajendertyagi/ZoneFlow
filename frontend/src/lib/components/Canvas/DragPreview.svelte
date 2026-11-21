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
  const strokeW = $derived(Math.max(2, workArea.w * 0.0018));
  const labelSize = $derived(Math.max(10, workArea.h * 0.016));

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
    <rect
      x={r.x}
      y={r.y}
      width={r.w}
      height={r.h}
      fill={tints[(entry.id - 1) % Math.max(1, tints.length)] ?? '#f59e0b'}
      fill-opacity={0.16}
      stroke="#fbbf24"
      stroke-width={strokeW}
      stroke-dasharray={`${Math.max(7, workArea.w * 0.007)} ${Math.max(5, workArea.w * 0.005)}`}
      rx={Math.max(2, workArea.w * 0.002)}
    />
    <text x={r.x + r.w / 2} y={r.y + r.h / 2} fill="#fbbf24" font-size={labelSize} font-family="var(--font-mono)" text-anchor="middle" dominant-baseline="central">
      {entry.rect.x.toFixed(1)}, {entry.rect.y.toFixed(1)} · {entry.rect.w.toFixed(1)}×{entry.rect.h.toFixed(1)}
    </text>
  {/each}

  {#if draw}
    {@const r = px(draw)}
    <rect
      x={r.x}
      y={r.y}
      width={r.w}
      height={r.h}
      fill="#f59e0b"
      fill-opacity={0.14}
      stroke="#fbbf24"
      stroke-width={strokeW}
      stroke-dasharray={`${Math.max(7, workArea.w * 0.007)} ${Math.max(5, workArea.w * 0.005)}`}
      rx={Math.max(2, workArea.w * 0.002)}
    />
    <text x={r.x + r.w / 2} y={r.y + r.h / 2} fill="#fbbf24" font-size={labelSize} font-family="var(--font-mono)" text-anchor="middle" dominant-baseline="central">
      {draw.w.toFixed(1)} × {draw.h.toFixed(1)}%
    </text>
  {/if}
</g>
