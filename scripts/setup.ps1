$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$RepositoryRoot = Split-Path -Parent $PSScriptRoot
Push-Location $RepositoryRoot

try {
    $NodeVersionText = (& node --version).TrimStart("v")
    $NodeMajor = [int]($NodeVersionText.Split(".")[0])

    if ($NodeMajor -lt 24) {
        throw "Node.js 24 or newer is required. Installed version: $NodeVersionText"
    }

    Write-Host "Installing PVP Trade dependencies with npm..." -ForegroundColor Cyan
    npm install
    Write-Host "Setup complete." -ForegroundColor Green
}
finally {
    Pop-Location
}
