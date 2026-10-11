$ErrorActionPreference = 'Stop'
$workspacePath = Split-Path -Parent $PSScriptRoot
$outputPath = Join-Path $workspacePath 'dist'
$packagePath = Join-Path $outputPath 'stratafetch-windows-x64'
New-Item -ItemType Directory -Force -Path $packagePath | Out-Null
$sourceCommit = git -C $workspacePath rev-parse HEAD
@("StrataFetch development preview","Source commit: $sourceCommit","Target: x86_64-pc-windows-msvc","Unsigned binaries. Native launch was blocked by Windows application control during local verification.","See docs/VERIFICATION.md for evidence and limitations.") | Set-Content -LiteralPath (Join-Path $packagePath 'BUILD.txt') -Encoding ascii
foreach ($binaryName in @('stratafetch.exe','stratafetch-desktop.exe','stratafetch-browser-host.exe')) {
    $binaryPath = Join-Path $workspacePath "target\release\$binaryName"
    if (-not (Test-Path -LiteralPath $binaryPath)) {throw "Build release binary first: $binaryName"}
    Copy-Item -LiteralPath $binaryPath -Destination $packagePath
}
foreach ($documentName in @('README.md','LICENSE-MIT','LICENSE-APACHE','THIRD-PARTY-LICENSES.txt','PRIVACY.md','SECURITY.md')) {Copy-Item -LiteralPath (Join-Path $workspacePath $documentName) -Destination $packagePath}
Copy-Item -LiteralPath (Join-Path $workspacePath 'extensions') -Destination $packagePath -Recurse -Force
Copy-Item -LiteralPath (Join-Path $workspacePath 'docs') -Destination $packagePath -Recurse -Force
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'register-browser-host.ps1') -Destination $packagePath
$archivePath = Join-Path $outputPath 'stratafetch-windows-x64.zip'
Compress-Archive -LiteralPath $packagePath -DestinationPath $archivePath -Force
$hash = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
"$hash  stratafetch-windows-x64.zip" | Set-Content -LiteralPath (Join-Path $outputPath 'SHA256SUMS') -Encoding ascii
Write-Output $archivePath
