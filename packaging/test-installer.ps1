param(
    [string]$InstallerPath,
    [string]$PackagePath,
    [string]$RegistrationName = 'StrataFetch',
    [string]$ShortcutFolder = 'StrataFetch'
)
$ErrorActionPreference = 'Stop'
$workspacePath = Split-Path -Parent $PSScriptRoot
if (-not $InstallerPath) { $InstallerPath = Join-Path $workspacePath 'dist/stratafetch-windows-x64-setup.exe' }
if (-not $PackagePath) { $PackagePath = Join-Path $workspacePath 'dist/stratafetch-windows-x64' }
if ($RegistrationName -notmatch '^StrataFetch(?:-InstallerTest-[a-zA-Z0-9-]+)?$' -or $ShortcutFolder -ne $RegistrationName) { throw 'Unexpected installer test destinations' }
$installPath = [IO.Path]::GetFullPath((Join-Path $workspacePath ".smoke\install-$([DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds())"))
$allowedPath = [IO.Path]::GetFullPath((Join-Path $workspacePath '.smoke')) + [IO.Path]::DirectorySeparatorChar
if (-not $installPath.StartsWith($allowedPath,[StringComparison]::OrdinalIgnoreCase)) {throw 'Installer test path escaped workspace'}
$registryPath = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\' + $RegistrationName
$shortcutPath = Join-Path $env:APPDATA ('Microsoft\Windows\Start Menu\Programs\' + $ShortcutFolder)
if (Test-Path -LiteralPath $registryPath) {throw 'Existing installation registration must be preserved; isolated test stopped.'}
if (Test-Path -LiteralPath $shortcutPath) {throw 'Existing Start Menu folder must be preserved; isolated test stopped.'}
$process = Start-Process -FilePath $InstallerPath -ArgumentList @('/S',"/D=$installPath") -WindowStyle Hidden -Wait -PassThru
if ($process.ExitCode -ne 0) {throw "Installer failed: $($process.ExitCode)"}
foreach ($name in @('stratafetch.exe','stratafetch-desktop.exe','stratafetch-browser-host.exe')) {
    $actual = (Get-FileHash -LiteralPath (Join-Path $installPath $name) -Algorithm SHA256).Hash
    $expected = (Get-FileHash -LiteralPath (Join-Path $PackagePath $name) -Algorithm SHA256).Hash
    if ($actual -ne $expected) {throw "Installed executable differs: $name"}
}
$expectedVersionMatch = [regex]::Match((Get-Content -LiteralPath (Join-Path $workspacePath 'Cargo.toml') -Raw), '(?m)^version = "([^"]+)"\r?$')
if (-not $expectedVersionMatch.Success) { throw 'Cannot determine installer test version' }
$expectedVersion = $expectedVersionMatch.Groups[1].Value
$registration = Get-ItemProperty -LiteralPath $registryPath
if ($registration.DisplayVersion -ne $expectedVersion -or $registration.InstallLocation -ne $installPath) { throw 'Installed application metadata differs from package' }
foreach ($shortcut in @('StrataFetch.lnk', 'Uninstall.lnk')) { if (-not (Test-Path -LiteralPath (Join-Path $shortcutPath $shortcut))) { throw "Missing shortcut: $shortcut" } }
$installedVersion = & (Join-Path $installPath 'stratafetch.exe') --version
if ($LASTEXITCODE -ne 0 -or $installedVersion -ne "stratafetch $expectedVersion") { throw 'Installed CLI failed version check' }
'preserve this unrelated file' | Set-Content -LiteralPath (Join-Path $installPath 'user-file.txt')
# Reinstall in the isolated location; never let a test fall back to the user's default directory.
$process = Start-Process -FilePath $InstallerPath -ArgumentList @('/S',"/D=$installPath") -WindowStyle Hidden -Wait -PassThru
if ($process.ExitCode -ne 0) { throw "Installer upgrade failed: $($process.ExitCode)" }
if (-not (Test-Path -LiteralPath (Join-Path $installPath 'user-file.txt'))) { throw 'Upgrade removed unrelated user data' }
$process = Start-Process -FilePath (Join-Path $installPath 'Uninstall.exe') -ArgumentList '/S' -WindowStyle Hidden -Wait -PassThru
if ($process.ExitCode -ne 0) {throw "Uninstaller failed: $($process.ExitCode)"}
for ($attempt=0; $attempt -lt 100 -and (Test-Path -LiteralPath $registryPath); $attempt++) {Start-Sleep -Milliseconds 100}
foreach ($name in @('stratafetch.exe','stratafetch-desktop.exe','stratafetch-browser-host.exe')) {
    if (Test-Path -LiteralPath (Join-Path $installPath $name)) {throw "Installed executable left behind: $name"}
}
if (Test-Path -LiteralPath $registryPath) {throw 'Application registration left behind'}
if (-not (Test-Path -LiteralPath (Join-Path $installPath 'user-file.txt'))) {throw 'Uninstaller removed unrelated user data'}
if (Test-Path -LiteralPath $shortcutPath) {throw 'Start Menu shortcuts left behind'}
Write-Output 'Installer/uninstaller roundtrip passed; all three binaries match, registration and shortcuts removed, unrelated file preserved.'
