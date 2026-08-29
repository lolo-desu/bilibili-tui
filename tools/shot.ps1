# Launch bilibili-tui, send optional keys, capture window (or screen) to PNG, kill app.
# Usage: powershell -ExecutionPolicy Bypass -File tools/shot.ps1 -Out shot.png -Keys "{ENTER}c" -LoadDelay 5 -KeyDelay 3
param(
    [string]$Out = "shot.png",
    [string]$Keys = "",
    [int]$LoadDelay = 5,
    [int]$KeyDelay = 3,
    [string]$Exe = "C:\Users\copy\.cargo\bin\bilibili-tui.exe",
    [string]$TitleHint = "bilibilitui",
    [string]$Open = ""
)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class Win {
    public delegate bool EnumProc(IntPtr h, IntPtr l);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
    [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("kernel32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool attach);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint msg, UIntPtr w, IntPtr l);
    [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr h, EnumProc cb, IntPtr l);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
    public struct RECT { public int L; public int T; public int R; public int B; }
}
"@

[Win]::SetProcessDPIAware() | Out-Null

function Find-WindowByTitle([string]$hint) {
    $found = [IntPtr]::Zero
    $cb = [Win+EnumProc]{
        param($h, $l)
        if (-not [Win]::IsWindowVisible($h)) { return $true }
        $sb = New-Object System.Text.StringBuilder 512
        [Win]::GetWindowText($h, $sb, 512) | Out-Null
        if ($sb.ToString().ToLower().Contains($hint.ToLower())) {
            $script:found = $h
            return $false
        }
        return $true
    }
    [Win]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null
    return $script:found
}

taskkill /IM mpv.exe /F 2>$null | Out-Null
taskkill /IM bilibili-tui.exe /F 2>$null | Out-Null
Start-Sleep -Milliseconds 800

# Snapshot existing top-level windows
$script:before = @{}
$cb0 = [Win+EnumProc]{
    param($h, $l)
    if ([Win]::IsWindowVisible($h)) { $script:before[$h] = $true }
    return $true
}
[Win]::EnumWindows($cb0, [IntPtr]::Zero) | Out-Null

# Escape quotes for Start-Process argument passing
if ($Open -ne "") {
  # Quote the spec so wt/cmd keep it as one argv entry (commas break %~2).
  start-process wt.exe -ArgumentList @('new-tab', '--title', 'bilibilitui', '-p', 'Windows PowerShell', 'cmd', '/c', "`"$Exe`" --open `"$Open`"")
} else {
  tools/launch_tui.cmd "$Exe" "$Open"
}
Start-Sleep -Milliseconds 2500
Start-Sleep -Milliseconds ($LoadDelay * 1000)

# Find NEW visible window (not in before-set), prefer one whose title matches
$hwnd = [IntPtr]::Zero
$script:candidates = New-Object System.Collections.ArrayList
$cb1 = [Win+EnumProc]{
    param($h, $l)
    if ([Win]::IsWindowVisible($h) -and -not $script:before.ContainsKey($h)) {
        $sb = New-Object System.Text.StringBuilder 512
        [Win]::GetWindowText($h, $sb, 512) | Out-Null
        $title = $sb.ToString()
        [void]$script:candidates.Add([pscustomobject]@{ H = $h; T = $title })
    }
    return $true
}
[Win]::EnumWindows($cb1, [IntPtr]::Zero) | Out-Null
$match = $script:candidates | Where-Object { $_.T -match $TitleHint } | Select-Object -First 1
if ($match) { $hwnd = $match.H }
elseif ($script:candidates.Count -gt 0) { $hwnd = $script:candidates[0].H }
Write-Output ("candidates: " + (($script:candidates | ForEach-Object { "[" + $_.T + "]" }) -join " "))

# Resolve the actual terminal control child window (CASCADIA class) for key delivery
if ($hwnd -ne [IntPtr]::Zero) {
    $script:termChild = [IntPtr]::Zero
    $cbc = [Win+EnumProc]{
        param($h, $l)
        $cn = New-Object System.Text.StringBuilder 256
        [Win]::GetClassName($h, $cn, 256) | Out-Null
        $cls = $cn.ToString()
        Write-Output ("child: " + $cls)
        if ($cls -match "CASCADIA|TermControl|TermBridge") { $script:termChild = $h }
        return $true
    }
    [Win]::EnumChildWindows($hwnd, $cbc, [IntPtr]::Zero) | Out-Null
    if ($script:termChild -ne [IntPtr]::Zero) { $keyTarget = $script:termChild } else { $keyTarget = $hwnd }
} else { $keyTarget = [IntPtr]::Zero }
Write-Output "keyTarget=$keyTarget"

$gotRect = $false
$rect = New-Object Win+RECT
if ($hwnd -ne [IntPtr]::Zero) {
    # bring to front reliably: alt-trick + restore + attach-thread-input
    [Win]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero)
    [Win]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 150
    [Win]::ShowWindow($hwnd, 9) | Out-Null
    $fg = [Win]::GetForegroundWindow()
    if ($fg -ne $hwnd) {
        $fgThread = [Win]::GetWindowThreadProcessId($fg, [ref]0)
        $myThread = [System.AppDomain]::GetCurrentThreadId()
        [Win]::AttachThreadInput($myThread, $fgThread, $true) | Out-Null
        [Win]::SetForegroundWindow($hwnd) | Out-Null
        [Win]::AttachThreadInput($myThread, $fgThread, $false) | Out-Null
    }
    Start-Sleep -Milliseconds 700
    if ([Win]::GetWindowRect($hwnd, [ref]$rect)) {
        $w = $rect.R - $rect.L; $h = $rect.B - $rect.T
        if ($w -gt 100 -and $h -gt 100) { $gotRect = $true }
    }
}

if ($Keys -ne "") {
    # Alt-key trick unlocks SetForegroundWindow restrictions
    [Win]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero)
    [Win]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 200
    [Win]::ShowWindow($hwnd, 9) | Out-Null  # SW_RESTORE
    $fg = [Win]::GetForegroundWindow()
    if ($fg -ne $hwnd) {
        $fgThread = [Win]::GetWindowThreadProcessId($fg, [ref]0)
        $myThread = [System.AppDomain]::GetCurrentThreadId()
        [Win]::AttachThreadInput($myThread, $fgThread, $true) | Out-Null
        [Win]::SetForegroundWindow($hwnd) | Out-Null
        [Win]::AttachThreadInput($myThread, $fgThread, $false) | Out-Null
    }
    Start-Sleep -Milliseconds 800
    $fgOk = ([Win]::GetForegroundWindow() -eq $hwnd)
    Write-Output "foreground_ok=$fgOk"
    $keyEvents = @{ 'enter' = 0x0D; 'space' = 0x20; 'down' = 0x28; 'up' = 0x26; 'left' = 0x25; 'right' = 0x27; 'tab' = 0x09; 'esc' = 0x1B; 'pgdn' = 0x22; 'pgup' = 0x21 }
    foreach ($tok in $Keys -split ',') {
        $t = $tok.Trim().ToLower()
        if ($t -eq "") { continue }
        if ($keyEvents.ContainsKey($t)) { $vk = [byte]$keyEvents[$t] }
        else { $vk = [byte][char]($t.ToUpper()[0]) }
        $scan = 0
        [void][System.Runtime.InteropServices.Marshal]::TryParse($vk.ToString(), [ref]$scan)
        $kd = [UIntPtr][uint32]1
        $ku = [UIntPtr][uint32]0xC0000001
        [Win]::PostMessage($keyTarget, 0x100, [UIntPtr]$vk, [IntPtr][int]1) | Out-Null
        Start-Sleep -Milliseconds 60
        [Win]::PostMessage($keyTarget, 0x101, [UIntPtr]$vk, [IntPtr][int]0xC0000001) | Out-Null
        Start-Sleep -Milliseconds 300
    }
    Start-Sleep -Milliseconds ($KeyDelay * 1000)
}

if (-not $gotRect) {
    $vs = [System.Windows.Forms.SystemInformation]::VirtualScreen
    $rect.L = $vs.X; $rect.T = $vs.Y; $rect.R = $vs.X + $vs.Width; $rect.B = $vs.Y + $vs.Height
}
$w = $rect.R - $rect.L; $h = $rect.B - $rect.T
$bmp = New-Object System.Drawing.Bitmap($w, $h)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($rect.L, $rect.T, 0, 0, $bmp.Size)
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()

taskkill /IM bilibili-tui.exe /F 2>$null | Out-Null
if ($hwnd -ne [IntPtr]::Zero) {
    [Win]::PostMessage($hwnd, 0x10, [UIntPtr]::Zero, [IntPtr]::Zero) | Out-Null  # WM_CLOSE
}
Write-Output "saved $Out ${w}x${h} hwnd=$hwnd gotRect=$gotRect"
