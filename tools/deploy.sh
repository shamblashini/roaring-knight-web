#!/usr/bin/env bash
# Build the release bundle and publish it to the gh-pages branch (GitHub Pages).
# The built site includes the extracted game assets; the main branch never does.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO_NAME="${REPO_NAME:-roaring-knight-web}"
cd "$ROOT"
python3 tools/pack_assets.py
trunk build --release --dist "$ROOT/release" --public-url "/$REPO_NAME/"
touch "$ROOT/release/.nojekyll"

TMP="$(mktemp -d)"
trap 'git -C "$ROOT" worktree remove --force "$TMP" >/dev/null 2>&1 || true; rm -rf "$TMP"' EXIT
if git ls-remote --exit-code --heads origin gh-pages >/dev/null 2>&1; then
    git fetch -q origin gh-pages
    git worktree add -q "$TMP" -B gh-pages origin/gh-pages
else
    git worktree add -q --detach "$TMP"
    git -C "$TMP" checkout -q --orphan gh-pages
    git -C "$TMP" rm -rq --cached . >/dev/null 2>&1 || true
fi
find "$TMP" -mindepth 1 -maxdepth 1 ! -name .git -exec rm -rf {} +
cp -a "$ROOT/release/." "$TMP/"
git -C "$TMP" add -A
if git -C "$TMP" diff --cached --quiet; then
    echo "nothing to deploy"
    exit 0
fi
git -C "$TMP" -c commit.gpgsign=false commit -qm "Deploy $(git -C "$ROOT" rev-parse --short HEAD)"
git -C "$TMP" push -q origin gh-pages
echo "deployed to gh-pages"
