param([ValidateSet('x86_64-pc-windows-msvc','x86_64-pc-windows-gnu')][string]$Target = 'x86_64-pc-windows-msvc')
$ErrorActionPreference = 'Stop'
$workspacePath = Split-Path -Parent $PSScriptRoot
python (Join-Path $workspacePath 'scripts/verify-windows-manifest.py') (Join-Path $workspacePath 'target/release/stratafetch.exe') (Join-Path $workspacePath 'target/release/stratafetch-desktop.exe') (Join-Path $workspacePath 'target/release/stratafetch-browser-host.exe')
if ($LASTEXITCODE -ne 0) { throw 'Windows manifest validation failed; rebuild before packaging' }
$outputPath = Join-Path $workspacePath 'dist'
$packagePath = Join-Path $outputPath 'stratafetch-windows-x64'
# This path is generated staging only. Fresh contents prevent removed scripts,
# old extensions, or earlier package files leaking into an upgrade archive.
$resolvedPackagePath = [IO.Path]::GetFullPath($packagePath)
$expectedPackagePath = [IO.Path]::GetFullPath((Join-Path $workspacePath 'dist\stratafetch-windows-x64'))
if ($resolvedPackagePath -ne $expectedPackagePath -or -not $resolvedPackagePath.StartsWith([IO.Path]::GetFullPath($workspacePath) + [IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)) { throw 'Package staging path escaped workspace' }
if (Test-Path -LiteralPath $resolvedPackagePath) { Remove-Item -LiteralPath $resolvedPackagePath -Recurse -Force }
New-Item -ItemType Directory -Force -Path $packagePath | Out-Null
$sourceCommit = git -C $workspacePath rev-parse HEAD
$manifestText = Get-Content -LiteralPath (Join-Path $workspacePath 'Cargo.toml') -Raw
if ($manifestText -notmatch '(?m)^version = "([^"]+)"\r?$') { throw 'Cannot determine workspace version' }
$packageVersion = $Matches[1]
$dirtySource = [bool](git -C $workspacePath status --porcelain --untracked-files=normal)
@("StrataFetch $packageVersion - unsigned development preview","Source commit: $sourceCommit","Working tree modified: $dirtySource","Target: $Target","Unsigned binaries; Windows may block execution. Do not bypass application-control policies.","See docs/RELEASE-NOTES.md for current evidence and limitations.") | Set-Content -LiteralPath (Join-Path $packagePath 'BUILD.txt') -Encoding ascii
foreach ($binaryName in @('stratafetch.exe','stratafetch-desktop.exe','stratafetch-browser-host.exe')) {
    $binaryPath = Join-Path $workspacePath "target\release\$binaryName"
    if (-not (Test-Path -LiteralPath $binaryPath)) {throw "Build release binary first: $binaryName"}
    Copy-Item -LiteralPath $binaryPath -Destination $packagePath
}
foreach ($documentName in @('README.md','LICENSE-MIT','LICENSE-APACHE','THIRD-PARTY-LICENSES.txt','PRIVACY.md','SECURITY.md','ARCHITECTURE.md','CONTRIBUTING.md','CODE_OF_CONDUCT.md','ROADMAP.md','CHANGELOG.md')) {Copy-Item -LiteralPath (Join-Path $workspacePath $documentName) -Destination $packagePath}
Copy-Item -LiteralPath (Join-Path $workspacePath 'extensions') -Destination $packagePath -Recurse -Force
Copy-Item -LiteralPath (Join-Path $workspacePath 'docs') -Destination $packagePath -Recurse -Force
Copy-Item -LiteralPath $PSScriptRoot -Destination $packagePath -Recurse -Force
$archivePath = Join-Path $outputPath 'stratafetch-windows-x64.zip'
Compress-Archive -LiteralPath $packagePath -DestinationPath $archivePath -Force
# Offer the desktop directly as well as the full CLI/browser integration bundle.
$desktopPath = Join-Path $outputPath 'stratafetch-desktop.exe'
Copy-Item -LiteralPath (Join-Path $packagePath 'stratafetch-desktop.exe') -Destination $desktopPath -Force
$checksums = foreach ($artifactPath in @($archivePath, $desktopPath)) {
    $hash = (Get-FileHash -LiteralPath $artifactPath -Algorithm SHA256).Hash.ToLowerInvariant()
    "$hash  $([IO.Path]::GetFileName($artifactPath))"
}
$checksums | Set-Content -LiteralPath (Join-Path $outputPath 'SHA256SUMS') -Encoding ascii
Write-Output $archivePath
Write-Output $desktopPath
