# Setup documents repo
if (Test-Path "docs/Documents/.git") {
    Push-Location "docs/Documents"
    git fetch origin main
    git reset --hard origin/main
    Pop-Location
} else {
    git clone https://github.com/JustEnoughVCS/Documents.git docs/Documents
}

# Hide .cargo, .github and .temp directories before build
if (Test-Path .cargo) {
    attrib +h .cargo
}
if (Test-Path .github) {
    attrib +h .github
}
if (Test-Path .temp) {
    attrib +h .temp
}
