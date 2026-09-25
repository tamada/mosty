#! /bin/sh
#
# Updates the version of mosty in Cargo.toml, Cargo.lock, and the badges.
# It is run by the update-version workflow on `releases/vX.Y.Z` branches.
#
#     sh .github/scripts/update_version.sh 0.2.0

set -eu

# The paths below are relative to the repository root.
cd "$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"

command -v cargo > /dev/null || {
    echo "$0: cargo is not on PATH, and Cargo.lock cannot be brought along without it" >&2
    exit 1
}

usage() {
    echo "usage: $0 <version>        e.g. $0 0.4.0" >&2
    echo "  the leading v of a tag name is accepted and ignored" >&2
    exit 1
}

[ $# -eq 1 ] || usage

# The version reaches the right-hand side of sed, so only X.Y.Z is accepted.
TO_VERSION=$(printf '%s' "$1" | sed -E 's/^v//')
case $TO_VERSION in
    *[!0-9.]* | *..* | .* | *. ) usage ;;
esac
echo "$TO_VERSION" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' || usage

V='[0-9]+\.[0-9]+\.[0-9]+'
TMP=

# Removes the temporary file of rewrite if the script fails.
cleanup() { [ -z "$TMP" ] || rm -f "$TMP"; }
trap cleanup EXIT

rewrite() {
    file=$1
    shift
    TMP="$file.tmp"
    sed -E "$@" "$file" > "$TMP"
    mv "$TMP" "$file"
    TMP=
}

# The patterns match the shape of versions after fixed prefixes, not the old
# version itself, so that other numbers are never rewritten.
rewrite Cargo.toml -e "s/^version = \".*\"/version = \"${TO_VERSION}\"/"

# docs/ does not exist until the document site is added.
for f in README.md docs/content/_index.md; do
    [ -f "$f" ] || continue
    rewrite "$f" \
        -e "s|(badge/Version-)${V}|\1${TO_VERSION}|g" \
        -e "s|(releases/tag/v)${V}|\1${TO_VERSION}|g" \
        -e "s|(mosty:)${V}|\1${TO_VERSION}|g"
done

# The container builds with `cargo build --locked`, which fails if Cargo.lock
# has the old version. `--workspace` updates only mosty itself.
cargo update --workspace
