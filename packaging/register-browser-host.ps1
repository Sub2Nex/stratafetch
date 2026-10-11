param([Parameter(Mandatory=$true)][string]$BinaryPath,[Parameter(Mandatory=$true)][ValidatePattern('^[a-p]{32}$')][string]$ChromiumExtensionId,[switch]$Uninstall)
$ErrorActionPreference = 'Stop'
$statePath = Join-Path $env:LOCALAPPDATA 'StrataFetch\StrataFetch\data'
$registryPaths = @('HKCU:\Software\Google\Chrome\NativeMessagingHosts\org.stratafetch.host','HKCU:\Software\Microsoft\Edge\NativeMessagingHosts\org.stratafetch.host','HKCU:\Software\Mozilla\NativeMessagingHosts\org.stratafetch.host')
if ($Uninstall) { foreach ($registryPath in $registryPaths) { if (Test-Path -LiteralPath $registryPath) { Remove-Item -LiteralPath $registryPath } }; return }
$resolvedBinary = (Resolve-Path -LiteralPath $BinaryPath).Path
New-Item -ItemType Directory -Force -Path $statePath | Out-Null
$origin = "chrome-extension://$ChromiumExtensionId/"
@($origin,'stratafetch@local') | ConvertTo-Json | Set-Content -Encoding utf8NoBOM -LiteralPath (Join-Path $statePath 'browser-origins.json')
$chromiumManifest = Join-Path $statePath 'chromium-host.json'
$firefoxManifest = Join-Path $statePath 'firefox-host.json'
@{name='org.stratafetch.host';description='StrataFetch native download bridge';path=$resolvedBinary;type='stdio';allowed_origins=@($origin)} | ConvertTo-Json | Set-Content -Encoding utf8NoBOM -LiteralPath $chromiumManifest
@{name='org.stratafetch.host';description='StrataFetch native download bridge';path=$resolvedBinary;type='stdio';allowed_extensions=@('stratafetch@local')} | ConvertTo-Json | Set-Content -Encoding utf8NoBOM -LiteralPath $firefoxManifest
for ($index=0; $index -lt $registryPaths.Count; $index++) {New-Item -Force -Path $registryPaths[$index] | Out-Null;Set-Item -LiteralPath $registryPaths[$index] -Value $(if ($index -eq 2) {$firefoxManifest} else {$chromiumManifest})}
Write-Output 'Registered for this user. Keep the binary at its registered path.'
