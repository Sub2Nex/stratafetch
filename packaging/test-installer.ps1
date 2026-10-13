$ErrorActionPreference = 'Stop'
$workspacePath = Split-Path -Parent $PSScriptRoot
$installPath = [IO.Path]::GetFullPath((Join-Path $workspacePath ".smoke\install-$([DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds())"))
$allowedPath = [IO.Path]::GetFullPath((Join-Path $workspacePath '.smoke')) + [IO.Path]::DirectorySeparatorChar
if (-not $installPath.StartsWith($allowedPath,[StringComparison]::OrdinalIgnoreCase)) {throw 'Installer test path escaped workspace'}
$registryPath = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\StrataFetch'
if (Test-Path -LiteralPath $registryPath) {throw 'Existing installation registration must be preserved; isolated test stopped.'}
if (Test-Path -LiteralPath (Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\StrataFetch')) {throw 'Existing Start Menu folder must be preserved; isolated test stopped.'}
$process = Start-Process -FilePath (Join-Path $workspacePath 'dist\stratafetch-windows-x64-setup.exe') -ArgumentList @('/S',"/D=$installPath") -WindowStyle Hidden -Wait -PassThru
if ($process.ExitCode -ne 0) {throw "Installer failed: $($process.ExitCode)"}
foreach ($name in @('stratafetch.exe','stratafetch-desktop.exe','stratafetch-browser-host.exe')) {
    $actual = (Get-FileHash -LiteralPath (Join-Path $installPath $name) -Algorithm SHA256).Hash
    $expected = (Get-FileHash -LiteralPath (Join-Path $workspacePath "target\release\$name") -Algorithm SHA256).Hash
    if ($actual -ne $expected) {throw "Installed executable differs: $name"}
}
'preserve this unrelated file' | Set-Content -LiteralPath (Join-Path $installPath 'user-file.txt')
$process = Start-Process -FilePath (Join-Path $installPath 'Uninstall.exe') -ArgumentList '/S' -WindowStyle Hidden -Wait -PassThru
if ($process.ExitCode -ne 0) {throw "Uninstaller failed: $($process.ExitCode)"}
for ($attempt=0; $attempt -lt 100 -and (Test-Path -LiteralPath $registryPath); $attempt++) {Start-Sleep -Milliseconds 100}
foreach ($name in @('stratafetch.exe','stratafetch-desktop.exe','stratafetch-browser-host.exe')) {
    if (Test-Path -LiteralPath (Join-Path $installPath $name)) {throw "Installed executable left behind: $name"}
}
if (Test-Path -LiteralPath $registryPath) {throw 'Application registration left behind'}
if (-not (Test-Path -LiteralPath (Join-Path $installPath 'user-file.txt'))) {throw 'Uninstaller removed unrelated user data'}
if (Test-Path -LiteralPath (Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\StrataFetch')) {throw 'Start Menu shortcuts left behind'}
Write-Output 'Installer/uninstaller roundtrip passed; all three binaries match, registration and shortcuts removed, unrelated file preserved.'
