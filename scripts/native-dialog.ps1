# UIA locates the disposable process's dialog; Win32 drives its classic controls.
param(
    [Parameter(Mandatory)][string]$RunRoot,
    [Parameter(Mandatory)][string]$Repository,
    [Parameter(Mandatory)][int]$ApplicationId,
    [Parameter(Mandatory)][ValidateSet('Export saved Ascension records', 'Preview an Ascension export', 'Export an Ascension recovery copy')][string]$Title,
    [ValidateSet('Inspect', 'Select', 'Cancel')][string]$Action = 'Inspect',
    [string]$SelectedPath
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class NativeDialogControls {
    public delegate bool Visitor(IntPtr hwnd, IntPtr data);
    [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr parent, Visitor visitor, IntPtr data);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr hwnd, StringBuilder value, int length);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr hwnd, StringBuilder value, int length);
    [DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool IsWindowEnabled(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool IsChild(IntPtr parent, IntPtr child);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint process);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr SendMessageTimeout(IntPtr hwnd, uint message, IntPtr wparam, string value, uint flags, uint timeout, out IntPtr result);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hwnd, uint message, IntPtr wparam, IntPtr lparam);
}
'@
$root = [IO.Path]::GetFullPath($RunRoot)
$artifacts = [IO.Path]::GetFullPath((Join-Path $Repository '.artifacts')) + '\'
if (!$root.StartsWith($artifacts, [StringComparison]::OrdinalIgnoreCase)) { throw 'Run directory must be inside repository artifacts.' }
$configuration = Get-Content -LiteralPath (Join-Path $root 'tauri.conf.json') -Raw | ConvertFrom-Json
if ($configuration.identifier -notmatch '^io\.ascension\.verification\.r[0-9a-f]{32}$' -or
    ![string]::Equals($configuration.app.appDirectoriesOverride, (Join-Path $root 'profile'), [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Expected an isolated verification configuration.'
}
if ($SelectedPath) {
    $SelectedPath = [IO.Path]::GetFullPath($SelectedPath)
    if (!$SelectedPath.StartsWith($root + '\', [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Selected files must stay inside this disposable run.'
    }
}
$expectedExe = Join-Path $root 'installed\ascension.exe'
$process = Get-Process -Id $ApplicationId -ErrorAction Stop
if (![string]::Equals($process.Path, $expectedExe, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Refusing to automate a process outside the disposable installation.'
}
$condition = [System.Windows.Automation.AndCondition]::new(
    [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ProcessIdProperty, $ApplicationId),
    [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::NameProperty, $Title)
)
$deadline = [DateTime]::UtcNow.AddSeconds(15)
$dialog = $null
while (!$dialog -and [DateTime]::UtcNow -lt $deadline) {
    $windows = [System.Windows.Automation.AutomationElement]::RootElement.FindAll([System.Windows.Automation.TreeScope]::Children, $condition)
    if ($windows.Count -gt 1) { throw 'Ambiguous native dialog.' }
    if ($windows.Count -eq 1) { $dialog = $windows[0] } else { Start-Sleep -Milliseconds 100 }
}
if (!$dialog) { throw 'Expected native dialog did not appear.' }
if ($dialog.Current.ClassName -ne '#32770') { throw 'Expected a Windows common dialog.' }
$nativeControls = [Collections.Generic.List[object]]::new()
$visitor = [NativeDialogControls+Visitor]{ param($handle, $unused)
    $class = [Text.StringBuilder]::new(256)
    [void][NativeDialogControls]::GetClassName($handle, $class, $class.Capacity)
    $label = [Text.StringBuilder]::new(256)
    if ($class.ToString() -eq 'Button') { [void][NativeDialogControls]::GetWindowText($handle, $label, $label.Capacity) }
    $nativeControls.Add([PSCustomObject]@{ handle=$handle; class=$class.ToString(); id=[NativeDialogControls]::GetDlgCtrlID($handle); label=$label.ToString(); visible=[NativeDialogControls]::IsWindowVisible($handle); enabled=[NativeDialogControls]::IsWindowEnabled($handle) })
    return $true
}
[void][NativeDialogControls]::EnumChildWindows([IntPtr]$dialog.Current.NativeWindowHandle, $visitor, [IntPtr]::Zero)
if ($Action -eq 'Inspect') {
    $nativeControls | ConvertTo-Json -Depth 3
    [PSCustomObject]@{ dialog = $dialog.Current.Name; class = $dialog.Current.ClassName; handle = $dialog.Current.NativeWindowHandle } | ConvertTo-Json
    exit
}
$buttonId = if ($Action -eq 'Cancel') { 2 } else { 1 }
function Assert-Control([IntPtr]$Handle) {
    [uint32]$owner = 0
    [void][NativeDialogControls]::GetWindowThreadProcessId($Handle, [ref]$owner)
    if ($owner -ne $ApplicationId -or ![NativeDialogControls]::IsChild([IntPtr]$dialog.Current.NativeWindowHandle, $Handle)) {
        throw 'Native control no longer belongs to the isolated dialog.'
    }
}
$button = @($nativeControls | Where-Object { $_.class -eq 'Button' -and $_.id -eq $buttonId -and $_.visible -and $_.enabled })
if ($button.Count -ne 1) { throw 'Expected one enabled native dialog button.' }
if ($Action -eq 'Select') {
    if (!$SelectedPath) { throw 'A disposable file path is required.' }
    # Observed common-dialog IDs: Save uses 1001, Open uses edt1 (1148).
    $editId = if ($Title -in @('Export saved Ascension records', 'Export an Ascension recovery copy')) { 1001 } else { 1148 }
    $edit = @($nativeControls | Where-Object { $_.class -eq 'Edit' -and $_.id -eq $editId -and $_.visible -and $_.enabled })
    if ($edit.Count -ne 1) {
        $nativeControls | Where-Object { $_.class -in @('Edit', 'Button') } | ConvertTo-Json -Depth 3 | Write-Output
        throw 'Expected one native file-name edit control.'
    }
    Assert-Control $edit[0].handle
    # WM_SETTEXT fills the actual dialog; the real Save/Open button confirms it.
    $result = [IntPtr]::Zero
    if ([NativeDialogControls]::SendMessageTimeout($edit[0].handle, 0x000C, [IntPtr]::Zero, $SelectedPath, 2, 2000, [ref]$result) -eq [IntPtr]::Zero -or $result -eq [IntPtr]::Zero) {
        throw 'The native file-name control rejected its value.'
    }
}
Assert-Control $button[0].handle
if (![NativeDialogControls]::PostMessage($button[0].handle, 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero)) {
    throw 'Could not activate the native dialog button.'
}
[PSCustomObject]@{ dialog=$Title; action=$Action; button=$button[0].label; process=$ApplicationId } | ConvertTo-Json -Compress
