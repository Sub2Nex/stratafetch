param([string]$BinaryPath,[ValidatePattern('^[a-p]{32}$')][string]$ChromiumExtensionId,[ValidatePattern('^[a-p]{32}$')][string]$EdgeExtensionId,[switch]$Uninstall,[ValidatePattern('^HKCU:\\Software\\StrataFetch-Test-[a-f0-9-]{36}$')][string]$TestRegistryRoot)
$ErrorActionPreference = 'Stop'
$statePath = Join-Path $env:LOCALAPPDATA 'StrataFetch\StrataFetch\data'
$registryPaths = @('HKCU:\Software\Google\Chrome\NativeMessagingHosts\org.stratafetch.host','HKCU:\Software\Microsoft\Edge\NativeMessagingHosts\org.stratafetch.host','HKCU:\Software\Mozilla\NativeMessagingHosts\org.stratafetch.host')
if ($TestRegistryRoot) {
    if (-not $env:STRATAFETCH_STATE_DIR) { throw 'Isolated registration test requires STRATAFETCH_STATE_DIR' }
    $statePath = $env:STRATAFETCH_STATE_DIR
    $registryPaths = @('Chrome','Edge','Firefox') | ForEach-Object { Join-Path $TestRegistryRoot $_ }
}
if ($Uninstall) {
    foreach ($registryPath in $registryPaths) {
        if (Test-Path -LiteralPath $registryPath) {
            $manifestPath = (Get-Item -LiteralPath $registryPath).GetValue('')
            if ($BinaryPath) {
                if (-not $manifestPath -or -not (Test-Path -LiteralPath $manifestPath)) { throw 'Cannot verify browser registration ownership. Registration was preserved.' }
                $registered = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
                if ($registered.path -ne $BinaryPath) { throw 'Another installation owns browser registration. Registration was preserved.' }
            }
        }
    }
    foreach ($registryPath in $registryPaths) { if (Test-Path -LiteralPath $registryPath) { Remove-Item -LiteralPath $registryPath } }
    foreach ($filename in @('browser-origins.json','chromium-host.json','firefox-host.json')) {
        $manifestPath = Join-Path $statePath $filename
        if (Test-Path -LiteralPath $manifestPath) { Remove-Item -LiteralPath $manifestPath }
    }
    Write-Output 'Browser registration removed. Download state preserved.'
    return
}
if (-not $BinaryPath -or -not $ChromiumExtensionId) { throw 'Registration requires -BinaryPath and -ChromiumExtensionId.' }
$resolvedBinary = (Resolve-Path -LiteralPath $BinaryPath).Path
if ([IO.Path]::GetFileName($resolvedBinary) -ne 'stratafetch-browser-host.exe') { throw 'Select stratafetch-browser-host.exe.' }
New-Item -ItemType Directory -Force -Path $statePath | Out-Null
$origin = "chrome-extension://$ChromiumExtensionId/"
$origins = @($origin)
if ($EdgeExtensionId -and $EdgeExtensionId -ne $ChromiumExtensionId) { $origins += "chrome-extension://$EdgeExtensionId/" }
$utf8 = New-Object System.Text.UTF8Encoding($false)
[IO.File]::WriteAllText((Join-Path $statePath 'browser-origins.json'),(ConvertTo-Json -InputObject @($origins + @('stratafetch@local'))),$utf8)
$chromiumManifest = Join-Path $statePath 'chromium-host.json'
$firefoxManifest = Join-Path $statePath 'firefox-host.json'
[IO.File]::WriteAllText($chromiumManifest,(@{name='org.stratafetch.host';description='StrataFetch native download bridge';path=$resolvedBinary;type='stdio';allowed_origins=$origins} | ConvertTo-Json),$utf8)
[IO.File]::WriteAllText($firefoxManifest,(@{name='org.stratafetch.host';description='StrataFetch native download bridge';path=$resolvedBinary;type='stdio';allowed_extensions=@('stratafetch@local')} | ConvertTo-Json),$utf8)
for ($index=0; $index -lt $registryPaths.Count; $index++) {New-Item -Force -Path $registryPaths[$index] | Out-Null;Set-Item -LiteralPath $registryPaths[$index] -Value $(if ($index -eq 2) {$firefoxManifest} else {$chromiumManifest})}
Write-Output 'Registered for this user. Keep the binary at its registered path.'
