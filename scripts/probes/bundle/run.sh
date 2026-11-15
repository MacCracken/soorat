#!/usr/bin/env bash
# soorat M0 probes P4, P5 and P11 (docs/development/soorat-port-plan.md §4 step 1): every command, in order.
# Logs go to out/. Needs cyrius 6.7.18, git, python3, and the sibling checkouts under ../../../../ (read only).
#
#   scripts/probes/bundle/run.sh            # everything (about 3 minutes on the dev box)
#   scripts/probes/bundle/run.sh p4|p5|p11  # one probe
#
# P4  one hisab:        p4_h350/ p4_h341/ p4_nopin/   (names.py p4 is the static cross-check)
# P5  mabda on 6.7.18:  p5_fold/ p5_dep/
# P11 the full bundle:  p11_full/ (patched/ and control/ are generated or measurement-only; see below)
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
REPOS=$(cd "$HERE/../../../.." && pwd)
OUT="$HERE/out"
mkdir -p "$OUT"
TIMEFORMAT='%R'

# Runs "$@" in the current directory; log to $OUT/$name.log, wall seconds appended as "wall_s=…".
logrun() {
    local name=$1; shift
    local t
    { t=$( { time "$@" > "$OUT/$name.log" 2>&1; } 2>&1 ); } 2>/dev/null
    local rc=$?
    echo "wall_s=$t" >> "$OUT/$name.log"
    printf '%-28s rc=%s wall=%ss\n' "$name" "$rc" "$t"
    grep -E '^(warning|error)|FAIL|passed|exceeds|^OK|^p11:' "$OUT/$name.log" | grep -v '✓' | sed 's/^/    /'
}

p4() {
    echo "== P4: one hisab"
    for d in p4_h350 p4_h341 p4_nopin; do
        cmp -s "$HERE/p4_h350/tests/p4_one_hisab.tcyr" "$HERE/$d/tests/p4_one_hisab.tcyr" || echo "  $d: test file drifted from p4_h350's"
        (cd "$HERE/$d" && cyrius deps > "$OUT/${d}_deps.log" 2>&1; grep '^note: hisab' "$OUT/${d}_deps.log" | sed "s/^/  $d: /")
        (cd "$HERE/$d" && logrun "${d}" cyrius test --bounds tests/p4_one_hisab.tcyr)
        (cd "$HERE/$d" && logrun "${d}_bvh" cyrius test --bounds tests/p4_goonj_bvh.tcyr)
    done
    # Static cross-check against the bundles at their tags (git show; nothing is checked out).
    local tmp
    tmp=$(mktemp -d)
    git -C "$REPOS/hisab" show 3.4.1:dist/hisab.cyr > "$tmp/hisab-3.4.1.cyr"
    git -C "$REPOS/hisab" show 3.5.0:dist/hisab.cyr > "$tmp/hisab-3.5.0.cyr"
    git -C "$REPOS/ushma" show 2.0.0:dist/ushma.cyr > "$tmp/ushma.cyr"
    git -C "$REPOS/prakash" show 2.15.2:dist/prakash.cyr > "$tmp/prakash.cyr"
    git -C "$REPOS/prakash" show 2.15.2:dist/prakash-fft.cyr > "$tmp/prakash-fft.cyr"
    git -C "$REPOS/pravash" show 2.0.1:dist/pravash.cyr > "$tmp/pravash.cyr"
    git -C "$REPOS/pravash" show 2.0.1:src/compute_ad.cyr > "$tmp/pravash-compute_ad.cyr"
    git -C "$REPOS/impetus" show 2.0.0:dist/impetus.cyr > "$tmp/impetus.cyr"
    git -C "$REPOS/impetus" show 2.0.0:dist/impetus-3d.cyr > "$tmp/impetus-3d.cyr"
    git -C "$REPOS/goonj" show 2.3.2:dist/goonj.cyr > "$tmp/goonj.cyr"
    (cd "$tmp" && python3 -I "$HERE/names.py" p4 hisab-3.4.1.cyr hisab-3.5.0.cyr ushma.cyr prakash.cyr prakash-fft.cyr \
        pravash.cyr pravash-compute_ad.cyr impetus.cyr impetus-3d.cyr goonj.cyr) > "$OUT/p4_static.log"
    rm -rf "$tmp"
    echo "  static scan: out/p4_static.log"
    grep -E '^fns in old|uses a (fn removed|name only)' "$OUT/p4_static.log" | sort | uniq -c | sed 's/^/    /'
}

