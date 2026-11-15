#!/usr/bin/env python3
"""P3: join out/p3_inputs.txt, out/p3_rust.txt and out/p3_cyrius.txt.

Per (input set, function): how often Rust (glibc f32) and Cyrius (ganita f32 tier)
equal the f64-rounded reference, how often they equal each other, and the worst ulp gaps.
"""
from collections import defaultdict

SETS = ["sphere", "step", "rotation", "slerp", "srgb_dec", "srgb_enc", "splits", "decay"]
FNS = ["sin", "cos", "acos", "pow", "exp", "ln", "log10"]


def key(b):
    return b if b < 0x80000000 else 0x80000000 - b


def ulps(a, b):
    return abs(key(a) - key(b))


inputs = [list(map(int, l.split())) for l in open("out/p3_inputs.txt")]
rust = [list(map(int, l.split())) for l in open("out/p3_rust.txt")]
cyr = [int(l) for l in open("out/p3_cyrius.txt") if l[:1].isdigit()]
assert len(inputs) == len(rust) == len(cyr), (len(inputs), len(rust), len(cyr))

st = defaultdict(lambda: [0, 0, 0, 0, 0, 0, 0])  # n, rust=ref, cyr=ref, rust=cyr, worst r, worst c, worst rc
for (s, f, a, b), (r, ref), c in zip(inputs, rust, cyr):
    x = st[(s, f)]
    x[0] += 1
    x[1] += r == ref
    x[2] += c == ref
    x[3] += r == c
    x[4] = max(x[4], ulps(r, ref))
    x[5] = max(x[5], ulps(c, ref))
    x[6] = max(x[6], ulps(r, c))

print(f"{'set':<9} {'fn':<6} {'n':>6}  {'rust=ref':>9} {'cyr=ref':>9} {'rust=cyr':>9}  worst ulp: rust/ref cyr/ref rust/cyr")
tot = [0, 0, 0, 0]
for (s, f), (n, rr, cr, rc, wr, wc, wrc) in sorted(st.items()):
    tot[0] += n; tot[1] += rr; tot[2] += cr; tot[3] += rc
    print(f"{SETS[s]:<9} {FNS[f]:<6} {n:>6}  {100*rr/n:8.3f}% {100*cr/n:8.3f}% {100*rc/n:8.3f}%   {wr:>4} {wc:>7} {wrc:>8}")
n = tot[0]
print(f"{'all':<16} {n:>6}  {100*tot[1]/n:8.3f}% {100*tot[2]/n:8.3f}% {100*tot[3]/n:8.3f}%")
