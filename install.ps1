# MediathekViewWeb CLI Installation Script for Windows
# Automatically detects architecture and installs the appropriate binary

[CmdletBinding()]
param(
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\mwb",
    [string]$Repository = "your-username/mwb",  # Replace with actual repository
    [switch]$AddToPath,
    [switch]$Force,
    [switch]$Help,
    [switch]$Version,
    [switch]$Uninstall
)

# Configuration
$BinaryName = "mwb.exe"
$TempDir = "$env:TEMP\mwb-install"

# Colors for output (using Write-Host with colors)
function Write-Status {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor Blue
}

function Write-Success {
    param([string]$Message)
    Write-Host "[SUCCESS] $Message" -ForegroundColor Green
}

function Write-Warning {
    param([string]$Message)
    Write-Host "[WARNING] $Message" -ForegroundColor Yellow
}

function Write-Error {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor Red
}

# Function to show help
function Show-Help {
    @"
MediathekViewWeb CLI Installation Script for Windows

Usage: .\install.ps1 [options]

Options:
  -InstallDir <path>    Installation directory (default: $env:LOCALAPPDATA\Programs\mwb)
  -Repository <repo>    GitHub repository (default: your-username/mwb)
  -AddToPath           Add installation directory to PATH
  -Force               Force reinstallation even if already installed
  -Help                Show this help message
  -Version             Show version information
  -Uninstall           Uninstall MediathekViewWeb CLI

Environment variables:
  MWB_INSTALL_DIR      Override default installation directory
  MWB_REPOSITORY       Override default repository

Examples:
  .\install.ps1                           # Install to default location
  .\install.ps1 -AddToPath                # Install and add to PATH
  .\install.ps1 -InstallDir "C:\Tools"    # Install to custom directory
  .\install.ps1 -Uninstall                # Uninstall

"@
}

# Function to detect architecture
function Get-Architecture {
    $arch = $env:PROCESSOR_ARCHITECTURE
    switch ($arch) {
        "AMD64" { return "x64" }
        "ARM64" { return "arm64" }
        default {
            Write-Error "Unsupported architecture: $arch"
            exit 1
        }
    }
}

# Function to get latest release version
function Get-LatestVersion {
    try {
        $apiUrl = "https://api.github.com/repos/$Repository/releases/latest"
        $response = Invoke-RestMethod -Uri $apiUrl -ErrorAction Stop
        return $response.tag_name
    }
    catch {
        Write-Error "Failed to get latest version: $($_.Exception.Message)"
        exit 1
    }
}

# Function to download file with progress
function Download-File {
    param(
        [string]$Url,
        [string]$OutputPath
    )
    
    try {
        Write-Status "Downloading from: $Url"
        $webClient = New-Object System.Net.WebClient
        $webClient.DownloadFile($Url, $OutputPath)
        $webClient.Dispose()
    }
    catch {
        Write-Error "Failed to download file: $($_.Exception.Message)"
        exit 1
    }
}

# Function to verify checksum
function Test-Checksum {
    param(
        [string]$FilePath,
        [string]$ChecksumFile
    )
    
    if (-not (Test-Path $ChecksumFile)) {
        Write-Warning "Checksum file not found. Skipping verification."
        return $true
    }
    
    try {
        $expectedHash = (Get-Content $ChecksumFile).Split()[0]
        $actualHash = (Get-FileHash $FilePath -Algorithm SHA256).Hash.ToLower()
        
        if ($expectedHash -eq $actualHash) {
            Write-Success "Checksum verification passed"
            return $true
        }
        else {
            Write-Error "Checksum verification failed"
            Write-Error "Expected: $expectedHash"
            Write-Error "Actual:   $actualHash"
            return $false
        }
    }
    catch {
        Write-Warning "Checksum verification failed: $($_.Exception.Message)"
        return $false
    }
}

