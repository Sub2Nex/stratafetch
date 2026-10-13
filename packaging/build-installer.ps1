param([Parameter(Mandatory=$true)][string]$Compiler)
$ErrorActionPreference = 'Stop'
$workspacePath = Split-Path -Parent $PSScriptRoot
$packagePath = Join-Path $workspacePath 'dist\stratafetch-windows-x64'
if (-not (Test-Path -LiteralPath (Join-Path $packagePath 'stratafetch-desktop.exe'))) { throw 'Run portable.ps1 first.' }
$instructions = [Collections.Generic.List[string]]::new()
foreach ($subdirectory in @('docs','extensions','packaging')) {
    $basePath = Join-Path $packagePath $subdirectory
    foreach ($file in Get-ChildItem -LiteralPath $basePath -Recurse -File) {
        $relativePath = $file.FullName.Substring($packagePath.Length + 1)
        if ($relativePath -match '["$\r\n]') { throw 'Unsafe installer filename.' }
        $instructions.Add('Delete "$INSTDIR\' + $relativePath + '"')
    }
    foreach ($directory in (Get-ChildItem -LiteralPath $basePath -Recurse -Directory | Sort-Object { $_.FullName.Length } -Descending)) {
        $instructions.Add('RMDir "$INSTDIR\' + $directory.FullName.Substring($packagePath.Length + 1) + '"')
    }
    $instructions.Add('RMDir "$INSTDIR\' + $subdirectory + '"')
}
$instructions | Set-Content -LiteralPath (Join-Path $PSScriptRoot 'installer-files.nsh') -Encoding utf8
Push-Location $PSScriptRoot
try {
    & $Compiler /V3 '/DPACKAGE_DIR=..\dist\stratafetch-windows-x64' installer.nsi
    if ($LASTEXITCODE -ne 0) { throw "NSIS failed: $LASTEXITCODE" }
} finally { Pop-Location }
$installerPath = Join-Path $workspacePath 'dist\stratafetch-windows-x64-setup.exe'
$hash = (Get-FileHash -LiteralPath $installerPath -Algorithm SHA256).Hash.ToLowerInvariant()
"$hash  stratafetch-windows-x64-setup.exe" | Add-Content -LiteralPath (Join-Path $workspacePath 'dist\SHA256SUMS') -Encoding ascii
Write-Output $installerPath
