#!/bin/sh
# P9 + P10 end to end. Run from anywhere; needs cargo and cyrius 6.7.18.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
rust="$here/../rust"
mkdir -p "$here/out/p9/rust" "$here/out/p9/cyr" "$here/out/p10/image" "$here/out/p10/chitra"

# P9: fixtures + soorat 1.2.0's view (Rust), then the bayan prototype (Cyrius), then compare.
(cd "$rust" && cargo run --release --locked --quiet --bin p9_glb)
(cd "$here" && cyrius deps >/dev/null && cyrius test --bounds tests/p9_glb.tcyr && cyrius test --bounds tests/p9_mem.tcyr)
echo "== P9 dumps that differ (Rust 1.2.0, overflow-checks on, vs the Cyrius prototype):"
for f in "$here"/out/p9/rust/*.txt; do
    b=$(basename "$f")
    grep -v '^#' "$f" | diff -q - "$here/out/p9/cyr/$b" >/dev/null 2>&1 || \
    { grep -v '^#' "$f" > "$here/out/p9/.r"; grep -v '^#' "$here/out/p9/cyr/$b" > "$here/out/p9/.c";
      diff -q "$here/out/p9/.r" "$here/out/p9/.c" >/dev/null || echo "  $b"; }
done
rm -f "$here/out/p9/.r" "$here/out/p9/.c"
(cd "$here" && python3 -I p9_floats.py)

# P10: fixtures + image's decode (Rust), chitra's decode (Cyrius), then compare.
(cd "$rust" && cargo run --release --locked --quiet --bin p10_codecs)
(cd "$here" && cyrius test --bounds tests/p10_decode.tcyr)
(cd "$rust" && cargo run --release --locked --quiet --bin p10_compare)
