$ErrorActionPreference = 'Stop'
$workspacePath = Split-Path -Parent $PSScriptRoot
$testRoot = 'HKCU:\Software\StrataFetch-Test-' + [guid]::NewGuid().ToString()
if (Test-Path -LiteralPath $testRoot) { throw 'Test registry root already exists' }
$statePath = Join-Path $workspacePath ('.smoke\browser-registration-' + [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds())
New-Item -ItemType Directory -Path $statePath | Out-Null
$previousState = $env:STRATAFETCH_STATE_DIR
$env:STRATAFETCH_STATE_DIR = $statePath
$hostPath = (Resolve-Path -LiteralPath (Join-Path $workspacePath 'target\release\stratafetch-browser-host.exe')).Path
$registerScript = Join-Path $workspacePath 'packaging\register-browser-host.ps1'
try {
    'preserve history' | Set-Content -LiteralPath (Join-Path $statePath 'history-marker.txt')
    & $registerScript -BinaryPath $hostPath -ChromiumExtensionId ('a'*32) -EdgeExtensionId ('b'*32) -TestRegistryRoot $testRoot
    $origins = Get-Content -LiteralPath (Join-Path $statePath 'browser-origins.json') -Raw | ConvertFrom-Json
    if ($origins.Count -ne 3 -or $origins -notcontains ('chrome-extension://' + ('b'*32) + '/')) { throw 'Distinct Edge origin missing' }
    foreach ($browserName in @('Chrome','Edge','Firefox')) {
        $manifestPath = (Get-Item -LiteralPath (Join-Path $testRoot $browserName)).GetValue('')
        $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
        if ($manifest.path -ne $hostPath -or $manifest.type -ne 'stdio') { throw 'Native manifest mismatch' }
    }
    $rejected = $false
    try { & $registerScript -Uninstall -BinaryPath 'C:\other-install\stratafetch-browser-host.exe' -TestRegistryRoot $testRoot } catch { $rejected = $true }
    if (-not $rejected -or -not (Test-Path -LiteralPath (Join-Path $testRoot 'Chrome'))) { throw 'Ownership guard failed' }
    & $registerScript -Uninstall -BinaryPath $hostPath -TestRegistryRoot $testRoot
    foreach ($browserName in @('Chrome','Edge','Firefox')) { if (Test-Path -LiteralPath (Join-Path $testRoot $browserName)) { throw 'Registration left behind' } }
    foreach ($fileName in @('browser-origins.json','chromium-host.json','firefox-host.json')) { if (Test-Path -LiteralPath (Join-Path $statePath $fileName)) { throw 'Manifest left behind' } }
    if (-not (Test-Path -LiteralPath (Join-Path $statePath 'history-marker.txt'))) { throw 'Uninstall removed history' }
    $manual = 'chrome-extension://' + ('c'*32) + '/'
    ConvertTo-Json -InputObject @($manual) | Set-Content -LiteralPath (Join-Path $statePath 'browser-origins.json')
    & $registerScript -BinaryPath $hostPath -ChromiumExtensionId ('a'*32) -EdgeExtensionId ('b'*32) -TestRegistryRoot $testRoot
    & $registerScript -BinaryPath $hostPath -ChromiumExtensionId ('a'*32) -EdgeExtensionId ('b'*32) -TestRegistryRoot $testRoot
    $merged = @(Get-Content -LiteralPath (Join-Path $statePath 'browser-origins.json') -Raw | ConvertFrom-Json)
    if ($merged.Count -ne 4 -or $merged -notcontains $manual) { throw 'Registration lost unrelated origins or duplicated entries' }
    & $registerScript -Uninstall -BinaryPath $hostPath -TestRegistryRoot $testRoot
    $remaining = @(Get-Content -LiteralPath (Join-Path $statePath 'browser-origins.json') -Raw | ConvertFrom-Json)
    if ($remaining.Count -ne 1 -or $remaining[0] -ne $manual) { throw 'Unregister removed unrelated origins' }
    Write-Output 'Isolated real HKCU registration, distinct Chrome/Edge origins, ownership rejection and unregister/data-preservation tests passed.'
} finally {
    $env:STRATAFETCH_STATE_DIR = $previousState
    if ($testRoot -notmatch '^HKCU:\\Software\\StrataFetch-Test-[a-f0-9-]{36}$') { throw 'Unsafe registry cleanup target' }
    if (Test-Path -LiteralPath $testRoot) { Remove-Item -LiteralPath $testRoot -Recurse }
}
