# NetDesk server setup. Runs on YOUR OWN Windows server (started by INSTALL.cmd).
# Downloads nothing. Uses only the files in this folder.
param([switch]$Uninstall)
$ErrorActionPreference = 'Stop'
$TaskName = 'NetDeskServer'
$RuleName = 'NetDesk Server'
$Dir = Join-Path $env:ProgramFiles 'NetDeskServer'
$Exe = Join-Path $Dir 'sing-box.exe'
$Cfg = Join-Path $Dir 'config.json'

function Step($m) { Write-Host "[*] $m" -ForegroundColor Cyan }

function Remove-OldInstall {
    if (Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue) {
        Stop-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
        Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false
    }
    Get-Process -Name 'sing-box' -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $Exe } | Stop-Process -Force
    Get-NetFirewallRule -DisplayName $RuleName -ErrorAction SilentlyContinue | Remove-NetFirewallRule
    Start-Sleep -Seconds 1
}

function Get-Sha256($path) {
    $sha = [System.Security.Cryptography.SHA256]::Create()
    $stream = [System.IO.File]::OpenRead($path)
    try { return ([BitConverter]::ToString($sha.ComputeHash($stream)) -replace '-', '').ToLower() }
    finally { $stream.Dispose(); $sha.Dispose() }
}

try {
    $me = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
    if (-not $me.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'Administrator rights are required. Without them the server cannot be installed.'
    }

    if ($Uninstall) {
        Remove-OldInstall
        if (Test-Path -LiteralPath $Dir) { Remove-Item -LiteralPath $Dir -Recurse -Force }
        Write-Host 'Removed.' -ForegroundColor Green
        return
    }

    if (-not [Environment]::Is64BitOperatingSystem -or [Environment]::OSVersion.Version.Major -lt 10) {
        throw 'Needs 64-bit Windows 10 or Windows Server 2016 or newer.'
    }

    $kit = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'kit.json') -Raw | ConvertFrom-Json
    $port = [int]$kit.port
    $srcExe = Join-Path $PSScriptRoot 'sing-box.exe'
    $srcCfg = Join-Path $PSScriptRoot 'config.json'
    if (-not (Test-Path -LiteralPath $srcCfg)) {
        throw 'config.json is missing. Copy the whole kit folder from your computer again.'
    }

    Step 'Checking sing-box.exe'
    if ((Get-Sha256 $srcExe) -ne $kit.sha256) { throw 'sing-box.exe is damaged or was changed. Copy the folder again.' }

    Step 'Removing a previous NetDesk server (if any)'
    Remove-OldInstall

    Step "Checking that TCP port $port is free"
    $busy = Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($busy) {
        $name = (Get-Process -Id $busy.OwningProcess -ErrorAction SilentlyContinue).ProcessName
        throw "Port $port is already used by '$name'. Create a new kit with another port (for example 8443)."
    }

    Step 'Copying files'
    New-Item -ItemType Directory -Force -Path $Dir | Out-Null
    # Lock the folder BEFORE the secret config is written: SYSTEM and Administrators full, LOCAL SERVICE read only.
    & icacls $Dir /inheritance:r /grant:r '*S-1-5-18:(OI)(CI)F' '*S-1-5-32-544:(OI)(CI)F' '*S-1-5-19:(OI)(CI)RX' | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not protect the install folder.' }
    Copy-Item -LiteralPath $srcExe -Destination $Exe -Force
    Copy-Item -LiteralPath $srcCfg -Destination $Cfg -Force

    Step 'Checking the configuration'
    & $Exe check -c $Cfg
    if ($LASTEXITCODE -ne 0) { throw 'sing-box rejected the configuration.' }

    Step 'Opening the Windows firewall for this program and port only'
    New-NetFirewallRule -DisplayName $RuleName -Direction Inbound -Action Allow -Protocol TCP -LocalPort $port -Program $Exe | Out-Null

    Step 'Creating the autostart task (runs as LOCAL SERVICE, not as admin)'
    $action = New-ScheduledTaskAction -Execute $Exe -Argument "run -c `"$Cfg`""
    $trigger = New-ScheduledTaskTrigger -AtStartup
    $principal = New-ScheduledTaskPrincipal -UserId 'NT AUTHORITY\LOCAL SERVICE' -LogonType ServiceAccount
    $settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit ([TimeSpan]::Zero) -RestartCount 10 -RestartInterval (New-TimeSpan -Minutes 1) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable
    Register-ScheduledTask -TaskName $TaskName -Action $action -Trigger $trigger -Principal $principal -Settings $settings | Out-Null
    Start-ScheduledTask -TaskName $TaskName

    Step 'Waiting for the server to start'
    $ok = $false
    for ($i = 0; $i -lt 20 -and -not $ok; $i++) {
        Start-Sleep -Milliseconds 700
        $ok = [bool](Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue)
    }
    if (-not $ok) { throw 'The server did not start. An antivirus may be blocking sing-box.exe.' }

    Step "Checking that this server can reach $($kit.sni)"
    if (-not (Test-NetConnection -ComputerName $kit.sni -Port 443 -InformationLevel Quiet -WarningAction SilentlyContinue)) {
        Write-Host "WARNING: cannot reach $($kit.sni):443. Create a new kit with another SNI site." -ForegroundColor Yellow
    }

    # The key now lives only in the protected folder.
    Remove-Item -LiteralPath $srcCfg -Force -ErrorAction SilentlyContinue

    Write-Host ''
    Write-Host 'DONE. The server is running and will start again after a reboot.' -ForegroundColor Green
    Write-Host "If NetDesk cannot connect, also open TCP port $port in your provider's panel firewall."
    Write-Host 'Now go back to NetDesk on your own computer and press the connect button.'
}
catch {
    Write-Host ''
    Write-Host "FAILED: $($_.Exception.Message)" -ForegroundColor Red
}
