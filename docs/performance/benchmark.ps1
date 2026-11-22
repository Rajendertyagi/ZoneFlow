# ZoneFlow — three-workload performance benchmark (Windows).
#
# Run from the repository root:
#   powershell -ExecutionPolicy Bypass -File docs\performance\benchmark.ps1 -Mode Background -Exe <path>
#
# Modes:
#   Background      - Rust backend only, browser closed. Run with the final
#                     release exe. This is the always-on workload.
#   EditorOpen      - Rust backend + browser with the editor loaded. Requires
#                     -Url (the editor URL, e.g. http://127.0.0.1:4000/) and a
#                     running frontend.
#   PointerActivity - EditorOpen plus 20s of synthetic cursor movement. This is
#                     NOT an editing workload: it does not drag or resize a zone.
#                     Requires -Url and a running frontend.
#
# Real SVG zone-boundary dragging is verified separately as a functional test
# (dev-only browser automation, never an installed runtime dependency). See
# docs/performance/results.md. Do not describe PointerActivity as editing.
#
# All modes sample WorkingSet64 / PrivateMemorySize64 / TotalProcessorTime over
# a fixed window and report raw MiB + CPU%. Browser processes are attributed
# only to this run and reported separately from the Rust process.
#
# IMPORTANT: close all browser windows before running EditorOpen/PointerActivity.
# The script snapshots existing browser PIDs and measures only NEW processes;
# if a browser is already running, opening a URL may reuse it and attribution
# will be incomplete. The script warns when this happens.

param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("Background", "EditorOpen", "PointerActivity")]
    [string]$Mode,
    [string]$Exe = "target\release\screenlayout.exe",
    [string]$Url = "",
    [string]$BrowserProcessName = "msedge",
    [int]$IdleSeconds = 30,
    [int]$ActiveSeconds = 20
)

$ErrorActionPreference = "Stop"

if ($Mode -ne "Background" -and [string]::IsNullOrWhiteSpace($Url)) {
    throw "-Url is required for $Mode (the editor URL, e.g. http://127.0.0.1:4000/)."
}

$exeFull = (Resolve-Path $Exe).Path

function Sample([System.Diagnostics.Process]$p) {
    $p.Refresh()
    return [pscustomobject]@{
        WS   = $p.WorkingSet64
        Priv = $p.PrivateMemorySize64
        CPU  = $p.TotalProcessorTime.TotalSeconds
    }
}

# Snapshot browser PIDs that exist BEFORE we launch, so we only attribute
# processes created by this run.
$preExistingBrowserPids = @(Get-Process -Name $BrowserProcessName -ErrorAction SilentlyContinue | ForEach-Object { $_.Id })

function SampleBrowser([string]$name, [int[]]$excludePids) {
    $procs = @(Get-Process -Name $name -ErrorAction SilentlyContinue | Where-Object { $excludePids -notcontains $_.Id })
    if ($procs.Count -eq 0) { return [pscustomobject]@{ WS = 0; Priv = 0; CPU = 0; Count = 0 } }
    return [pscustomobject]@{
        WS    = ($procs | Measure-Object -Property WorkingSet64 -Sum).Sum
        Priv  = ($procs | Measure-Object -Property PrivateMemorySize64 -Sum).Sum
        CPU   = ($procs | Measure-Object -Property TotalProcessorTime -Sum).Sum.TotalSeconds
        Count = $procs.Count
    }
}

