$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$repoOwner = "denizlg24"
$repoName = "envoy"
$binaryName = "envy.exe"  # The final name you want for the command
$installDir = "$env:LOCALAPPDATA\envoy"
$binaryPath = "$installDir\$binaryName"
$zipPath = "$installDir\temp_install.zip"

Write-Host "Installing Envoy CLI..." -ForegroundColor Cyan

# Create Install Directory
if (!(Test-Path $installDir)) {
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
}

try {
    Write-Host "Fetching latest release..." -ForegroundColor Yellow
    
    # Get all releases and pick the first one (handles Pre-releases better)
    $allReleases = Invoke-RestMethod -Uri "https://api.github.com/repos/$repoOwner/$repoName/releases"
    $latestRelease = $allReleases | Select-Object -First 1
    
    if (!$latestRelease) { throw "No releases found on GitHub." }

    Write-Host "Found release: $($latestRelease.name)" -ForegroundColor Cyan

    # Find the Windows Zip asset (matches "windows-msvc.zip")
    $asset = $latestRelease.assets | Where-Object { $_.name -like "*windows-msvc.zip" } | Select-Object -First 1

    if (!$asset) {
        throw "Could not find a Windows zip file in the latest release."
    }

    $downloadUrl = $asset.browser_download_url
} catch {
    Write-Host "Error fetching release info: $_" -ForegroundColor Red
    exit 1
}

try {
    Write-Host "Downloading $downloadUrl..." -ForegroundColor Yellow
    Invoke-WebRequest -Uri $downloadUrl -OutFile $zipPath
    
    Write-Host "Extracting..." -ForegroundColor Yellow
    # Extract to a temporary folder inside install dir
    $tempExtractPath = "$installDir\temp_extract"
    if (Test-Path $tempExtractPath) { Remove-Item $tempExtractPath -Recurse -Force }
    
    Expand-Archive -Path $zipPath -DestinationPath $tempExtractPath -Force
    
    # Find any .exe inside the extracted folder
    $extractedExe = Get-ChildItem -Path $tempExtractPath -Recurse -Filter "*.exe" | Select-Object -First 1
    
    if (!$extractedExe) {
        throw "No .exe file found inside the downloaded zip."
    }

    # Move and Rename the binary to the final location
    Move-Item -Path $extractedExe.FullName -Destination $binaryPath -Force
    
    # Cleanup temp files
    Remove-Item $zipPath -Force
    Remove-Item $tempExtractPath -Recurse -Force

    Write-Host "Installed to: $binaryPath" -ForegroundColor Green

} catch {
    Write-Host "Error installing: $_" -ForegroundColor Red
    if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
    exit 1
}

# Add to PATH
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$installDir*") {
    Write-Host "Adding to PATH..." -ForegroundColor Yellow
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$installDir", "User")
    $env:Path = "$env:Path;$installDir"
    Write-Host "Added to PATH" -ForegroundColor Green
} else {
    Write-Host "Already in PATH" -ForegroundColor Green
}

Write-Host ""
Write-Host "Installation complete!" -ForegroundColor Green
Write-Host "Type 'envy --help' to start." -ForegroundColor White