p5() {
    echo "== P5: mabda on 6.7.18"
    local fold="$HOME/.cyrius/versions/6.7.18/lib/mabda.cyr"
    if git -C "$REPOS/mabda" show 4.2.0:dist/mabda.cyr | cmp -s - "$fold"; then
        echo "  stdlib fold lib/mabda.cyr == mabda tag 4.2.0 dist/mabda.cyr ($(sha256sum "$fold" | cut -c1-16)…, $(wc -c < "$fold") B)"
    else
        echo "  stdlib fold lib/mabda.cyr DIFFERS from mabda tag 4.2.0 dist/mabda.cyr"
    fi
    sed -n 2p "$fold" | sed 's/^/  fold header: /'
    for d in p5_fold p5_dep; do
        cmp -s "$HERE/p5_fold/tests/p5_mabda.tcyr" "$HERE/$d/tests/p5_mabda.tcyr" || echo "  $d: test file drifted"
        (cd "$HERE/$d" && cyrius deps > "$OUT/${d}_deps.log" 2>&1)
        for i in 1 2 3; do (cd "$HERE/$d" && logrun "${d}_run$i" cyrius -v test --bounds tests/p5_mabda.tcyr); done
        grep -h 'binary:' "$OUT/${d}_run1.log" | sed "s/^/  $d: /"
        local pad
        pad=$(cd "$HERE/$d" && headroom tests/p5_mabda.tcyr --bounds)
        echo "  $d: 24 MiB cap: largest pad that still preprocesses = $pad B -> expanded unit ~ $((25165824 - pad)) B (±4096)" | tee "$OUT/${d}_headroom.log"
    done
}

# A test unit's expanded size as the 24 MiB cap sees it: the largest comment pad that still preprocesses,
# found by bisection to 4 KiB (a pad past the cap stops at "expanded source exceeds 24MB"). size = 25165824 - pad.
# Run from the project directory; uses build/.
headroom() {
    local unit=$1 lo=0 hi=25165824 mid
    mkdir -p build
    while (( hi - lo > 4096 )); do
        mid=$(( (lo + hi) / 2 ))
        python3 -I -c "import sys; q, r = divmod(int(sys.argv[1]), 100); sys.stdout.write(('#' + 'p' * 98 + '\n') * q + (('#' + 'p' * (r - 2) + '\n') if r >= 2 else '\n' * r))" "$mid" > build/pad.cyr
        { echo 'include "build/pad.cyr"'; cat "$unit"; } > build/p11_pad.tcyr
        if cyrius test $2 build/p11_pad.tcyr 2>&1 | grep -q 'expanded source exceeds'; then hi=$mid; else lo=$mid; fi
    done
    rm -f build/pad.cyr build/p11_pad.tcyr
    echo "$lo"
}

