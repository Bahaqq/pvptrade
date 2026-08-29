$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
    throw "GitHub CLI is required to request the cloud Anchor build. Install GitHub CLI, then rerun this script."
}

$RepositoryRoot = Split-Path -Parent $PSScriptRoot
Push-Location $RepositoryRoot

try {
    gh workflow run anchor-build.yml
    Write-Host "Anchor cloud build requested. Run 'gh run list --workflow anchor-build.yml' to inspect it." -ForegroundColor Green
}
finally {
    Pop-Location
}
