#!/usr/bin/env bash
set -euo pipefail

# Bump the version. VERSION is the single source of truth: cyrius.cyml reads it
# through `version = "${file:VERSION}"`. The Rust crate in rust-old/ is the frozen
# 1.2.0 oracle and is never bumped.
NEW_VERSION="${1:?Usage: $0 <new-version>}"

echo "$NEW_VERSION" > VERSION
grep -q '^version = "${file:VERSION}"' cyrius.cyml || {
    echo "cyrius.cyml does not read VERSION; fix its [package] version" >&2
    exit 1
}
echo "Bumped to $NEW_VERSION"
