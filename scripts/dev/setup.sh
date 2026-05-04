# Change to the directory where the script is located
cd "$(dirname "$0")/../../" || exit 1

# Setup documents repo
if [ -d "docs/Documents/.git" ]; then
    cd docs/Documents
    git fetch origin main
    git reset --hard origin/main
    cd -
else
    git clone https://github.com/JustEnoughVCS/Documents.git docs/Documents
fi
