# ZoneFlow — Performance Baseline (pre-WS-A)

**Recorded:** 2026-11-21 · **Commit:** `363201c` (clean tree, latest `main`)
**Purpose:** trustworthy pre-change baseline for the WS-A backend-correctness work. No targets are set here — this is measurement only.
**Current status:** historical egui baseline. For the three-workload model (background / editor open / pointer activity) and the headless Rust + Axum spike results, see `results.md` and `benchmark.ps1` in this directory.

## Environment

| Item | Value |
|---|---|
| OS | Windows 11 Pro, 10.0.26300 (64-bit) |
| CPU | 12th Gen Intel Core i3-12100, 4 cores / 8 logical |
| RAM | 31.6 GB physical |
| Toolchain | rustc 1.99.1, cargo 1.99.1, host `x86_64-pc-windows-msvc` |
| Release profile | `opt-level=2`, `lto=true`, `strip=true` (Cargo.toml) |
| Renderer | eframe/egui 0.37 + glow (`default-features=false, features=["glow",...]`) |

## Method

Reproducible script: `docs/performance/baseline.ps1` (run from repo root). It launches `target/release/screenlayout.exe` and:

- **Cold startup:** `Stopwatch` from `Start-Process` until the main window handle is non-zero **and** `IsWindowVisible` — i.e. GUI readiness, not just process launch. 5s settle afterwards.
- **Idle RAM/CPU:** after settle, 30s with no input. `WorkingSet64` / `PrivateMemorySize64` sampled at window end; CPU = `TotalProcessorTime` delta / wall time × 100.
- **Active RAM/CPU:** 20s of synthetic mouse movement over the window canvas (random positions, 40ms interval) to drive repaint/interaction work; same counters. Cursor restored and app terminated afterwards.

Three consecutive runs were taken; the median/range is reported.

## Results

| Metric | Run 1 | Run 2 | Run 3 | Reported |
|---|---|---|---|---|
| Release exe size | — | — | — | **14,857,216 bytes (14.17 MiB)** |
| Cold startup → window visible | ~240 ms | ~190 ms | ~250 ms | **~190–250 ms** |
| Idle working set | 119.9 MiB | 119.8 MiB | 119.5 MiB | **~120 MiB** |
| Idle private bytes | 61.4 MiB | 61.2 MiB | 61.0 MiB | **~61 MiB** |
| Idle CPU | 0.21% | 0.16% | 0.05% | **< 0.3%** |
| Active working set | 122.9 MiB | 122.9 MiB | 122.8 MiB | **~123 MiB** |
| Active private bytes | 64.4 MiB | 64.2 MiB | 64.2 MiB | **~64 MiB** |
| Active CPU | 9.9% | 46.78% | 45.16% | **highly variable (see note)** |

Binary SHA-256: `ECD51861B54FE3F0CEE8A0D53BBFA42BCC2D72E9E7C2B7B20549E3971A0A7431`

## Notes on trustworthiness

- **Startup and RAM are stable and trustworthy** (tight ranges across 3 runs). Startup values are rounded to ~10 ms: the script polls window visibility every 50 ms, so detection granularity is ±50 ms and the raw stopwatch tenths overstate precision. The reported ~190–250 ms range is the meaningful figure.
- **Idle CPU is consistently near zero** (< 0.3%) — expected for an event-driven egui app.
- **Active CPU is noisy** (9.9%–46.8%). The synthetic random-cursor method drives egui repaint work that depends heavily on where the cursor lands (hover targets, canvas vs chrome) and on OS scheduling. Treat this as an upper-bound indicator of interactive load, **not** a precise or repeatable figure. A deterministic workload would be needed for a stable active-CPU number; that is out of scope for this baseline.
- Earlier planning-doc claims of "~10 MB exe" and "~49 MB private RAM" are **not** what this build measures: actual is ~14.17 MiB and ~61 MiB private / ~120 MiB working set. (Docs sync is WS-D scope; recorded here as a finding.)

## Re-measurement

Re-run `docs/performance/baseline.ps1` at each checkpoint and append results here so deltas are comparable. Keep the same environment and idle/active durations.
