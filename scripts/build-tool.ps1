# NetDesk: build a tool from the source in vendor/ and put the result in
# %LOCALAPPDATA%\NetDesk\bin (NetDesk finds it there with "Auto-detect").
# This script only builds. It never runs the built tool.
#
#   powershell -ExecutionPolicy Bypass -File scripts\build-tool.ps1 sing-box
param(
  [Parameter(Mandatory = $true)]
  [ValidateSet("sing-box", "xray", "flowdriver", "sni-spoofing-go", "sni-spoofing-rust", "sni-spoofing-py", "master-http-relay", "mhr-cfw")]
  [string]$Tool
)
$ErrorActionPreference = "Stop"
$repo = Split-Path $PSScriptRoot -Parent
$base = Join-Path $env:LOCALAPPDATA "NetDesk"
$bin = Join-Path $base "bin"
New-Item -ItemType Directory -Force $bin | Out-Null

function Need($cmd, $hint) {
  if (-not (Get-Command $cmd -ErrorAction SilentlyContinue)) { throw "'$cmd' not found. Install it first: $hint" }
}
function Check($what) { if ($LASTEXITCODE -ne 0) { throw "$what failed (exit $LASTEXITCODE)" } }

function Build-Go($dir, $pkg, $out, $extra) {
  Need go "winget install GoLang.Go"
  Push-Location (Join-Path $repo "vendor\$dir")
  try {
    $target = Join-Path $bin $out
    # Slow or unstable networks often drop module downloads. Downloaded modules
    # stay cached, so each retry makes progress.
    # Fall back to a mirror if the default module proxy keeps failing. This is safe:
    # Go verifies every module against go.sum. Your own GOPROXY setting wins.
    if (-not $env:GOPROXY) { $env:GOPROXY = "https://proxy.golang.org|https://goproxy.io|direct" }
    # "go build" fetches only what this platform needs, so retry the build itself.
    $max = 15
    for ($i = 1; $i -le $max; $i++) {
      & go build -trimpath @extra -o $target $pkg
      if ($LASTEXITCODE -eq 0) { break }
      if ($i -lt $max) {
        Write-Host "Build or download interrupted, retrying ($i/$max)..." -ForegroundColor Yellow
        Start-Sleep 3
      }
    }
    Check "go build"
    Done $target
  } finally { Pop-Location }
}

function Setup-Python($dir, $name) {
  Need python "winget install Python.Python.3.12"
  # Work on a copy so config files and keys never end up inside the repository
  $work = Join-Path $base "tools\$name"
  New-Item -ItemType Directory -Force $work | Out-Null
  Copy-Item (Join-Path $repo "vendor\$dir\*") $work -Recurse -Force
  & python -m venv (Join-Path $work ".venv"); Check "python -m venv"
  $py = Join-Path $work ".venv\Scripts\python.exe"
  $req = Join-Path $work "requirements.txt"
  if (Test-Path $req) { & $py -m pip install -r $req; Check "pip install" }
  Write-Host ""
  Write-Host "Ready." -ForegroundColor Green
  Write-Host "  Executable : $py"
  Write-Host "  Arguments  : main.py"
  Write-Host "  Work folder: $work"
}

function Done($path) {
  Write-Host ""
  Write-Host "Built: $path" -ForegroundColor Green
  # .NET directly: Get-FileHash is unavailable when Windows PowerShell is started from PowerShell 7
  $stream = [IO.File]::OpenRead($path)
  try { $hash = [Security.Cryptography.SHA256]::Create().ComputeHash($stream) } finally { $stream.Dispose() }
  Write-Host "SHA-256: $(([BitConverter]::ToString($hash) -replace '-', '').ToLower())"
}

switch ($Tool) {
  "sing-box" {
    # with_external_windivert: do not embed the WinDivert driver (not vendored here; NetDesk does not use it)
    $ld = (Get-Content (Join-Path $repo "vendor\sing-box\release\LDFLAGS") -Raw).Trim()
    Build-Go "sing-box" "./cmd/sing-box" "sing-box.exe" @("-tags", "with_quic,with_utls,with_clash_api,badlinkname,tfogo_checklinkname0,with_external_windivert", "-ldflags", "$ld -s -w")
  }
  "xray" { Build-Go "Xray-core" "./main" "xray.exe" @("-ldflags", "-s -w") }
  "flowdriver" {
    Build-Go "FlowDriver" "./cmd/client" "flowdriver-client.exe" @()
    Build-Go "FlowDriver" "./cmd/server" "flowdriver-server.exe" @()
  }
  "sni-spoofing-go" {
    Build-Go "SNI-Spoofing-Go" "." "sni-spoofing-go.exe" @()
    Write-Host "Note: on Windows this tool also needs the official WinDivert driver files next to the exe (see the tutorial)."
  }
  "sni-spoofing-rust" {
    Need cargo "winget install Rustlang.Rustup"
    Push-Location (Join-Path $repo "vendor\sni-spoofing-rust")
    try {
      & cargo build --release --target-dir (Join-Path $base "build\sni-rs"); Check "cargo build"
      $exe = Get-ChildItem (Join-Path $base "build\sni-rs\release") -Filter "sni-spoof*.exe" | Select-Object -First 1
      if (-not $exe) { throw "Build finished but no sni-spoof*.exe was found" }
      Copy-Item $exe.FullName (Join-Path $bin $exe.Name) -Force
      Done (Join-Path $bin $exe.Name)
    } finally { Pop-Location }
    Write-Host "Note: on Windows this tool also needs the official WinDivert driver files next to the exe (see the tutorial)."
  }
  "sni-spoofing-py" { Setup-Python "SNI-Spoofing" "sni-spoofing-py" }
  "master-http-relay" { Setup-Python "MasterHttpRelayVPN" "master-http-relay" }
  "mhr-cfw" { Setup-Python "mhr-cfw" "mhr-cfw" }
}
