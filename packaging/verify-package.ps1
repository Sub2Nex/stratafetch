param([switch]$PortableOnly)
$ErrorActionPreference = 'Stop'
$workspacePath = Split-Path -Parent $PSScriptRoot
$manifestText = Get-Content -LiteralPath (Join-Path $workspacePath 'Cargo.toml') -Raw
if ($manifestText -notmatch '(?m)^version = "([^"]+)"$') { throw 'Cannot determine workspace version' }
$expectedVersion = $Matches[1]
$reportedVersion = & (Join-Path $workspacePath 'target\release\stratafetch.exe') --version
if ($LASTEXITCODE -ne 0 -or $reportedVersion -ne "stratafetch $expectedVersion") { throw 'CLI binary version differs from workspace; build --bins as well as --examples' }
$archivePath = Join-Path $workspacePath 'dist\stratafetch-windows-x64.zip'
$archive = [IO.Compression.ZipFile]::OpenRead($archivePath)
try {
    foreach ($name in @('stratafetch.exe','stratafetch-desktop.exe','stratafetch-browser-host.exe')) {
        $entries = @($archive.Entries | Where-Object { $_.FullName.Replace('\','/').EndsWith("/$name") })
        if ($entries.Count -ne 1) { throw "Expected one archive executable: $name" }
        $stream = $entries[0].Open()
        $algorithm = [Security.Cryptography.SHA256]::Create()
        try { $hash = [Convert]::ToHexString($algorithm.ComputeHash($stream)).ToLowerInvariant() }
        finally { $algorithm.Dispose(); $stream.Dispose() }
        $expected = (Get-FileHash -LiteralPath (Join-Path $workspacePath "target\release\$name") -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($hash -ne $expected) { throw "Archive executable differs from build: $name" }
    }
    foreach ($name in @('BUILD.txt','LICENSE-MIT','LICENSE-APACHE','THIRD-PARTY-LICENSES.txt','docs/RELEASE-NOTES.md','docs/INSTALLATION.md','extensions/chromium/manifest.json','extensions/firefox/manifest.json','packaging/register-browser-host.ps1')) {
        if (-not @($archive.Entries | Where-Object { $_.FullName.Replace('\','/').EndsWith("/$name") }).Count) {throw "Missing package file: $name"}
    }
    $entry = @($archive.Entries | Where-Object { $_.FullName.Replace('\','/').EndsWith('/BUILD.txt') })[0]
    $reader = [IO.StreamReader]::new($entry.Open())
    try { $metadata = $reader.ReadToEnd() } finally { $reader.Dispose() }
    $sourceCommit = git -C $workspacePath rev-parse HEAD
    if (-not $metadata.Contains("Source commit: $sourceCommit")) {throw 'Package source revision differs from HEAD'}
} finally { $archive.Dispose() }
$desktopPath = Join-Path $workspacePath 'dist\stratafetch-desktop.exe'
$desktopHash = (Get-FileHash -LiteralPath $desktopPath -Algorithm SHA256).Hash.ToLowerInvariant()
$builtDesktopHash = (Get-FileHash -LiteralPath (Join-Path $workspacePath 'target\release\stratafetch-desktop.exe') -Algorithm SHA256).Hash.ToLowerInvariant()
if ($desktopHash -ne $builtDesktopHash) { throw 'Standalone desktop executable differs from build' }
$checkedArtifacts = @{}
foreach ($line in Get-Content -LiteralPath (Join-Path $workspacePath 'dist\SHA256SUMS')) {
    if ($line -notmatch '^([a-f0-9]{64})  ([a-zA-Z0-9._-]+)$') {throw 'Malformed SHA256SUMS'}
    $expectedHash = $Matches[1]; $name = $Matches[2]
    if ($checkedArtifacts.ContainsKey($name)) { throw "Duplicate artifact checksum: $name" }
    $actual = (Get-FileHash -LiteralPath (Join-Path $workspacePath "dist\$name") -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $expectedHash) {throw "Package checksum mismatch: $name"}
    $checkedArtifacts[$name] = $true
}
foreach ($name in @('stratafetch-windows-x64.zip', 'stratafetch-desktop.exe')) {
    if (-not $checkedArtifacts.ContainsKey($name)) { throw "Missing artifact checksum: $name" }
}
if (-not $PortableOnly -and (Get-AuthenticodeSignature -LiteralPath (Join-Path $workspacePath 'dist\stratafetch-windows-x64-setup.exe')).Status -ne 'NotSigned') {throw 'Review unexpected installer signature status'}
Write-Output 'ZIP executable hashes, required files, source revision and artifact checksums verified. Installer checked only when requested.'
