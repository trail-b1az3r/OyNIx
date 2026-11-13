#!/usr/bin/env bash
# Build the .deb and pacman packages for VERSION and publish them as a
# GitHub release "vVERSION" (needs the GitHub CLI, logged in: `gh auth login`).
#
#   scripts/release.sh 5.1.0                 build + publish
#   scripts/release.sh 5.1.0 --draft         publish as a draft
#   scripts/release.sh 5.1.0-beta.1          pre-release (any version with '-')
#   scripts/release.sh 5.1.0 --dry-run       build only, show what would be published
#
# Without a version nothing is released: use scripts/build-deb.sh / build-pkg.sh.
set -euo pipefail
cd "$(dirname "$0")/.."
VERSION="${1:-}"; shift || true
[ -n "$VERSION" ] || { sed -n '2,10p' "$0"; exit 1; }
VERSION="${VERSION#v}"
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-+.][0-9A-Za-z.-]+)?$ ]] || { echo "Version '$VERSION' is not like 1.2.3 or 1.2.3-beta.1" >&2; exit 1; }
DRAFT=""; DRY=0
for a in "$@"; do case "$a" in --draft) DRAFT="--draft" ;; --dry-run) DRY=1 ;; *) echo "unknown option $a" >&2; exit 1 ;; esac; done
PRE=""; [[ "$VERSION" == *-* ]] && PRE="--prerelease"
TAG="v$VERSION"

if [ "$DRY" = 0 ]; then
  command -v gh >/dev/null || { echo "Install the GitHub CLI (gh) to publish releases" >&2; exit 1; }
  gh auth status >/dev/null 2>&1 || { echo "Run 'gh auth login' first" >&2; exit 1; }
  [ -z "$(git status --porcelain)" ] || { echo "Commit or stash your changes first" >&2; exit 1; }
  git fetch -q origin
  [ "$(git rev-parse HEAD)" = "$(git rev-parse '@{u}' 2>/dev/null || echo none)" ] || { echo "Push your branch first (the release points at the pushed commit)" >&2; exit 1; }
  ! gh release view "$TAG" >/dev/null 2>&1 || { echo "Release $TAG already exists" >&2; exit 1; }
fi

scripts/build-deb.sh "$VERSION"
scripts/build-pkg.sh "$VERSION"
rm -rf dist && mkdir dist
cp target/deb/oynix_"${VERSION}"_*.deb target/pkg/oynix-"${VERSION//-/_}"-*.pkg.tar.zst dist/
(cd dist && sha256sum * > SHA256SUMS)

NOTES=$(cat <<NOTES
## Install

**Debian / Ubuntu / Mint**: \`sudo apt install ./$(basename dist/*.deb)\`

**Arch / Manjaro / EndeavourOS**: \`sudo pacman -U $(basename dist/*.pkg.tar.zst)\`

Both packages include their own Qt runtime under \`/usr/lib/oynix\`. Verify with \`sha256sum -c SHA256SUMS\`.
NOTES
)

if [ "$DRY" = 1 ]; then
  echo; echo "Dry run — would publish $TAG ${PRE:+(pre-release)} ${DRAFT:+(draft)} with:"; ls -lh dist
  exit 0
fi
gh release create "$TAG" dist/* --target "$(git rev-parse HEAD)" --title "OyNIx $VERSION" \
  --notes "$NOTES" --generate-notes $PRE $DRAFT
echo "Published $(gh release view "$TAG" --json url -q .url)"
