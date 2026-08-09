#requires -Version 5.1
<#
.SYNOPSIS
    Installs helix-trainer from a pre-built GitHub release binary.

.DESCRIPTION
    Downloads the release archive matching the current CPU architecture,
    verifies its SHA-256 checksum, and installs helix-trainer.exe.

.PARAMETER Version
    Release to install, e.g. "0.5.12". Defaults to the latest release.

.PARAMETER InstallDir
    Install directory. Defaults to "$env:LOCALAPPDATA\helix-trainer\bin".

.PARAMETER NoPathUpdate
    Skip adding the install directory to the user PATH.

.EXAMPLE
    irm https://raw.githubusercontent.com/bug-ops/helix-trainer/main/install.ps1 | iex
#>
param(
    [string]$Version = "latest",
    [string]$InstallDir = "$env:LOCALAPPDATA\helix-trainer\bin",
    [switch]$NoPathUpdate
)

$ErrorActionPreference = "Stop"
$Repo = "bug-ops/helix-trainer"
$Binary = "helix-trainer.exe"

function Get-Arch {
    switch ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture) {
        "X64" { return "x86_64" }
        "Arm64" { return "aarch64" }
        default { throw "Unsupported architecture: $_" }
    }
}

$Arch = Get-Arch
$Target = "$Arch-pc-windows-msvc"

if ($Version -eq "latest") {
    Write-Host "Resolving latest release..."
    $Release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
    $Version = $Release.tag_name -replace '^v', ''
}

$Archive = "helix-trainer-v$Version-$Target.zip"
$BaseUrl = "https://github.com/$Repo/releases/download/v$Version"

$TmpDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.IO.Path]::GetRandomFileName())
New-Item -ItemType Directory -Path $TmpDir | Out-Null

try {
    $ArchivePath = Join-Path $TmpDir $Archive
    $ChecksumPath = "$ArchivePath.sha256"

    Write-Host "Downloading $Archive..."
    Invoke-WebRequest -Uri "$BaseUrl/$Archive" -OutFile $ArchivePath
    Invoke-WebRequest -Uri "$BaseUrl/$Archive.sha256" -OutFile $ChecksumPath

    Write-Host "Verifying checksum..."
    $Expected = (Get-Content $ChecksumPath) -split '\s+' | Select-Object -First 1
    $Actual = (Get-FileHash -Path $ArchivePath -Algorithm SHA256).Hash.ToLower()
    if ($Expected.ToLower() -ne $Actual) {
        throw "Checksum mismatch: expected $Expected, got $Actual"
    }

    Write-Host "Extracting..."
    Expand-Archive -Path $ArchivePath -DestinationPath $TmpDir -Force

    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    $SourceExe = Join-Path $TmpDir "helix-trainer-v$Version-$Target\$Binary"
    Copy-Item -Path $SourceExe -Destination (Join-Path $InstallDir $Binary) -Force

    Write-Host "Installed $Binary $Version to $InstallDir\$Binary"

    if (-not $NoPathUpdate) {
        $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
        if ($UserPath -notlike "*$InstallDir*") {
            [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
            Write-Host "Added $InstallDir to your user PATH (restart your terminal to pick it up)."
        }
    }
    else {
        Write-Host ""
        Write-Host "Add $InstallDir to your PATH to run helix-trainer from anywhere."
    }
}
finally {
    Remove-Item -Path $TmpDir -Recurse -Force -ErrorAction SilentlyContinue
}
