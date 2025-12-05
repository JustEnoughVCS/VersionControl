# Setup documents repo
if (Test-Path "docs/Documents/.git") {
    Push-Location "docs/Documents"
    git pull origin main
    Pop-Location
} else {
    git clone https://github.com/JustEnoughVCS/Documents.git docs/Documents
}
