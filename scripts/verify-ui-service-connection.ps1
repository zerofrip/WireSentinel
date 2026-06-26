#Requires -Version 5.1
<#
.SYNOPSIS
  Verifies wire-sentinel-service.exe API reachability (token, status, diagnostics, WebSocket).

.EXIT CODES
  0 - all checks passed
  1 - one or more checks failed
#>
param(
    [int]$Port = $(if ($env:WIRESENTINEL_API_PORT) { [int]$env:WIRESENTINEL_API_PORT } else { 8170 }),
    [int]$TimeoutSec = 10
)

$ErrorActionPreference = "Stop"
$base = "http://127.0.0.1:$Port"
$failed = $false

function Write-Step($msg) { Write-Host "==> $msg" -ForegroundColor Cyan }
function Pass($msg) { Write-Host "OK  $msg" -ForegroundColor Green }
function Fail($msg) { Write-Host "FAIL $msg" -ForegroundColor Red; $script:failed = $true }

Write-Step "WireSentinel UI/Service connection check (port $Port)"

# 1. Token (public endpoint)
Write-Step "GET /api/v1/auth/token"
try {
    $tokenResp = Invoke-RestMethod -Uri "$base/api/v1/auth/token" -TimeoutSec $TimeoutSec
    if (-not $tokenResp.token) {
        Fail "token response missing .token field"
    } else {
        Pass "token received (length $($tokenResp.token.Length))"
    }
} catch {
    Fail "auth/token: $_"
    Write-Host "Ensure wire-sentinel-service.exe is running and listening on $Port"
    exit 1
}

$headers = @{ Authorization = "Bearer $($tokenResp.token)" }

# 2. Status
Write-Step "GET /api/v1/status"
try {
    $status = Invoke-RestMethod -Uri "$base/api/v1/status" -Headers $headers -TimeoutSec $TimeoutSec
    if (-not $status.running) {
        Fail "status.running is false"
    } elseif ($status.api_port -ne $Port) {
        Fail "status.api_port=$($status.api_port) does not match expected $Port"
    } else {
        Pass "running=true api_port=$($status.api_port) connections=$($status.connection_count)"
    }
} catch {
    Fail "status: $_"
}

# 3. Diagnostics
Write-Step "GET /api/v1/diagnostics"
try {
    $diag = Invoke-RestMethod -Uri "$base/api/v1/diagnostics" -Headers $headers -TimeoutSec $TimeoutSec
    Pass "diagnostics returned (keys: $($diag.PSObject.Properties.Name -join ', '))"
} catch {
    Fail "diagnostics: $_"
}

# 4. WebSocket (minimal handshake via .NET ClientWebSocket)
Write-Step "WebSocket /api/v1/events"
try {
    Add-Type -AssemblyName System.Net.WebSockets -ErrorAction SilentlyContinue
    $wsUri = [Uri]("ws://127.0.0.1:$Port/api/v1/events?token=$([Uri]::EscapeDataString($tokenResp.token))")
    $cts = New-Object System.Threading.CancellationTokenSource
    $cts.CancelAfter([TimeSpan]::FromSeconds($TimeoutSec))
    $ws = [System.Net.WebSockets.ClientWebSocket]::new()
    $connectTask = $ws.ConnectAsync($wsUri, $cts.Token)
    $connectTask.Wait()
    if ($ws.State -eq [System.Net.WebSockets.WebSocketState]::Open) {
        Pass "WebSocket connected"
        $ws.Abort()
        $ws.Dispose()
    } else {
        Fail "WebSocket state: $($ws.State)"
    }
} catch {
    Fail "WebSocket: $_"
}

# 5. Process check (informational)
Write-Step "Process check"
$svc = Get-Process -Name "wire-sentinel-service" -ErrorAction SilentlyContinue
if ($svc) {
    Pass "wire-sentinel-service.exe running (PID $($svc.Id -join ', '))"
} else {
    Write-Host "WARN wire-sentinel-service process not found (API may be console child)" -ForegroundColor Yellow
}

if ($failed) {
    Write-Host "`nOne or more checks failed." -ForegroundColor Red
    exit 1
}

Write-Host "`nAll checks passed." -ForegroundColor Green
exit 0
