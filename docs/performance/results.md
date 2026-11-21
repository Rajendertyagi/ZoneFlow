# ZoneFlow — Performance results (three-workload model)

**Status:** measurement record for the Rust + Axum + Svelte architecture. Phase 6
(2026-11-22) measured the **Background** workload on the final egui-free release
binary; the two browser-dependent workloads remain **NOT MEASURED** (reasons below).
**Method:** `docs/performance/benchmark.ps1` (three-workload harness) and the
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
| **Final implementation (Phase 6, egui removed)** | **12.0 / 2.0 MiB** | **11.9 / 2.0 MiB** | **11.9 / 2.0 MiB** | **0.00–0.10%** | **3,827,200 B (3.65 MiB)** |

The spike contains the real backend modules (`config`, `engine_core`, `engine`,
`system`, `autostart`), tray, global hotkeys, an axum server on an ephemeral
loopback port, and a 50 ms poll loop. It was verified live: tray/hotkeys
initialised without error and HTTP responded. It has **no GUI toolkit**.

The **final implementation** row is the actual Phase 6 measurement of the shipped
binary (`target\release\screenlayout.exe`, SHA-256
`D11618346125C83382268449EC55EB09900D62758BBBAFE49831D65A2722B43E`), taken with
`docs/performance/benchmark.ps1 -Mode Background -Exe target\release\screenlayout.exe`
run three consecutive times. It is the complete always-on process: command
worker, Tokio/Axum server, dynamic tray, global hotkeys, native message loop and
the layered drag overlay — with the egui/eframe/shadcn-egui dependencies and code
removed. Range: **11.9–12.0 MiB working set, 2.0 MiB private, 0.00–0.10% CPU**.

Interpretation: removing the resident GUI cuts the always-on process from
~60 MiB private to ~2 MiB private and the executable from 14.21 MiB to 3.65 MiB.
This is the always-on process only — **not** the memory of the complete
application with the browser editor open (Workload 2, not measured).

### Workload 2 — Editor open

**NOT MEASURED (Phase 6).** The Svelte frontend and the `-Mode EditorOpen` harness
both exist, but a faithful measurement was not feasible in this environment:

1. The per-launch session token is **not written to the log** (only the bound
   port is), so the tokenised editor URL cannot be reconstructed from
   `zoneflow.log`. The URL is held only in process memory and passed to the
   command worker.
2. The tray **"Open editor…"** action could not be driven reliably from the
   benchmark harness: the native `muda` popup menu is not exposed through UI
   Automation, and synthetic keystroke navigation did not fire the action.
3. **33 pre-existing Microsoft Edge processes** (long-running user sessions)
   were present. `benchmark.ps1` attributes browser cost only to **new** PIDs
   created by the run; opening a URL reuses the existing Edge instance, so
   browser WS/private/CPU would be reported as 0 — a misleading figure. Closing
   those sessions to get clean attribution is out of scope for this task.

No numbers are claimed for this workload. To measure it later: log the editor
URL (or expose it via a dev-only hook), close all browser windows first, then run
`benchmark.ps1 -Mode EditorOpen -Url <tokenised-url>` ≥3 times and record the Rust
process and browser process tree **separately**.

### Workload 3 — Pointer activity (not editing)

**NOT MEASURED (Phase 6).** Same blockers as Workload 2: no way to obtain the
tokenised URL, no reliable automated tray action, and 33 pre-existing Edge
processes that defeat the harness's new-PID-only browser attribution. The harness
mode (`-Mode PointerActivity`) moves the cursor randomly for 20 s and reports
`Pointer*` fields sampled **after** the movement; it does **not** drag or resize
a zone, so it would measure cursor-driven repaint cost only. The historical egui
active-CPU figures (1.01–46.14%) were noisy under synthetic cursor input and are
**not** comparable to a real SVG-canvas editing workload. No numbers are claimed.

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

- Idle/pointer RAM: `WorkingSet64` and `PrivateMemorySize64` sampled at the end of
  a 30 s window. Baselines are taken immediately before that window — after the
  browser has launched and settled for Editor open / Pointer activity — so CPU
  deltas cover only the measurement window.
- CPU: `TotalProcessorTime` delta / wall time × 100 over the same window. Browser
  CPU is reported for Editor open as well as Pointer activity.
- Startup: 50 ms window-visibility polling gives ±50 ms granularity; report as a
  range, never to tenths of a millisecond.
- Run ≥3 consecutive measurements and report the range, not a single value.
- Browser process trees must be summed by process name and reported separately
  from the Rust process.
