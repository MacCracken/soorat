#!/bin/sh
# run_p6_mb25.sh — P6 against mabda's render tier MB2-MB5 (mabda 4.3.1): soorat's shadow
# map, HDR bloom threshold, IBL cube (faces + generated mips), an sRGB copy, MSAA sprite
# and GPU particles through the public tier, each read back. Logs in
# build/logs/p6mb25-<case>.log. MB5 (a Wayland surface) is mabda's
# programs/surface_wayland_e2e.cyr: it needs a running compositor.
#
# Build first, against a mabda checkout's bundle:
#     MABDA_DIST=/home/macro/Repos/mabda/dist/mabda.cyr sh build.sh p6_mb25
cd "$(dirname "$0")"
mkdir -p build/logs
log="build/logs/p6mb25-all.log"
timeout 180 ./build/p6_mb25 all > "$log" 2>&1
rc=$?
echo "=== p6_mb25 all -> exit $rc  ($log)"
grep -v '^mabda: DEPRECATION\|^  The AMD wgpu route\|^adapter ' "$log" | sed 's/^/    /'
exit $rc
