# ZoneFlow — three-workload performance benchmark (Windows).
#
# Run from the repository root:
#   powershell -ExecutionPolicy Bypass -File docs\performance\benchmark.ps1 -Mode Background -Exe <path>
#
# Modes:
#   Background    - Rust backend only, browser closed. Runnable now with the
#                   headless spike exe. This is the always-on workload.
#   EditorOpen    - Rust backend + browser with the editor loaded. Requires
#                   -Url (the tokenised editor URL) and a running frontend.
#   ActiveEditing - EditorOpen plus 20s of synthetic pointer movement over the
#                   SVG canvas. Requires -Url and a running frontend.
#
# All modes sample WorkingSet64 / PrivateMemorySize64 / TotalProcessorTime over
# a fixed idle window and report raw MiB + CPU%. Browser processes are summed
# separately from the Rust process so the two costs are never conflated.

param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("Background", "EditorOpen", "ActiveEditing")]
    [string]$Mode,
    [string]$Exe = "target\release\screenlayout.exe",
    [string]$Url = "",
    [string]$BrowserProcessName = "msedge",
    [int]$IdleSeconds = 30,
    [int]$ActiveSeconds = 20
)

$ErrorActionPreference = "Stop"

if ($Mode -ne "Background" -and [string]::IsNullOrWhiteSpace($Url)) {
    throw "-Url is required for $Mode (the tokenised editor URL)."
}

$exeFull = (Resolve-Path $Exe).Path
$proc = Start-Process -FilePath $exeFull -PassThru
Start-Sleep -Seconds 5

function Sample([System.Diagnostics.Process]$p) {
    $p.Refresh()
    return [pscustomobject]@{
        WS   = $p.WorkingSet64
        Priv = $p.PrivateMemorySize64
        CPU  = $p.TotalProcessorTime.TotalSeconds
    }
}

function SampleBrowser([string]$name) {
    $procs = Get-Process -Name $name -ErrorAction SilentlyContinue
    if (-not $procs) { return [pscustomobject]@{ WS = 0; Priv = 0; CPU = 0; Count = 0 } }
    return [pscustomobject]@{
        WS    = ($procs | Measure-Object -Property WorkingSet64 -Sum).Sum
        Priv  = ($procs | Measure-Object -Property PrivateMemorySize64 -Sum).Sum
        CPU   = ($procs | Measure-Object -Property TotalProcessorTime -Sum).Sum.TotalSeconds
        Count = $procs.Count
    }
}

$rustStart = Sample $proc
$browserStart = SampleBrowser $BrowserProcessName

if ($Mode -ne "Background") {
    Start-Process $Url
    Start-Sleep -Seconds 5
}

Start-Sleep -Seconds $IdleSeconds

$rustEnd = Sample $proc
$browserEnd = SampleBrowser $BrowserProcessName

$out = [ordered]@{
    Mode                 = $Mode
    RustWorkingSetMiB    = [math]::Round($rustEnd.WS / 1MB, 1)
    RustPrivateMiB       = [math]::Round($rustEnd.Priv / 1MB, 1)
    RustCpuPercent       = [math]::Round((($rustEnd.CPU - $rustStart.CPU) / $IdleSeconds) * 100, 2)
    BrowserProcessCount  = $browserEnd.Count
    BrowserWorkingSetMiB = [math]::Round($browserEnd.WS / 1MB, 1)
    BrowserPrivateMiB    = [math]::Round($browserEnd.Priv / 1MB, 1)
    ExeBytes             = (Get-Item $exeFull).Length
    ExeMiB               = [math]::Round((Get-Item $exeFull).Length / 1MB, 2)
}

if ($Mode -eq "ActiveEditing") {
    # Deterministic pointer movement over the canvas region. Replace the rect
    # with the real SVG canvas bounds once the frontend exists.
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
    $rng = New-Object System.Random
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    while ($sw.Elapsed.TotalSeconds -lt $ActiveSeconds) {
        [void][ZfBench]::SetCursorPos($rng.Next(200, 1400), $rng.Next(200, 800))
        Start-Sleep -Milliseconds 40
    }
    $sw.Stop()
    $out.ActiveSampleSeconds = [math]::Round($sw.Elapsed.TotalSeconds, 1)
    [void][ZfBench]::SetCursorPos($orig.X, $orig.Y)
}

try { $proc.CloseMainWindow() | Out-Null } catch {}
Start-Sleep -Milliseconds 500
if (-not $proc.HasExited) { Stop-Process -Id $proc.Id -Force }

$out | ConvertTo-Json -Depth 3
