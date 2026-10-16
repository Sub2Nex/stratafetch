param([string]$BinaryPath,[ValidatePattern('^[a-p]{32}$')][string]$ChromiumExtensionId,[ValidatePattern('^[a-p]{32}$')][string]$EdgeExtensionId,[switch]$Uninstall,[ValidatePattern('^HKCU:\\Software\\StrataFetch-Test-[a-f0-9-]{36}$')][string]$TestRegistryRoot)
$ErrorActionPreference = 'Stop'
$statePath = if ($env:STRATAFETCH_STATE_DIR) { $env:STRATAFETCH_STATE_DIR } else { Join-Path $env:LOCALAPPDATA 'StrataFetch\StrataFetch\data' }
$registryPaths = @('HKCU:\Software\Google\Chrome\NativeMessagingHosts\org.stratafetch.host','HKCU:\Software\Microsoft\Edge\NativeMessagingHosts\org.stratafetch.host','HKCU:\Software\Mozilla\NativeMessagingHosts\org.stratafetch.host')
if ($TestRegistryRoot) {
    if (-not $env:STRATAFETCH_STATE_DIR) { throw 'Isolated registration test requires STRATAFETCH_STATE_DIR' }
    $registryPaths = @('Chrome','Edge','Firefox') | ForEach-Object { Join-Path $TestRegistryRoot $_ }
}
$originsPath = Join-Path $statePath 'browser-origins.json'
$recordPath = Join-Path $statePath 'browser-registration-windows.json'
$current = @(if (Test-Path -LiteralPath $originsPath) { Get-Content -LiteralPath $originsPath -Raw | ConvertFrom-Json })
$record = if (Test-Path -LiteralPath $recordPath) { Get-Content -LiteralPath $recordPath -Raw | ConvertFrom-Json } else { $null }
foreach ($registryPath in $registryPaths) {
    if (Test-Path -LiteralPath $registryPath) {
        $manifestPath = (Get-Item -LiteralPath $registryPath).GetValue('')
        if ($BinaryPath) {
            if (-not $manifestPath -or -not (Test-Path -LiteralPath $manifestPath)) { throw 'Cannot verify browser registration ownership. Registration was preserved.' }
            $registered = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
            $expected = if ($Uninstall) { [IO.Path]::GetFullPath($BinaryPath) } else { (Resolve-Path -LiteralPath $BinaryPath).Path }
            if ($registered.path -ne $expected) { throw 'Another installation owns browser registration. Unregister that installation first; registration was preserved.' }
        }
    }
}
$utf8 = New-Object System.Text.UTF8Encoding($false)
if ($Uninstall) {
    foreach ($registryPath in $registryPaths) { if (Test-Path -LiteralPath $registryPath) { Remove-Item -LiteralPath $registryPath } }
    $remaining = if ($record) { @(@($current | Where-Object { $_ -notin $record.managed }) + @($record.baseline) | Select-Object -Unique) } else { @() }
    if ($remaining.Count) { [IO.File]::WriteAllText($originsPath,(ConvertTo-Json -InputObject @($remaining)),$utf8) }
    elseif (Test-Path -LiteralPath $originsPath) { Remove-Item -LiteralPath $originsPath }
    foreach ($filename in @('chromium-host.json','firefox-host.json','browser-registration-windows.json')) {
        $manifestPath = Join-Path $statePath $filename
        if (Test-Path -LiteralPath $manifestPath) { Remove-Item -LiteralPath $manifestPath }
    }
    Write-Output 'Browser registration removed. Download state and unrelated origins preserved.'
    return
}
if (-not $BinaryPath -or -not $ChromiumExtensionId) { throw 'Registration requires -BinaryPath and -ChromiumExtensionId.' }
$resolvedBinary = (Resolve-Path -LiteralPath $BinaryPath).Path
if ([IO.Path]::GetFileName($resolvedBinary) -ne 'stratafetch-browser-host.exe') { throw 'Select stratafetch-browser-host.exe.' }
if ($record -and $record.binary -ne $resolvedBinary) { throw 'Another installation owns browser registration. Unregister it first.' }
New-Item -ItemType Directory -Force -Path $statePath | Out-Null
$managed = @("chrome-extension://$ChromiumExtensionId/",'stratafetch@local')
if ($EdgeExtensionId -and $EdgeExtensionId -ne $ChromiumExtensionId) { $managed += "chrome-extension://$EdgeExtensionId/" }
$baseline = if ($record) { @(@($current | Where-Object { $_ -notin $record.managed }) + @($record.baseline) | Select-Object -Unique) } else { @($current) }
$allOrigins = @(@($baseline) + @($managed) | Select-Object -Unique)
$chromiumOrigins = @($allOrigins | Where-Object { $_ -match '^chrome-extension://[a-p]{32}/$' })
$chromiumManifest = Join-Path $statePath 'chromium-host.json'
$firefoxManifest = Join-Path $statePath 'firefox-host.json'
[IO.File]::WriteAllText($originsPath,(ConvertTo-Json -InputObject @($allOrigins)),$utf8)
[IO.File]::WriteAllText($chromiumManifest,(@{name='org.stratafetch.host';description='StrataFetch native download bridge';path=$resolvedBinary;type='stdio';allowed_origins=$chromiumOrigins} | ConvertTo-Json),$utf8)
[IO.File]::WriteAllText($firefoxManifest,(@{name='org.stratafetch.host';description='StrataFetch native download bridge';path=$resolvedBinary;type='stdio';allowed_extensions=@('stratafetch@local')} | ConvertTo-Json),$utf8)
[IO.File]::WriteAllText($recordPath,(@{binary=$resolvedBinary;managed=$managed;baseline=$baseline} | ConvertTo-Json),$utf8)
for ($index=0; $index -lt $registryPaths.Count; $index++) {New-Item -Force -Path $registryPaths[$index] | Out-Null;Set-Item -LiteralPath $registryPaths[$index] -Value $(if ($index -eq 2) {$firefoxManifest} else {$chromiumManifest})}
Write-Output 'Local native bridge ready. Load the bundled extension through the browser to approve it.'
