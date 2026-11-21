# ZoneFlow — Performance results (three-workload model)

**Status:** measurement record for the Rust + Axum + Svelte architecture evaluation.
**Method:** `docs/performance/benchmark.ps1` (new three-workload harness) and the
historical `docs/performance/baseline.ps1` (egui baseline). All figures measured
on the same machine; raw values are recorded, not targets.

## Environment

| Item | Value |
|---|---|
| OS | Windows 11 Pro, 10.0.26300 (64-bit) |
| CPU | 12th Gen Intel Core i3-12100, 4 cores / 8 logical |
| RAM | 31.6 GB physical |
| Toolchain | rustc 1.99.1, cargo 1.99.1, host `x86_64-pc-windows-msvc` |
| Release profile | `opt-level=2`, `lto=true`, `strip=true` |

## Workload definitions

1. **Background** — Rust backend, tray, hotkeys and HTTP server; browser closed.
   This is the always-on process and the workload the new architecture targets.
2. **Editor open** — Rust backend plus browser with the full Svelte interface loaded.
3. **Pointer activity** — editor open plus 20 s of synthetic cursor movement. This
   is **not** an editing workload: the cursor does not drag or resize a zone. Real
   SVG zone-boundary dragging is a separate functional test (dev-only browser
   automation, never an installed runtime dependency). Do not describe this
   workload as editing.

These must never be reported as one number. The headless backend's RAM reduction
is **not** the memory consumption of the complete application.

## Raw results

### Workload 1 — Background

| Build | Run 1 WS / private | Run 2 WS / private | Run 3 WS / private | CPU | Exe |
|---|---|---|---|---|---|
| Current egui app (GUI resident, `1976f67`) | 109.4 / 58.8 MiB | 111.8 / 61.6 MiB | 111.4 / 61.3 MiB | 0.26–0.47% | 14,901,248 B (14.21 MiB) |
| Headless Rust + Axum spike | 12.5 / 2.1 MiB | 10.9 / 2.0 MiB | 10.9 / 2.0 MiB | 0.05–0.42% | 1,490,432 B (1.42 MiB) |

The spike contains the real backend modules (`config`, `engine_core`, `engine`,
`system`, `autostart`), tray, global hotkeys, an axum server on an ephemeral
loopback port, and a 50 ms poll loop. It was verified live: tray/hotkeys
initialised without error and HTTP responded. It has **no GUI toolkit**.

Interpretation: removing the resident GUI cuts the always-on process from
~60 MiB private to ~2 MiB private. This is the architectural motivation for the
new direction. It is a floor measurement, not the final application.

### Workload 2 — Editor open

**Not yet measured.** The Svelte frontend does not exist yet. The harness is
ready (`-Mode EditorOpen -Url <tokenised-url>`); record results here when the
frontend is built. Measure the Rust process and the browser process tree
separately — summing them into one figure hides the trade-off.

### Workload 3 — Pointer activity (not editing)

**Not yet measured.** Requires the frontend. The harness mode is
`-Mode PointerActivity`; it moves the cursor randomly for 20 s and reports
`Pointer*` fields sampled **after** the movement. It does **not** drag or resize
a zone, so it measures cursor-driven repaint cost only. Browser processes are
attributed only to PIDs created by the run — close all browser windows first, or
the script warns that attribution may be incomplete. The historical egui
active-CPU figures (1.01–46.14%) were noisy under synthetic cursor input and are
**not** comparable to a real SVG-canvas editing workload.

**Real editing verification (separate functional test, not a benchmark):** a
dev-only browser automation script (e.g. Playwright, test-only dependency) should
perform a real SVG zone-boundary drag, assert the frontend interaction completes,
and assert the backend receives and persists the geometry update. Resource usage
during that test may be recorded later; no values are claimed here.

## Historical egui baseline (pre-change, `363201c`)

Recorded in `baseline.md`: exe 14,857,216 B (14.17 MiB); cold startup ~190–250 ms;
idle ~120 MiB WS / ~61 MiB private / <0.3% CPU; active ~123 MiB WS / ~64 MiB
private with highly variable CPU. Startup and RAM were stable; active CPU was not.

## Measurement notes

- Idle/active RAM: `WorkingSet64` and `PrivateMemorySize64` sampled at the end of
  a 30 s window after a 5 s settle.
- CPU: `TotalProcessorTime` delta / wall time × 100 over the same window.
- Startup: 50 ms window-visibility polling gives ±50 ms granularity; report as a
  range, never to tenths of a millisecond.
- Run ≥3 consecutive measurements and report the range, not a single value.
- Browser process trees must be summed by process name and reported separately
  from the Rust process.
