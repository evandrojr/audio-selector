# Usage: .\release.ps1 1.0.0

param (
    [Parameter(Mandatory=$true)]
    [string]$Version
)

$TagVersion = "v$Version"

Write-Host "Releasing version $TagVersion..."

# Check branch
$branch = git rev-parse --abbrev-ref HEAD
if ($branch -ne "main") {
    Write-Error "Error: You must be on the main branch to release."
    exit 1
}

# Tag and push
git tag -a "$TagVersion" -m "Release $TagVersion"
git push origin main
git push origin "$TagVersion"

Write-Host "Tag $TagVersion pushed. GitHub Actions will now build and create the release."
