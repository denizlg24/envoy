# Envoy Windows Installer
# Usage: iwr -useb https://raw.githubusercontent.com/denizlg24/envoy/master/install.ps1 | iex

$ErrorActionPreference = 'Stop'


$repoOwner = "denizlg24"
$repoName = "envoy"
$binaryName = "envy.exe"
$installDir = "$env:LOCALAPPDATA\envoy"
$binaryPath = "$installDir\$binaryName"

Write-Host "Installing Envoy CLI..." -ForegroundColor Cyan

if (!(Test-Path $installDir)) {
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
}

try {
    Write-Host "Fetching latest release..." -ForegroundColor Yellow
    $latestRelease = Invoke-RestMethod -Uri "https://api.github.com/repos/$repoOwner/$repoName/releases/latest"
    $downloadUrl = $latestRelease.assets | Where-Object { $_.name -eq $binaryName } | Select-Object -ExpandProperty browser_download_url
    
    if (!$downloadUrl) {
        throw "No Windows binary found in latest release"
    }
} catch {
    Write-Host "No release found, downloading from master branch..." -ForegroundColor Yellow
    $downloadUrl = "https://github.com/$repoOwner/$repoName/releases/download/latest/$binaryName"
}


try {
    Write-Host "Downloading from: $downloadUrl" -ForegroundColor Yellow
    Invoke-WebRequest -Uri $downloadUrl -OutFile $binaryPath
    Write-Host "Downloaded to: $binaryPath" -ForegroundColor Green
} catch {
    Write-Host "Error downloading binary: $_" -ForegroundColor Red
    Write-Host ""
    Write-Host "Alternative: Build from source:" -ForegroundColor Yellow
    Write-Host "  1. Install Rust from https://rustup.rs/" -ForegroundColor White
    Write-Host "  2. Clone repo: git clone https://github.com/$repoOwner/$repoName" -ForegroundColor White
    Write-Host "  3. Build: cargo build --release" -ForegroundColor White
    exit 1
}


$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$installDir*") {
    Write-Host "Adding to PATH..." -ForegroundColor Yellow
    [Environment]::SetEnvironmentVariable(
        "Path",
        "$userPath;$installDir",
        "User"
    )
    $env:Path = "$env:Path;$installDir"
    Write-Host "Added to PATH" -ForegroundColor Green
} else {
    Write-Host "Already in PATH" -ForegroundColor Green
}


Write-Host ""
Write-Host "Installation complete!" -ForegroundColor Green
Write-Host ""
Write-Host "To get started:" -ForegroundColor Cyan
Write-Host "  envy --help" -ForegroundColor White
Write-Host ""
Write-Host "Note: You may need to restart your terminal for PATH changes to take effect." -ForegroundColor Yellow