$proc = $null
$orig = $null
try {
    $proc = Start-Process -FilePath $exeFull -PassThru
    Start-Sleep -Seconds 5

    if ($Mode -ne "Background") {
        Start-Process $Url
        Start-Sleep -Seconds 5
        $newBrowser = SampleBrowser $BrowserProcessName $preExistingBrowserPids
        if ($newBrowser.Count -eq 0) {
            Write-Warning "No new $BrowserProcessName processes detected - the browser may have reused an existing instance. Close all browser windows and re-run for clean attribution."
        }
    }

    # Baselines are taken AFTER the browser has loaded and settled, immediately
    # before the idle window. CPU deltas are divided only by $IdleSeconds, so
    # sampling earlier would include launch/settle CPU in the reported rate.
    $rustStart = Sample $proc
    $browserStart = SampleBrowser $BrowserProcessName $preExistingBrowserPids

    # ---- Idle / editor-open window ----
    Start-Sleep -Seconds $IdleSeconds

    $rustIdleEnd = Sample $proc
    $browserIdleEnd = SampleBrowser $BrowserProcessName $preExistingBrowserPids

    $out = [ordered]@{
        Mode                 = $Mode
        RustWorkingSetMiB    = [math]::Round($rustIdleEnd.WS / 1MB, 1)
        RustPrivateMiB       = [math]::Round($rustIdleEnd.Priv / 1MB, 1)
        RustCpuPercent       = [math]::Round((($rustIdleEnd.CPU - $rustStart.CPU) / $IdleSeconds) * 100, 2)
        BrowserProcessCount  = $browserIdleEnd.Count
        BrowserWorkingSetMiB = [math]::Round($browserIdleEnd.WS / 1MB, 1)
        BrowserPrivateMiB    = [math]::Round($browserIdleEnd.Priv / 1MB, 1)
        BrowserCpuPercent    = [math]::Round((($browserIdleEnd.CPU - $browserStart.CPU) / $IdleSeconds) * 100, 2)
        ExeBytes             = (Get-Item $exeFull).Length
        ExeMiB               = [math]::Round((Get-Item $exeFull).Length / 1MB, 2)
    }

    # ---- Pointer activity: sample AFTER the movement, not before ----
    if ($Mode -eq "PointerActivity") {
        Add-Type @"
using System;
using System.Runtime.InteropServices;
public class ZfBench {
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
    [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
}
"@
        $orig = New-Object ZfBench+POINT
        [void][ZfBench]::GetCursorPos([ref]$orig)

        $rustActiveStart = Sample $proc
        $browserActiveStart = SampleBrowser $BrowserProcessName $preExistingBrowserPids

        $rng = New-Object System.Random
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        while ($sw.Elapsed.TotalSeconds -lt $ActiveSeconds) {
            [void][ZfBench]::SetCursorPos($rng.Next(200, 1400), $rng.Next(200, 800))
            Start-Sleep -Milliseconds 40
        }
        $sw.Stop()

        $rustActiveEnd = Sample $proc
        $browserActiveEnd = SampleBrowser $BrowserProcessName $preExistingBrowserPids
        $activeWall = $sw.Elapsed.TotalSeconds

        $out.PointerRustWorkingSetMiB    = [math]::Round($rustActiveEnd.WS / 1MB, 1)
        $out.PointerRustPrivateMiB       = [math]::Round($rustActiveEnd.Priv / 1MB, 1)
        $out.PointerRustCpuPercent       = [math]::Round((($rustActiveEnd.CPU - $rustActiveStart.CPU) / $activeWall) * 100, 2)
        $out.PointerBrowserWorkingSetMiB = [math]::Round($browserActiveEnd.WS / 1MB, 1)
        $out.PointerBrowserPrivateMiB    = [math]::Round($browserActiveEnd.Priv / 1MB, 1)
        $out.PointerBrowserCpuPercent    = [math]::Round((($browserActiveEnd.CPU - $browserActiveStart.CPU) / $activeWall) * 100, 2)
        $out.PointerSampleSeconds        = [math]::Round($activeWall, 1)
    }

    $out | ConvertTo-Json -Depth 3
}
finally {
    # Cursor restoration and process cleanup run even if the benchmark fails.
    if ($null -ne $orig) {
        try { [void][ZfBench]::SetCursorPos($orig.X, $orig.Y) } catch {}
    }
    if ($null -ne $proc) {
        try { $proc.CloseMainWindow() | Out-Null } catch {}
        Start-Sleep -Milliseconds 500
        if (-not $proc.HasExited) { Stop-Process -Id $proc.Id -Force }
    }
}
