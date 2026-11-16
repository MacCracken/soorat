#!/bin/sh
# build.sh — build the P6/P7/P8 GPU probe programs against mabda's wgpu path.
#
# The recipe is mabda's (Makefile `build/%.o` + `build/phase0`, docs/stdlib-integration.md §4):
# object-mode compile with cycc, objcopy -L to localize the stdlib's libc-named globals,
# then gcc-link with mabda's C launcher and the static wgpu-native. mabda is read-only:
# its launcher source and wgpu-native are used in place, every output lands in build/.
#
# Which mabda the programs compile against:
#   - default: lib/mabda.cyr, the stdlib fold `cyrius deps` resolved (mabda 4.2.0 on 6.7.18);
#   - MABDA_DIST=<path to a dist/mabda.cyr>: that bundle instead, e.g. a mabda checkout's
#     regenerated dist (the render-tier milestones MB1-MB5 are measured this way). src/ and
#     lib/ are copied into build/stage/ with lib/mabda.cyr replaced, and compiled there, so
#     lib/ itself is never edited. The launcher always comes from $MABDA/deps/wgpu_main.c.
#
# Usage: sh build.sh [program ...]     (default: p6_wgsl p7_upload)
#        MABDA_DIST=/home/macro/Repos/mabda/dist/mabda.cyr sh build.sh p6_wgsl
set -eu
cd "$(dirname "$0")"
MABDA=${MABDA:-/home/macro/Repos/mabda}
WGPU="$MABDA/deps/wgpu-native"
LOCALIZE="-L memcpy -L memset -L memchr -L strlen -L strchr -L strstr -L memeq -L atoi -L print_num -L println"
mkdir -p build

if [ ! -f build/wgpu_main.o ] || [ "$MABDA/deps/wgpu_main.c" -nt build/wgpu_main.o ]; then
    gcc -c "$MABDA/deps/wgpu_main.c" -I"$WGPU/include" -Wall -Wextra -Werror -o build/wgpu_main.o
fi

SRCROOT=.
if [ -n "${MABDA_DIST:-}" ]; then
    [ -f "$MABDA_DIST" ] || { echo "build.sh: MABDA_DIST=$MABDA_DIST is not a file" >&2; exit 1; }
    rm -rf build/stage
    mkdir -p build/stage
    cp -r src lib build/stage/
    cp "$MABDA_DIST" build/stage/lib/mabda.cyr
    SRCROOT=build/stage
    echo "compiling against $MABDA_DIST"
fi

progs=${*:-"p6_wgsl p7_upload"}
for p in $progs; do
    (cd "$SRCROOT" && printf 'object;\n' | cat - "src/$p.cyr" | cycc) > "build/$p.o"
    # shellcheck disable=SC2086
    objcopy $LOCALIZE "build/$p.o"
    gcc build/wgpu_main.o "build/$p.o" "$WGPU/lib/libwgpu_native.a" -lpthread -ldl -lm -o "build/$p"
    echo "built build/$p"
done
