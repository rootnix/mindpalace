# mindpalace installer (Windows)
#   irm https://raw.githubusercontent.com/rootnix/mindpalace/main/install.ps1 | iex
$ErrorActionPreference = "Stop"

$Repo = if ($env:MP_REPO) { $env:MP_REPO } else { "https://github.com/rootnix/mindpalace.git" }
$ReleaseBase = if ($env:MP_RELEASE_BASE) { $env:MP_RELEASE_BASE } else { "https://github.com/rootnix/mindpalace/releases/latest/download" }
$InstallDir = if ($env:MP_INSTALL_DIR) { $env:MP_INSTALL_DIR } else { Join-Path $HOME ".local\share\mindpalace" }

if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    Write-Error "mindpalace: git is required (https://git-scm.com)"
}

# 1. repo checkout — integrations (Claude plugin, skill) + templates live here
if (Test-Path (Join-Path $InstallDir ".git")) {
    Write-Host "updating existing install at $InstallDir"
    git -C $InstallDir pull --ff-only -q
} else {
    Write-Host "installing to $InstallDir"
    New-Item -ItemType Directory -Force -Path (Split-Path $InstallDir) | Out-Null
    git clone --depth 1 -q $Repo $InstallDir
}
$BinDir = Join-Path $InstallDir "bin"
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

# 2. mp binary: prebuilt release download > cargo build
$Exe = Join-Path $BinDir "mp.exe"
try {
    Invoke-WebRequest -Uri "$ReleaseBase/mp-windows-x64.exe" -OutFile $Exe -UseBasicParsing
    Write-Host "installed prebuilt binary (mp-windows-x64.exe)"
} catch {
    if (Get-Command cargo -ErrorAction SilentlyContinue) {
        Write-Host "prebuilt binary unavailable — building from source (cargo)..."
        Push-Location $InstallDir
        cargo build --release -q
        Pop-Location
        Copy-Item (Join-Path $InstallDir "target\release\mp.exe") $Exe -Force
    } else {
        Write-Error "no prebuilt binary and cargo is not installed — install Rust (https://rustup.rs) and re-run"
    }
}

# 3. add the bin dir to the user PATH
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$BinDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$BinDir", "User")
    $env:Path = "$env:Path;$BinDir"
    Write-Host "added $BinDir to your user PATH (restart open terminals)"
}

Write-Host ""
Write-Host "mindpalace installed. next:"
Write-Host "  mp init -g        # create your wiki + auto-integrate agent tools"
