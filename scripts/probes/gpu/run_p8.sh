#!/bin/sh
# run_p8.sh — build and run probe P8; logs in build/logs/p8-<mode>.log.
cd "$(dirname "$0")"
mkdir -p build/logs
cyrius build --no-deps src/p8_arena.cyr build/p8_arena || exit 1
cat /sys/kernel/mm/transparent_hugepage/enabled > build/logs/p8-thp.txt 2>/dev/null
for m in "arena 100000" "growable 100000" "freelist 100000" "global 10000"; do
    set -- $m
    ./build/p8_arena "$1" "$2" > "build/logs/p8-$1.log" 2>&1
    echo "=== p8 $1 $2 -> exit $?"
    cat "build/logs/p8-$1.log"
done
