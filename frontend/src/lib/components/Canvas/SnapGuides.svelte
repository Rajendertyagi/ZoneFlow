<script lang="ts">
  /** 25/50/75 guides (from settings) plus a HUD shown while resizing. */

  interface Props {
    workArea: { w: number; h: number };
    points: number[];
    active: { axis: 'v' | 'h'; at: number; snapped: number | null } | null;
    /** Show guides prominently in draw mode or during a resize. */
    emphasized: boolean;
  }

  let { workArea, points, active, emphasized }: Props = $props();

  const hudSize = $derived(Math.max(11, workArea.h * 0.018));
  const hudPad = $derived(Math.max(6, workArea.h * 0.01));

  const hud = $derived.by(() => {
    if (!active) return null;
    const isV = active.axis === 'v';
    const pos = (active.at / 100) * (isV ? workArea.w : workArea.h);
    const text = active.snapped !== null
      ? `${active.at.toFixed(1)}%  ·  snapped to ${active.snapped}%`
      : `${active.at.toFixed(1)}%`;
    const width = text.length * hudSize * 0.62 + hudPad * 2;
    const height = hudSize + hudPad * 1.6;
    // Keep the HUD inside the viewBox near the active guide.
    const x = isV
      ? Math.min(Math.max(pos + hudPad, 0), workArea.w - width)
      : Math.min(Math.max(workArea.w / 2 - width / 2, 0), workArea.w - width);
    const y = isV
      ? Math.min(Math.max(workArea.h / 2 - height / 2, 0), workArea.h - height)
      : Math.min(Math.max(pos + hudPad, 0), workArea.h - height);
    return { x, y, width, height, text };
  });
</script>

<g class="pointer-events-none">
  {#each points as point (point)}
    <line
      x1={(point / 100) * workArea.w}
      y1={0}
      x2={(point / 100) * workArea.w}
      y2={workArea.h}
      stroke="#f59e0b"
      stroke-opacity={active?.axis === 'v' && active.snapped === point ? 0.85 : emphasized ? 0.28 : 0.12}
      stroke-width={active?.axis === 'v' && active.snapped === point ? 2 : 1}
      stroke-dasharray={`${Math.max(4, workArea.w * 0.004)} ${Math.max(6, workArea.w * 0.006)}`}
    />
    <line
      x1={0}
      y1={(point / 100) * workArea.h}
      x2={workArea.w}
      y2={(point / 100) * workArea.h}
      stroke="#f59e0b"
      stroke-opacity={active?.axis === 'h' && active.snapped === point ? 0.85 : emphasized ? 0.28 : 0.12}
      stroke-width={active?.axis === 'h' && active.snapped === point ? 2 : 1}
      stroke-dasharray={`${Math.max(4, workArea.w * 0.004)} ${Math.max(6, workArea.w * 0.006)}`}
    />
  {/each}

  {#if hud}
    <rect x={hud.x} y={hud.y} width={hud.width} height={hud.height} rx={hud.height / 3} fill="#0b1322" fill-opacity={0.94} stroke="#f59e0b" stroke-opacity={0.7} stroke-width={1} />
    <text x={hud.x + hudPad} y={hud.y + hud.height / 2} fill="#fbbf24" font-size={hudSize} font-family="var(--font-mono)" dominant-baseline="central">{hud.text}</text>
  {/if}
</g>
