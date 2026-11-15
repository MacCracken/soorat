#!/bin/sh
# run_p8_gpu.sh — the P8 GPU soak: 100,000 full frames, then 20,000 frames of each variant
# to split the RSS growth by call. Logs in build/logs/p8-gpu-<variant>.log.
cd "$(dirname "$0")"
mkdir -p build/logs
sh build.sh p8_gpu_frame || exit 1
./build/p8_gpu_frame 100000 full > build/logs/p8-gpu-full.log 2>&1; echo "=== full 100000 -> exit $?"
grep -a 'frame \|B$' build/logs/p8-gpu-full.log
for v in plain nodraw nowrite writeonly pollonly cpuonly; do
    ./build/p8_gpu_frame 20000 "$v" > "build/logs/p8-gpu-$v.log" 2>&1; echo "=== $v 20000 -> exit $?"
    grep -a '  frame ' "build/logs/p8-gpu-$v.log"
done

# Experiment: a COPY of mabda's launcher (build/, mabda untouched) whose submit shim also calls
# wgpuCommandBufferRelease(cmd) — mabda 4.2.0 never releases a submitted command buffer and its
# fn table has no slot for it. If the non-Cyrius RSS growth disappears, that is its cause.
MABDA=${MABDA:-/home/macro/Repos/mabda}
sed 's|    wgpuQueueSubmit(queue, 1, &cmd);|    wgpuQueueSubmit(queue, 1, \&cmd);\n    wgpuCommandBufferRelease(cmd);  /* P8 experiment: release after submit */|' \
    "$MABDA/deps/wgpu_main.c" > build/wgpu_main_cbrelease.c
gcc -c build/wgpu_main_cbrelease.c -I"$MABDA/deps" -I"$MABDA/deps/wgpu-native/include" -Wall -Wextra -Werror \
    -o build/wgpu_main_cbrelease.o
gcc build/wgpu_main_cbrelease.o build/p8_gpu_frame.o "$MABDA/deps/wgpu-native/lib/libwgpu_native.a" \
    -lpthread -ldl -lm -o build/p8_gpu_frame_cbrelease
for v in nodraw full; do
    ./build/p8_gpu_frame_cbrelease 20000 "$v" > "build/logs/p8-gpu-cbrelease-$v.log" 2>&1
    echo "=== cbrelease $v 20000 -> exit $?"
    grep -a '  frame ' "build/logs/p8-gpu-cbrelease-$v.log"
done
