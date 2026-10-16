param(
    [Parameter(Mandatory=$true)][string]$Compiler,
    [string]$PackagePath,
    [string]$OutputPath,
    [string]$TestId
)
$ErrorActionPreference = 'Stop'
$installerBuildArgs = @((Join-Path $PSScriptRoot 'build-installer.py'), '--compiler', $Compiler)
if ($PackagePath) { $installerBuildArgs += @('--package', $PackagePath) }
if ($OutputPath) { $installerBuildArgs += @('--output', $OutputPath) }
if ($TestId) { $installerBuildArgs += @('--test-id', $TestId) }
python @installerBuildArgs
if ($LASTEXITCODE -ne 0) { throw "Installer build failed: $LASTEXITCODE" }
