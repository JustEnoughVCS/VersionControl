# Setup documents repo
if [ -d "docs/Documents/.git" ]; then
    cd docs/Documents
    git fetch origin main
    git reset --hard origin/main
    cd -
else
    git clone https://github.com/JustEnoughVCS/Documents.git docs/Documents
fi
