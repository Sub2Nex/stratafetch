$ErrorActionPreference = 'Stop'
$workspacePath = Split-Path -Parent $PSScriptRoot
$resultPath = Join-Path $workspacePath ('.smoke\scalability-' + [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds())
New-Item -ItemType Directory -Path $resultPath | Out-Null
$binaryPath = Join-Path $workspacePath 'target\release\examples\scalability.exe'
$benchmarkProcess = Start-Process -FilePath $binaryPath -ArgumentList ('"' + $resultPath + '"') -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $resultPath 'stdout.log') -RedirectStandardError (Join-Path $resultPath 'stderr.log')
$samples = @{}
while (-not $benchmarkProcess.HasExited) {
    $stagePath = Join-Path $resultPath 'stage.json'
    if (Test-Path -LiteralPath $stagePath) {
        try { $stage = Get-Content -LiteralPath $stagePath -Raw | ConvertFrom-Json } catch { $stage = $null }
        if ($stage.stage -eq 'queued_idle') {
            $benchmarkProcess.Refresh()
            $key = [string]$stage.count
            if (-not $samples.ContainsKey($key)) { $samples[$key] = [Collections.Generic.List[object]]::new() }
            $samples[$key].Add(@{seconds=[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()/1000.0; cpuSeconds=$benchmarkProcess.TotalProcessorTime.TotalSeconds; workingSetBytes=$benchmarkProcess.WorkingSet64; privateBytes=$benchmarkProcess.PrivateMemorySize64})
        }
    }
    Start-Sleep -Milliseconds 200
}
$benchmarkProcess.WaitForExit()
if ($benchmarkProcess.ExitCode -ne 0) { throw (Get-Content -LiteralPath (Join-Path $resultPath 'stderr.log') -Raw) }
$metrics = foreach ($key in @('100','1000','10000')) {
    $values = $samples[$key]
    if ($values.Count -lt 5) { throw "Too few process samples for $key" }
    $elapsed = $values[-1].seconds - $values[0].seconds
    @{records=[int]$key; samples=$values.Count; sampleSeconds=$elapsed; cpuSeconds=$values[-1].cpuSeconds-$values[0].cpuSeconds; oneCoreCpuPercent=100*($values[-1].cpuSeconds-$values[0].cpuSeconds)/$elapsed; peakWorkingSetBytes=($values.workingSetBytes | Measure-Object -Maximum).Maximum; peakPrivateBytes=($values.privateBytes | Measure-Object -Maximum).Maximum}
}
$report = @{timestampUtc=[DateTimeOffset]::UtcNow.ToString('o'); os=[Environment]::OSVersion.VersionString; logicalProcessors=[Environment]::ProcessorCount; profile='release'; workload='persisted paused records, then all queued for u64::MAX; no HTTP transfers'; latency=(Get-Content -LiteralPath (Join-Path $resultPath 'latency.json') -Raw | ConvertFrom-Json); process=$metrics}
$report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $resultPath 'report.json') -Encoding utf8
Write-Output (Join-Path $resultPath 'report.json')
