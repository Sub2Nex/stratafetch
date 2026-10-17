param(
    [Parameter(Mandatory=$true)][string]$InstallDirectory,
    [Parameter(Mandatory=$true)][string]$Version,
    [string]$TestId
)
$ErrorActionPreference = 'Stop'
$product = if ($TestId) { 'StrataFetch-InstallerTest-'+$TestId } else { 'StrataFetch' }
$iconPath = Join-Path $InstallDirectory 'stratafetch.ico'
if (-not (Test-Path -LiteralPath $iconPath)) { throw 'Installed shell icon is missing' }
if ((Get-FileHash -LiteralPath $iconPath).Hash -ne (Get-FileHash -LiteralPath (Join-Path $PSScriptRoot '../assets/stratafetch.ico')).Hash) { throw 'Installed shell icon differs from product artwork' }
$baseKey = [Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::CurrentUser, [Microsoft.Win32.RegistryView]::Registry64)
$key = $baseKey.OpenSubKey('Software\Microsoft\Windows\CurrentVersion\Uninstall\'+$product)
try {
    if ($null -eq $key -or $key.GetValue('DisplayVersion') -ne $Version) { throw 'Installed apps version is stale' }
    if ($key.GetValue('DisplayIcon') -ne ('"'+$iconPath+'",0')) { throw 'Installed apps icon reference is missing or incorrectly quoted' }
} finally {
    if ($key) { $key.Dispose() }
    $baseKey.Dispose()
}
$shell = New-Object -ComObject WScript.Shell
$startMenu = Join-Path ([Environment]::GetFolderPath('Programs')) $product
foreach ($name in @('StrataFetch','Uninstall')) {
    $shortcutPath = Join-Path $startMenu ($name+'.lnk')
    if (-not (Test-Path -LiteralPath $shortcutPath)) { throw ('Missing shortcut: '+$shortcutPath) }
    $shortcut = $shell.CreateShortcut($shortcutPath)
    if ($shortcut.IconLocation -ne ($iconPath+',0')) { throw ('Shortcut has no explicit product icon: '+$name) }
    $targetName = if ($name -eq 'StrataFetch') { 'stratafetch-desktop.exe' } else { 'Uninstall.exe' }
    if ($shortcut.TargetPath -ne (Join-Path $InstallDirectory $targetName)) { throw ('Shortcut target differs: '+$name) }
}
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class StrataFetchShellIconCheck {
    [DllImport("shell32.dll", CharSet=CharSet.Unicode)]
    public static extern uint ExtractIconExW(string file, int index, out IntPtr large, out IntPtr small, uint count);
    [DllImport("user32.dll")]
    public static extern bool DestroyIcon(IntPtr icon);
}
'@
$large = [IntPtr]::Zero
$small = [IntPtr]::Zero
try {
    $count = [StrataFetchShellIconCheck]::ExtractIconExW($iconPath, 0, [ref]$large, [ref]$small, 1)
    if ($count -ne 1 -or $large -eq [IntPtr]::Zero -or $small -eq [IntPtr]::Zero) { throw 'Windows Shell could not extract the registered icon' }
} finally {
    if ($large -ne [IntPtr]::Zero) { [void][StrataFetchShellIconCheck]::DestroyIcon($large) }
    if ($small -ne [IntPtr]::Zero) { [void][StrataFetchShellIconCheck]::DestroyIcon($small) }
}
[pscustomobject]@{Version=$Version; Icon=$iconPath; InstalledApps=$true; StartMenu=$true; WindowsIconExtraction=$true} | ConvertTo-Json
