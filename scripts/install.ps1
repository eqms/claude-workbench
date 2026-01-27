#Requires -Version 5.1
<#
.SYNOPSIS
    Install claude-workbench on Windows.

.DESCRIPTION
    Downloads the latest claude-workbench release from GitHub and installs it,
    or builds from local source with -Local flag.

.PARAMETER Local
    Build from source using cargo instead of downloading from GitHub.

.PARAMETER Check
    Only check dependencies, don't install.

.PARAMETER InstallDir
    Installation directory (default: $env:LOCALAPPDATA\Programs\claude-workbench).

.PARAMETER Help
    Show help message.

.PARAMETER Version
    Show script version.

.EXAMPLE
    .\install.ps1
    # Install latest release from GitHub

.EXAMPLE
    .\install.ps1 -Local
    # Build from local source

.EXAMPLE
    .\install.ps1 -Check
    # Only check dependencies
#>

param(
    [switch]$Local,
    [switch]$Check,
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\claude-workbench",
    [switch]$Help,
    [switch]$Version
)

$ErrorActionPreference = "Stop"

# Configuration
$ScriptVersion = "1.0.0"
$BinaryName = "claude-workbench.exe"
$Repo = "eqms/claude-workbench"
$GitHubUrl = "https://github.com/$Repo"

# --- Help ---

function Write-HelpMessage {
    Write-Host ""
    Write-Host "claude-workbench installer (PowerShell)" -ForegroundColor White
    Write-Host ""
    Write-Host "USAGE:" -ForegroundColor White
    Write-Host "    .\install.ps1 [OPTIONS]"
    Write-Host ""
    Write-Host "OPTIONS:" -ForegroundColor White
    Write-Host "    -Help          Show this help message"
    Write-Host "    -Version       Show script version"
    Write-Host "    -Local         Build from source with cargo (requires Git repo checkout)"
    Write-Host "    -InstallDir    Installation directory"
    Write-Host "                   (default: $env:LOCALAPPDATA\Programs\claude-workbench)"
    Write-Host "    -Check         Only check dependencies, don't install"
    Write-Host ""
    Write-Host "EXAMPLES:" -ForegroundColor White
    Write-Host "    .\install.ps1                          # Install latest release"
    Write-Host "    .\install.ps1 -Local                   # Build from source"
    Write-Host "    .\install.ps1 -Check                   # Check dependencies only"
    Write-Host "    .\install.ps1 -InstallDir C:\Tools     # Custom install directory"
    Write-Host ""
}

# --- UI Helpers ---

function Write-Banner {
    Write-Host ""
    Write-Host "+============================================================+" -ForegroundColor Blue
    Write-Host "|        Claude Workbench -- Installer v$ScriptVersion              |" -ForegroundColor Blue
    Write-Host "+============================================================+" -ForegroundColor Blue
    Write-Host ""
}

function Write-Step {
    param([int]$Step, [int]$Total, [string]$Message)
    Write-Host "[$Step/$Total] $Message" -ForegroundColor Blue
}

function Write-Row {
    param([string]$Content)
    $padding = 56 - $Content.Length
    if ($padding -lt 0) { $padding = 0 }
    $pad = " " * $padding
    Write-Host "| " -ForegroundColor Blue -NoNewline
    Write-Host " $Content$pad" -NoNewline
    Write-Host "|" -ForegroundColor Blue
}

# --- Platform Detection ---

function Get-Platform {
    $arch = $env:PROCESSOR_ARCHITECTURE

    switch ($arch) {
        "AMD64" {
            $script:AssetName = "claude-workbench-x86_64-pc-windows-msvc.zip"
            $script:PlatformArch = "x64"
        }
        "ARM64" {
            $script:AssetName = "claude-workbench-aarch64-pc-windows-msvc.zip"
            $script:PlatformArch = "ARM64"
        }
        default {
            Write-Host "Error: Unsupported architecture: $arch" -ForegroundColor Red
            exit 1
        }
    }

    Write-Host "  Platform:  Windows $script:PlatformArch" -ForegroundColor Cyan
    Write-Host "  Asset:     $script:AssetName" -ForegroundColor DarkGray
    Write-Host ""
}

