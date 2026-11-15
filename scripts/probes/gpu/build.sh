#!/bin/sh
# build.sh — build the P6/P7 GPU probe programs against mabda 4.2.0's wgpu path.
#
# The recipe is mabda's (Makefile `build/%.o` + `build/phase0`, docs/stdlib-integration.md §4):
# object-mode compile with cycc, objcopy -L to localize the stdlib's libc-named globals,
# then gcc-link with mabda's C launcher and the static wgpu-native. mabda is read-only:
# its launcher source and wgpu-native are used in place, every output lands in build/.
#
# Usage: sh build.sh [program ...]     (default: p6_wgsl p7_upload)
set -eu
cd "$(dirname "$0")"
MABDA=${MABDA:-/home/macro/Repos/mabda}
WGPU="$MABDA/deps/wgpu-native"
LOCALIZE="-L memcpy -L memset -L memchr -L strlen -L strchr -L strstr -L memeq -L atoi -L print_num -L println"
mkdir -p build

if [ ! -f build/wgpu_main.o ] || [ "$MABDA/deps/wgpu_main.c" -nt build/wgpu_main.o ]; then
    gcc -c "$MABDA/deps/wgpu_main.c" -I"$WGPU/include" -Wall -Wextra -Werror -o build/wgpu_main.o
fi

progs=${*:-"p6_wgsl p7_upload"}
for p in $progs; do
    printf 'object;\n' | cat - "src/$p.cyr" | cycc > "build/$p.o"
    # shellcheck disable=SC2086
    objcopy $LOCALIZE "build/$p.o"
    gcc build/wgpu_main.o "build/$p.o" "$WGPU/lib/libwgpu_native.a" -lpthread -ldl -lm -o "build/$p"
    echo "built build/$p"
done
