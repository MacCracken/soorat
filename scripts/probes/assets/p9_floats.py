#!/usr/bin/env python3
"""P9: classify every JSON number in fixtures/p9/float_stress.glb against exact arithmetic.

For each of the 4,000 decimal strings (400 joint nodes x T3 R4 S3) it computes, exactly
(fractions.Fraction):
  cr32  the correctly rounded f32 of the decimal;
  dr32  the f32 of the correctly rounded f64 (double rounding: what a correct f64 parser
        followed by `as f32` gives);
and compares Rust 1.2.0's joint TRS (out/p9/rust/float_stress.txt: serde_json -> f32) and
the Cyrius prototype's (out/p9/cyr/float_stress.txt: bayan f64 -> f32_from).
Run from scripts/probes/assets: python3 -I p9_floats.py
"""
import json
import struct
from fractions import Fraction


def nearest_f32(x: Fraction) -> int:
    """Bits of the f32 nearest to x (ties to even); finite inputs within f32 range."""
    if x == 0:
        return 0
    sign = 0x80000000 if x < 0 else 0
    a = -x if x < 0 else x
    # binade
    e = a.numerator.bit_length() - a.denominator.bit_length()
    if Fraction(2) ** e > a:
        e -= 1
    while Fraction(2) ** (e + 1) <= a:
        e += 1
    if e < -126:
        e = -126  # subnormal spacing
    q = a / Fraction(2) ** (e - 23)  # significand scaled to 24 bits
    n = q.numerator // q.denominator
    rem = q - n
    if rem > Fraction(1, 2) or (rem == Fraction(1, 2) and n % 2 == 1):
        n += 1
    bits_e = e + 127
    if n >= 1 << 24:
        n >>= 1
        bits_e += 1
    if n < 1 << 23:  # subnormal
        return sign | n
    if bits_e >= 255:
        return sign | 0x7F800000
    return sign | (bits_e << 23) | (n - (1 << 23))


def f64_to_f32_bits(s: str) -> int:
    return struct.unpack("<I", struct.pack("<f", float(s)))[0]


def load_numbers():
    raw = open("fixtures/p9/float_stress.glb", "rb").read()
    jl = struct.unpack_from("<I", raw, 12)[0]
    doc = json.loads(raw[20 : 20 + jl], parse_float=str, parse_int=str)
    out = []
    for nd in doc["nodes"]:
        out.append(nd["translation"] + nd["rotation"] + nd["scale"])
    return out


def load_dump(path):
    rows = []
    for line in open(path):
        if line.startswith("joint "):
            f = line.split()
            t = f[f.index("t") + 1 : f.index("t") + 4]
            r = f[f.index("r") + 1 : f.index("r") + 5]
            s = f[f.index("s") + 1 : f.index("s") + 4]
            rows.append([int(h, 16) for h in t + r + s])
    return rows


def main():
    nums = load_numbers()
    rust = load_dump("out/p9/rust/float_stress.txt")
    cyr = load_dump("out/p9/cyr/float_stress.txt")
    stats = {"n": 0, "rust_cr": 0, "rust_dr": 0, "cyr_cr": 0, "cyr_dr": 0, "rust_eq_cyr": 0,
             "cr_ne_dr": 0}
    kinds = {}
    examples = []
    for i, row in enumerate(nums):
        for j, s in enumerate(row):
            kind = "random-shortest" if (i * 10 + j + 1) % 2 == 0 else "midpoint-20digit"
            k = kinds.setdefault(kind, {"n": 0, "rust_dr": 0, "cyr_dr": 0, "rust_cr": 0, "cyr_cr": 0})
            cr = nearest_f32(Fraction(s))
            dr = f64_to_f32_bits(s)
            rb, cb = rust[i][j], cyr[i][j]
            stats["n"] += 1
            k["n"] += 1
            stats["cr_ne_dr"] += cr != dr
            stats["rust_cr"] += rb == cr
            stats["cyr_cr"] += cb == cr
            stats["rust_dr"] += rb == dr
            stats["cyr_dr"] += cb == dr
            k["rust_dr"] += rb == dr
            k["cyr_dr"] += cb == dr
            k["rust_cr"] += rb == cr
            k["cyr_cr"] += cb == cr
            stats["rust_eq_cyr"] += rb == cb
            if rb != cb and len(examples) < 4:
                examples.append((i, j, s, f"{rb:08x}", f"{cb:08x}", f"{dr:08x}", f"{cr:08x}"))
    print("numbers", stats["n"])
    print("rust == cyrius", stats["rust_eq_cyr"])
    print("double-rounded (correct f64 -> f32): rust", stats["rust_dr"], "cyrius", stats["cyr_dr"])
    print("correctly rounded f32:               rust", stats["rust_cr"], "cyrius", stats["cyr_cr"])
    print("decimals where correct-f32 != f64->f32", stats["cr_ne_dr"])
    for kind, k in kinds.items():
        print(f"  {kind}: n {k['n']}  rust==dr {k['rust_dr']}  cyr==dr {k['cyr_dr']}"
              f"  rust==cr {k['rust_cr']}  cyr==cr {k['cyr_cr']}")
    for e in examples:
        print("  node %d comp %d  %s  rust %s  cyr %s  f64->f32 %s  cr %s" % e)


main()