# Function to add to PATH
function Add-ToPath {
    param([string]$Directory)
    
    $currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
    
    if ($currentPath -notlike "*$Directory*") {
        Write-Status "Adding $Directory to user PATH..."
        $newPath = if ($currentPath) { "$currentPath;$Directory" } else { $Directory }
        [Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
        Write-Success "Added to PATH. Please restart your PowerShell session or run: `$env:PATH += ';$Directory'"
        return $true
    }
    else {
        Write-Status "Directory already in PATH"
        return $false
    }
}

# Function to remove from PATH
function Remove-FromPath {
    param([string]$Directory)
    
    $currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
    
    if ($currentPath -like "*$Directory*") {
        Write-Status "Removing $Directory from user PATH..."
        $newPath = ($currentPath -split ';' | Where-Object { $_ -ne $Directory }) -join ';'
        [Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
        Write-Success "Removed from PATH"
        return $true
    }
    else {
        Write-Status "Directory not found in PATH"
        return $false
    }
}

# Function to test installation
function Test-Installation {
    $binaryPath = Join-Path $InstallDir $BinaryName
    
    if (Test-Path $binaryPath) {
        try {
            $version = & $binaryPath --version 2>$null | Select-Object -First 1
            Write-Success "MediathekViewWeb CLI is installed and accessible"
            Write-Status "Version: $version"
            Write-Status "Location: $binaryPath"
            return $true
        }
        catch {
            Write-Warning "Binary found but not executable"
            return $false
        }
    }
    else {
        Write-Warning "Binary not found at expected location"
        return $false
    }
}

# Function to show usage examples
function Show-Usage {
    @"

🎬 MediathekViewWeb CLI - Usage Examples:

Basic search:
  mwb search "documentary"
  mwb search "tatort" --size 10

VLC integration with quality selection:
  mwb search "news" --vlc          # Medium quality (default)
  mwb search "arte" --vlc=h        # HD quality
  mwb search "zdf" -v=l            # Low quality

Export formats:
  mwb search "science" --format json
  mwb search "culture" --format csv
  mwb search "sports" --format xspf

Advanced search:
  mwb search "documentary >60"     # Longer than 60 minutes
  mwb search "!ARD science"        # ARD channel, science topic
  mwb search "climate" --include "documentary|report"

For more options:
  mwb search --help
  mwb --help

"@
    Write-Host $usage -ForegroundColor Cyan
}

# Function to install binary
function Install-Binary {
    param(
        [string]$Version,
        [string]$Architecture
    )
    
    $platform = "windows-$Architecture"
    $binaryName = "mwb-$platform.exe"
    $downloadUrl = "https://github.com/$Repository/releases/download/$Version/$binaryName"
    $checksumUrl = "https://github.com/$Repository/releases/download/$Version/$binaryName.sha256"
    
    Write-Status "Installing MediathekViewWeb CLI $Version for $platform..."
    
    # Create directories
    if (-not (Test-Path $TempDir)) {
        New-Item -ItemType Directory -Path $TempDir -Force | Out-Null
    }
    
    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }
    
    $tempBinary = Join-Path $TempDir $binaryName
    $tempChecksum = Join-Path $TempDir "$binaryName.sha256"
    $finalBinary = Join-Path $InstallDir $BinaryName
    
    try {
        # Download binary
        Write-Status "Downloading binary..."
        Download-File $downloadUrl $tempBinary
        
        # Download checksum
        Write-Status "Downloading checksum..."
        try {
            Download-File $checksumUrl $tempChecksum
        }
        catch {
            Write-Warning "Could not download checksum file"
        }
        
        # Verify checksum
        if (Test-Path $tempChecksum) {
            Write-Status "Verifying checksum..."
            if (-not (Test-Checksum $tempBinary $tempChecksum)) {
                Write-Error "Checksum verification failed"
                exit 1
            }
        }
        
        # Install binary
        Write-Status "Installing to $InstallDir..."
        Copy-Item $tempBinary $finalBinary -Force
        
        Write-Success "Installation completed successfully!"
        
        # Add to PATH if requested
        if ($AddToPath) {
            Add-ToPath $InstallDir
        }
        
        return $true
    }
    catch {
        Write-Error "Installation failed: $($_.Exception.Message)"
        return $false
    }
    finally {
        # Clean up
        if (Test-Path $TempDir) {
            Remove-Item $TempDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

# Function to uninstall
function Uninstall-Binary {
    $binaryPath = Join-Path $InstallDir $BinaryName
    $removed = $false
    
    if (Test-Path $binaryPath) {
        Write-Status "Removing $binaryPath..."
        Remove-Item $binaryPath -Force
        Write-Success "Binary removed"
        $removed = $true
    }
    
    # Remove from PATH
    if (Remove-FromPath $InstallDir) {
        $removed = $true
    }
    
    # Remove directory if empty
    if ((Test-Path $InstallDir) -and ((Get-ChildItem $InstallDir).Count -eq 0)) {
        Write-Status "Removing empty directory $InstallDir..."
        Remove-Item $InstallDir -Force
    }
    
    if ($removed) {
        Write-Success "MediathekViewWeb CLI uninstalled successfully"
    }
    else {
        Write-Warning "MediathekViewWeb CLI was not found"
    }
}

# Main function
function Main {
    Write-Host "🎬 MediathekViewWeb CLI Installation Script for Windows" -ForegroundColor Magenta
    Write-Host "========================================================" -ForegroundColor Magenta
    Write-Host ""
    
    # Override with environment variables if set
    if ($env:MWB_INSTALL_DIR) {
        $script:InstallDir = $env:MWB_INSTALL_DIR
    }
    if ($env:MWB_REPOSITORY) {
        $script:Repository = $env:MWB_REPOSITORY
    }
    
    # Handle command line arguments
    if ($Help) {
        Show-Help
        return
    }
    
    if ($Uninstall) {
        Uninstall-Binary
        return
    }
    
    if ($Version) {
        try {
            $latestVersion = Get-LatestVersion
            Write-Host "Latest available version: $latestVersion"
            
            $binaryPath = Join-Path $InstallDir $BinaryName
            if (Test-Path $binaryPath) {
                try {
                    $currentVersion = & $binaryPath --version 2>$null | Select-Object -First 1
                    Write-Host "Currently installed: $currentVersion"
                }
                catch {
                    Write-Host "Currently installed: unknown"
                }
            }
            else {
                Write-Host "MediathekViewWeb CLI is not installed"
            }
        }
        catch {
            Write-Error "Failed to get version information"
        }
        return
    }
    
    # Check if already installed
    $binaryPath = Join-Path $InstallDir $BinaryName
    if ((Test-Path $binaryPath) -and -not $Force) {
        try {
            $currentVersion = & $binaryPath --version 2>$null | Select-Object -First 1
            $latestVersion = Get-LatestVersion
            
            if ($currentVersion -like "*$latestVersion*") {
                Write-Success "MediathekViewWeb CLI $latestVersion is already installed"
                if (Test-Installation) {
                    Show-Usage
                }
                return
            }
            else {
                Write-Status "Updating from current version to $latestVersion"
            }
        }
        catch {
            Write-Status "Existing installation found, proceeding with update..."
        }
    }
    
    # Detect architecture
    $architecture = Get-Architecture
    Write-Status "Detected architecture: $architecture"
    
    # Get latest version
    Write-Status "Fetching latest release information..."
    $version = Get-LatestVersion
    Write-Status "Latest version: $version"
    
    # Install
    if (Install-Binary $version $architecture) {
        if (Test-Installation) {
            Show-Usage
        }
        else {
            Write-Warning "Installation may not be complete"
        }
    }
    else {
        Write-Error "Installation failed"
        exit 1
    }
}

# Check PowerShell version
if ($PSVersionTable.PSVersion.Major -lt 5) {
    Write-Error "This script requires PowerShell 5.0 or later"
    exit 1
}

# Check if running as administrator (not required, but warn if needed for system-wide install)
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] "Administrator")

if (-not $isAdmin -and $InstallDir.StartsWith("C:\Program Files")) {
    Write-Warning "Installing to Program Files may require administrator privileges"
    Write-Status "Consider running as administrator or using -InstallDir to specify a user directory"
}

# Run main function
Main