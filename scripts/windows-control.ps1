param([ValidateSet('start','stop','status','logs')][string]$Action)
$ErrorActionPreference = 'Stop'
$studioExecutable = 'D:\devw\my\tools\simpleVoiceover\simpleVoiceover.exe'
$studioUrl = 'http://127.0.0.1:5174'
$studioLog = Join-Path $env:LOCALAPPDATA 'simpleVoiceover\simpleVoiceover.log'
function Get-StudioStatus {
    try { return Invoke-RestMethod -Uri "$studioUrl/api/health" -TimeoutSec 2 }
    catch { return $null }
}
switch ($Action) {
    'start' {
        $studioStatus = Get-StudioStatus
        if ($studioStatus) { Write-Output "simpleVoiceover already running (pid $($studioStatus.pid)): $studioUrl"; exit 0 }
        if (-not (Test-Path -LiteralPath $studioExecutable)) { throw 'Build the app first: bin/build-windows in WSL' }
        Start-Process -FilePath $studioExecutable -WindowStyle Normal | Out-Null
        for ($studioAttempt = 0; $studioAttempt -lt 30; $studioAttempt++) {
            Start-Sleep -Milliseconds 200
            $studioStatus = Get-StudioStatus
            if ($studioStatus) { Write-Output "simpleVoiceover started (pid $($studioStatus.pid)): $studioUrl"; exit 0 }
        }
        throw "simpleVoiceover did not start. See $studioLog"
    }
    'stop' {
        $studioStatus = Get-StudioStatus
        if (-not $studioStatus) { Write-Output 'simpleVoiceover is stopped'; exit 0 }
        Invoke-RestMethod -Method Post -Uri "$studioUrl/api/close" -TimeoutSec 5 | Out-Null
        for ($studioAttempt = 0; $studioAttempt -lt 40; $studioAttempt++) {
            Start-Sleep -Milliseconds 150
            if (-not (Get-StudioStatus)) { Write-Output 'simpleVoiceover stopped'; exit 0 }
        }
        Write-Output 'Close requested. Check the app window: save or discard unsaved changes, or finish the current operation.'
    }
    'status' {
        $studioStatus = Get-StudioStatus
        if ($studioStatus) { $studioStatus | Format-List } else { Write-Output 'simpleVoiceover is stopped' }
    }
    'logs' {
        if (Test-Path -LiteralPath $studioLog) { Get-Content -LiteralPath $studioLog -Tail 100 -Wait }
        else { Write-Output 'No logs yet. Run: svoice start' }
    }
}
