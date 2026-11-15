#!/usr/bin/env python3
"""P2: diff the Rust (out/p2_rust.txt) and Cyrius (out/p2_cyrius.txt) math_util outputs.

Usage: python3 p2_compare.py out/p2_rust.txt out/p2_cyrius.txt

Each line is `<fn> <case> <f32 bits in decimal>...`. Reports, per function, how many
cases and values match bit for bit, and the ulp distance of any that don't.
"""
import sys
from collections import defaultdict


def load(path):
    rows = {}
    for line in open(path):
        parts = line.split()
        if len(parts) < 3 or not parts[1].isdigit() or parts[0].endswith(":"):
            continue  # compiler notes, blank lines
        rows[(parts[0], int(parts[1]))] = [int(x) for x in parts[2:]]
    return rows


def key(bits):
    return bits if bits < 0x80000000 else 0x80000000 - bits  # ordered line, ±0 one point


rust, cyr = load(sys.argv[1]), load(sys.argv[2])
stats = defaultdict(lambda: [0, 0, 0, 0])  # cases, values, value mismatches, worst ulp
missing = sorted(set(rust) ^ set(cyr))
for k, rv in rust.items():
    if k not in cyr:
        continue
    cv = cyr[k]
    s = stats[k[0]]
    s[0] += 1
    s[1] += len(rv)
    for a, b in zip(rv, cv):
        if a != b:
            nan = lambda x: (x & 0x7F800000) == 0x7F800000 and (x & 0x7FFFFF)
            if nan(a) and nan(b):
                continue  # both NaN: payload/sign of a NaN is not a parity variable
            s[2] += 1
            s[3] = max(s[3], abs(key(a) - key(b)))
for name, (cases, values, bad, worst) in stats.items():
    print(f"{name:<16} cases {cases:4d}  values {values:5d}  mismatches {bad:3d}  worst {worst} ulp")
print(f"rows only on one side: {len(missing)}")
sys.exit(1 if missing or any(s[2] for s in stats.values()) else 0)