p11() {
    echo "== P11: the full bundle"
    cd "$HERE/p11_full" || return 1
    mkdir -p build patched
    cyrius -v deps > "$OUT/p11_deps.log" 2>&1
    grep -E '^note|tag ' "$OUT/p11_deps.log" | sed 's/^/  /'
    # As declared: the test compile and the production build.
    logrun p11_test cyrius -v test --bounds tests/p11_full.tcyr
    logrun p11_build_asis env CYRIUS_DCE=1 cyrius build src/main.cyr build/p11_asis
    # Measurement-only unit: the same prepends, in the same order (from the -v listing), with ranga's two
    # arity-conflicting fns renamed rg_* in a copy. It is what lets the rest of P11 (run, sizes) be measured.
    sed -E 's/\b(un)?premultiply_alpha\b/rg_\1premultiply_alpha/g' lib/ranga.cyr > patched/ranga.cyr
    gen() {
        awk -v test="$2" '
          /^  stdlib: /            { print "include \"lib/" $2 ".cyr\""; next }
          /^  stdlib \(test\): /   { if (test == 1) print "include \"lib/" $3 ".cyr\""; next }
          /^  [a-z_-]+: [a-z_-]+\.cyr$/ { f = $2; if (f == "ranga.cyr") print "include \"patched/ranga.cyr\""; else print "include \"lib/" f "\""; next }
        ' "$1"
    }
    {
        echo "# GENERATED by ../run.sh from the -v prepend listing of tests/p11_full.tcyr; built with --no-deps."
        echo "# The P11 unit with ranga's premultiply_alpha / unpremultiply_alpha renamed rg_* (patched/ranga.cyr), the"
        echo "# one change that lets it compile. A measurement artifact, not a proposed fix."
        gen "$OUT/p11_test.log" 1
        sed -n '/^include "src/,$p' tests/p11_full.tcyr | sed 's/test_group("p11: /test_group("p11 patched: /'
    } > patched/p11_patched.tcyr
    {
        echo "# GENERATED by ../run.sh (see patched/p11_patched.tcyr); production scope: no assert / bench."
        gen "$OUT/p11_test.log" 0
        sed -n '/^include "src/,$p' src/main.cyr
    } > patched/main_patched.cyr
    for i in 1 2 3; do logrun "p11_patched_test_run$i" cyrius test --no-deps --bounds patched/p11_patched.tcyr; done
    for i in 1 2 3; do logrun "p11_patched_nodce_run$i" env CYRIUS_DCE=0 cyrius build --no-deps patched/main_patched.cyr build/p11_nodce; done
    for i in 1 2 3; do logrun "p11_patched_dce_run$i" env CYRIUS_DCE=1 cyrius build --no-deps patched/main_patched.cyr build/p11_dce; done
    ls -l build/p11_nodce build/p11_dce | awk '{print "  " $5 " B  " $9}'
    ./build/p11_dce | sed 's/^/  run: /'
    logrun p11_control_ranga_alone cyrius test --no-deps --bounds control/ranga_alone.tcyr
    local pad
    pad=$(headroom tests/p11_full.tcyr --bounds)
    echo "  24 MiB cap: largest pad that still preprocesses = $pad B -> expanded unit ~ $((25165824 - pad)) B (±4096)" | tee "$OUT/p11_headroom.log"
    # Names.
    python3 -I "$HERE/names.py" prefixes lib/*.cyr > "$OUT/p11_prefixes.log"
    python3 -I "$HERE/names.py" shared lib/mabda.cyr lib/chitra.cyr lib/prakash.cyr lib/ranga.cyr lib/impetus.cyr \
        lib/pravash.cyr lib/goonj.cyr lib/ushma.cyr lib/selah.cyr lib/agnodrm.cyr lib/samvada.cyr lib/hisab.cyr \
        lib/bayan.cyr lib/sandhi.cyr lib/sigil.cyr lib/sankoch.cyr lib/sakshi.cyr > "$OUT/p11_shared.log"
    grep -v ': 0$' "$OUT/p11_prefixes.log" | sed 's/^/  /'
    tail -1 "$OUT/p11_shared.log" | sed 's/^/  /'
    cd "$HERE" || return 1
}

case "${1:-all}" in
    p4) p4 ;;
    p5) p5 ;;
    p11) p11 ;;
    all) p4; p5; p11 ;;
    *) echo "usage: $0 [p4|p5|p11|all]"; exit 2 ;;
esac