# --- Dependency Checking ---

function Test-Dependency {
    param(
        [string]$Name,
        [bool]$Required,
        [string]$InstallHint = ""
    )

    $cmd = Get-Command $Name -ErrorAction SilentlyContinue
    if ($cmd) {
        $ver = "installed"
        try {
            $ver = & $Name --version 2>$null | Select-Object -First 1
            if (-not $ver) { $ver = "installed" }
        } catch {
            $ver = "installed"
        }
        Write-Host "  [OK] $Name ($ver)" -ForegroundColor Green
        return $true
    } else {
        if ($Required) {
            Write-Host "  [MISSING] $Name (required)" -ForegroundColor Red
        } else {
            Write-Host "  [--] $Name (optional)" -ForegroundColor Yellow
        }
        if ($InstallHint) {
            Write-Host "           -> $InstallHint" -ForegroundColor DarkGray
        }
        return (-not $Required)
    }
}

function Test-AllDependencies {
    Write-Host "Dependency Check:" -ForegroundColor White
    Write-Host ""

    $ok = $true

    if (-not (Test-Dependency "git" $true "winget install Git.Git")) {
        $ok = $false
    }

    Test-Dependency "lazygit" $false "winget install JesseDuffield.lazygit" | Out-Null
    Test-Dependency "claude" $false "https://docs.anthropic.com/en/docs/claude-code" | Out-Null

    if ($Local) {
        if (-not (Test-Dependency "cargo" $true "https://rustup.rs")) {
            $ok = $false
        }
    }

    # Windows Terminal check
    Write-Host ""
    if ($env:WT_SESSION) {
        Write-Host "  [OK] Windows Terminal detected" -ForegroundColor Green
    } else {
        Write-Host "  [--] Windows Terminal recommended for best experience" -ForegroundColor Yellow
        Write-Host "           -> https://aka.ms/terminal" -ForegroundColor DarkGray
    }

    Write-Host ""

    if (-not $ok) {
        Write-Host "Missing required dependencies. Please install them first." -ForegroundColor Red
        return $false
    }

    Write-Host "All required dependencies are available." -ForegroundColor Green
    Write-Host ""
    return $true
}

# --- Download & Install ---

