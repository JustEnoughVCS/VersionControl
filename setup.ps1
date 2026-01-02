# Setup documents repo
if (Test-Path "docs/Documents/.git") {
    Push-Location "docs/Documents"
    git fetch origin main
    git reset --hard origin/main
    Pop-Location
} else {
    git clone https://github.com/JustEnoughVCS/Documents.git docs/Documents
}
