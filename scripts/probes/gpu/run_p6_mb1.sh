#!/bin/sh
# run_p6_mb1.sh — P6 against mabda's render tier MB1 (mabda 4.3.0): every soorat render
# pipeline through the MB1 descriptor in ONE process, then the error cases (which
# aborted the process under mabda 4.2.0). Logs in build/logs/p6mb1-<case>.log.
#
# Build first, against a mabda checkout's bundle:
#     MABDA_DIST=/home/macro/Repos/mabda/dist/mabda.cyr sh build.sh p6_mb1
cd "$(dirname "$0")"
mkdir -p build/logs
rc_all=0
run() {
    tag=$(echo "$*" | tr ' ' '-')
    log="build/logs/p6mb1-$tag.log"
    timeout 120 ./build/p6_mb1 "$@" > "$log" 2>&1
    rc=$?
    echo "=== p6_mb1 $* -> exit $rc  ($log)"
    grep -v '^mabda: DEPRECATION\|^  The AMD wgpu route\|^adapter ' "$log" | sed 's/^/    /'
    [ "$rc" -eq 0 ] || rc_all=1
}
run pipes
run err
run draw
exit $rc_all
