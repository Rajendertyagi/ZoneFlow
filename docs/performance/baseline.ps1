# ZoneFlow performance baseline measurement (Windows).
# Run from the repository root: powershell -ExecutionPolicy Bypass -File docs\performance\baseline.ps1
# Launches the release build, measures cold startup to a visible main window,
# then samples idle and active RAM/CPU. Restores the cursor and kills the app.

param(
    [string]$Exe = "target\release\screenlayout.exe",
    [int]$IdleSeconds = 30,
    [int]$ActiveSeconds = 20
)

$ErrorActionPreference = "Stop"

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class ZfWin {
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT r);
    [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
}
"@

$exeFull = (Resolve-Path $Exe).Path
$out = [ordered]@{}

# ---- Cold startup: process start -> main window visible ----
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$proc = Start-Process -FilePath $exeFull -PassThru
$deadline = (Get-Date).AddSeconds(30)
$ready = $false
while ((Get-Date) -lt $deadline) {
    Start-Sleep -Milliseconds 50
    $proc.Refresh()
    if ($proc.MainWindowHandle -ne [IntPtr]::Zero -and
        [ZfWin]::IsWindowVisible($proc.MainWindowHandle)) {
        $ready = $true
        break
    }
}
$sw.Stop()
$out.ColdStartupReady = $ready
$out.ColdStartupMs = if ($ready) { [math]::Round($sw.Elapsed.TotalMilliseconds, 1) } else { -1 }

# Let the UI settle before sampling.
Start-Sleep -Seconds 5

function Sample([System.Diagnostics.Process]$p) {
    $p.Refresh()
    return [pscustomobject]@{
        WS   = $p.WorkingSet64
        Priv = $p.PrivateMemorySize64
        CPU  = $p.TotalProcessorTime.TotalSeconds
    }
}

# ---- Idle: 30s window, no input ----
$idleStart = Sample $proc
Start-Sleep -Seconds $IdleSeconds
$idleEnd = Sample $proc
$idleWall = $IdleSeconds
$out.IdleWorkingSetMiB = [math]::Round($idleEnd.WS / 1MB, 1)
$out.IdlePrivateMiB = [math]::Round($idleEnd.Priv / 1MB, 1)
$out.IdleCpuPercent = [math]::Round((($idleEnd.CPU - $idleStart.CPU) / $idleWall) * 100, 2)

# ---- Active: drive mouse movement over the canvas for $ActiveSeconds ----
$rect = New-Object ZfWin+RECT
[void][ZfWin]::GetWindowRect($proc.MainWindowHandle, [ref]$rect)
$orig = New-Object ZfWin+POINT
[void][ZfWin]::GetCursorPos([ref]$orig)

$activeStart = Sample $proc
$swActive = [System.Diagnostics.Stopwatch]::StartNew()
$rng = New-Object System.Random
while ($swActive.Elapsed.TotalSeconds -lt $ActiveSeconds) {
    $x = $rng.Next($rect.Left + 40, [math]::Max($rect.Left + 41, $rect.Right - 40))
    $y = $rng.Next($rect.Top + 120, [math]::Max($rect.Top + 121, $rect.Bottom - 80))
    [void][ZfWin]::SetCursorPos($x, $y)
    Start-Sleep -Milliseconds 40
}
$swActive.Stop()
$activeEnd = Sample $proc
$activeWall = $swActive.Elapsed.TotalSeconds
$out.ActiveWorkingSetMiB = [math]::Round($activeEnd.WS / 1MB, 1)
$out.ActivePrivateMiB = [math]::Round($activeEnd.Priv / 1MB, 1)
$out.ActiveCpuPercent = [math]::Round((($activeEnd.CPU - $activeStart.CPU) / $activeWall) * 100, 2)
$out.ActiveSampleSeconds = [math]::Round($activeWall, 1)

# Restore cursor and stop the app.
[void][ZfWin]::SetCursorPos($orig.X, $orig.Y)
try { $proc.CloseMainWindow() | Out-Null } catch {}
Start-Sleep -Milliseconds 500
if (-not $proc.HasExited) { Stop-Process -Id $proc.Id -Force }

$out | ConvertTo-Json -Depth 3