function Install-FromRelease {
    $url = "$GitHubUrl/releases/latest/download/$script:AssetName"
    $tmpDir = Join-Path $env:TEMP "cwb-install-$(Get-Random)"
    New-Item -ItemType Directory -Path $tmpDir -Force | Out-Null

    try {
        Write-Step 1 3 "Downloading latest release..."
        Write-Host "  $url" -ForegroundColor DarkGray

        $archivePath = Join-Path $tmpDir $script:AssetName
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        Invoke-WebRequest -Uri $url -OutFile $archivePath -UseBasicParsing

        Write-Host "  Downloaded" -ForegroundColor Green
        Write-Host ""

        Write-Step 2 3 "Extracting archive..."
        Expand-Archive -Path $archivePath -DestinationPath $tmpDir -Force

        # Find binary
        $binary = Get-ChildItem -Path $tmpDir -Filter $BinaryName -Recurse | Select-Object -First 1
        if (-not $binary) {
            Write-Host "Error: Binary not found in archive" -ForegroundColor Red
            exit 1
        }

        Write-Host "  Extracted" -ForegroundColor Green
        Write-Host ""

        Install-Binary $binary.FullName
    } finally {
        Remove-Item -Path $tmpDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Build-Local {
    $projectDir = $null

    if (Test-Path "Cargo.toml") {
        $projectDir = Get-Location
    } elseif (Test-Path (Join-Path $PSScriptRoot "..\Cargo.toml")) {
        $projectDir = Resolve-Path (Join-Path $PSScriptRoot "..")
    } else {
        Write-Host "Error: Cargo.toml not found. Run -Local from the project directory." -ForegroundColor Red
        exit 1
    }

    Write-Step 1 3 "Building release version..."
    Write-Host "  cargo build --release" -ForegroundColor DarkGray
    Write-Host ""

    Push-Location $projectDir
    try {
        & cargo build --release
        if ($LASTEXITCODE -ne 0) {
            Write-Host "Error: Build failed" -ForegroundColor Red
            exit 1
        }
    } finally {
        Pop-Location
    }

    $binary = Join-Path $projectDir "target\release\$BinaryName"
    if (-not (Test-Path $binary)) {
        Write-Host "Error: Build failed -- binary not found at $binary" -ForegroundColor Red
        exit 1
    }

    Write-Host ""
    Write-Host "  Build successful" -ForegroundColor Green
    Write-Host ""

    Install-Binary $binary
}

function Install-Binary {
    param([string]$Source)

    $stepNum = if ($Local) { 2 } else { 3 }
    Write-Step $stepNum 3 "Installing binary..."

    if (-not (Test-Path $InstallDir)) {
        Write-Host "  Creating directory: $InstallDir"
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }

    $dest = Join-Path $InstallDir $BinaryName
    Copy-Item -Path $Source -Destination $dest -Force

    Write-Host "  Installed to $dest" -ForegroundColor Green
    Write-Host ""
}

# --- PATH Management ---

function Add-ToPath {
    $currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")

    if ($currentPath -split ";" | Where-Object { $_ -eq $InstallDir }) {
        return
    }

    Write-Host "Note: $InstallDir is not in your PATH" -ForegroundColor Yellow
    Write-Host ""

    $answer = Read-Host "  Add to user PATH? (y/N)"
    if ($answer -eq "y" -or $answer -eq "Y") {
        $newPath = "$InstallDir;$currentPath"
        [Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
        Write-Host "  PATH updated. Restart your terminal for changes to take effect." -ForegroundColor Green
    } else {
        Write-Host "  Skipped. You can add it manually:" -ForegroundColor DarkGray
        Write-Host "  `$env:PATH = `"$InstallDir;`$env:PATH`"" -ForegroundColor Cyan
    }
    Write-Host ""
}

# --- Completion ---

function Write-Completion {
    $binaryPath = Join-Path $InstallDir $BinaryName
    $size = (Get-Item $binaryPath).Length
    $sizeMB = [math]::Round($size / 1MB, 1)

    $ver = "unknown"
    try {
        $ver = & $binaryPath --version 2>$null | Select-Object -First 1
        if (-not $ver) { $ver = "unknown" }
    } catch {}

    Write-Host "+============================================================+" -ForegroundColor Blue
    Write-Host "|                  Installation Complete                     |" -ForegroundColor Blue
    Write-Host "+============================================================+" -ForegroundColor Blue
    Write-Row "Binary:    $BinaryName"
    Write-Row "Version:   $ver"
    Write-Row "Size:      ${sizeMB}MB"
    Write-Row "Location:  $binaryPath"
    Write-Host "+============================================================+" -ForegroundColor Blue
    Write-Host "|  Run with:  claude-workbench                               |" -ForegroundColor Blue
    Write-Host "+============================================================+" -ForegroundColor Blue
    Write-Host ""
}

# --- Main ---

if ($Help) {
    Write-HelpMessage
    exit 0
}

if ($Version) {
    Write-Host "install.ps1 version $ScriptVersion"
    exit 0
}

Write-Banner
Get-Platform

if (-not (Test-AllDependencies)) {
    exit 1
}

if ($Check) {
    Write-Host "Dependency check complete. Use without -Check to install."
    exit 0
}

if ($Local) {
    Build-Local
} else {
    Install-FromRelease
}

Add-ToPath
Write-Completion
