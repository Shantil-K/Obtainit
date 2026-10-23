#!/usr/bin/env bash
# Cut a release: bump the version, commit, tag and push. GitHub Actions then
# builds the installers and publishes the release, and installed copies of
# Obtainit update themselves.
#   scripts/release.sh [patch|minor|major|1.2.3]
set -euo pipefail
cd "$(dirname "$0")/.."
git diff --quiet || { echo "Commit or stash your changes first." >&2; exit 1; }
v=$(node scripts/bump-version.mjs "${1:-patch}")
cargo update -p obtainit -p lifter-core --quiet 2>/dev/null || true
git commit -am "Release v$v"
git tag "v$v"
git push origin HEAD "v$v"
echo "Pushed v$v — watch the build at https://github.com/Shantil-K/Obtainit/actions"
