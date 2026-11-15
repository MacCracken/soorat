#!/bin/sh
# run_p6.sh — run every P6 case in its own process (a wgpu validation error aborts the
# process) and keep each case's full stdout+stderr in build/logs/p6-<case>.log.
cd "$(dirname "$0")"
mkdir -p build/logs
run() {
    tag=$(echo "$*" | tr ' ' '-')
    log="build/logs/p6-$tag.log"
    timeout 120 ./build/p6_wgsl "$@" > "$log" 2>&1
    rc=$?
    echo "=== p6 $* -> exit $rc  ($log)"
    # Show the case's own lines and the first panic message; the abort backtrace stays in the log.
    grep -v '^mabda: DEPRECATION\|^  The AMD wgpu route\|^context OK\|^adapter \|^P6 case' "$log" \
        | sed '/^note: run with/,$d' | grep -v '^ *$' | sed 's/^/    /'
}
run modules
run bad
for s in sprite line pbr pbr_skinned shadow ssao bloom postprocess gpu_particles; do run pipe "$s"; done
run rpb sprite
run draw postprocess
run draw_bound postprocess
run compute
run compute_v2
