#!/usr/bin/env python3
"""P14's checker (Part 0c, ADR 0003): settles probe P14's flagged sRGB arguments exactly.

    python3 -I -B scripts/probes/math/p14_check.py build/logs/p14-srgb-pow-<timestamp>.log

P14 (`p14_srgb_pow.tcyr`) prints every f32 argument whose f64 sRGB power lies within 2^-38 of an f32 midpoint
(`D`/`E` lines: the argument's bits, the f64's bits), and every 2^14-th argument's f64 (`SD`/`SE`).
- **Flagged arguments.** Each is evaluated exactly with scripts/oracle/validate.py's `srgb_to_linear_ref` /
  `linear_to_srgb_ref`, which use rational arithmetic and integer power comparisons. Where narrowing the f64 (as the
  port does) gives another f32, the argument is an exception: printed as `(argument bits, correct bits)` pairs, for
  src/tables.cyr's `_SR_SRGB_EXC_*` and validate.py's emitted rows.
- **Sampled arguments.** Their f64's relative error against a 50-digit decimal evaluation is measured. If the worst
  is far below 2^-38, every argument P14 did not flag narrows correctly.
Stdlib only.
"""
import decimal
import math
import os
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
sys.argv = [sys.argv[0], ROOT] + sys.argv[1:]
sys.path.insert(0, os.path.join(ROOT, "scripts", "oracle"))
import validate as V  # noqa: E402


def f64(b):
    return struct.unpack("<d", struct.pack("<Q", b))[0]


def main():
    log = sys.argv[2]
    with open(log, encoding="utf-8") as f:
        lines = [l.strip() for l in f]
    recs = []
    for i, l in enumerate(lines):
        if l in ("D", "E", "SD", "SE") and i + 2 < len(lines):
            recs.append((l, int(lines[i + 1], 16), f64(int(lines[i + 2], 16))))
    flagged = {"D": 0, "E": 0}
    exc = {"D": [], "E": []}
    worst = {"D": 0.0, "E": 0.0}
    decimal.getcontext().prec = 50
    D = decimal.Decimal
    for tag, b, v in recs:
        x = V.from_bits(b)
        if tag in ("D", "E"):
            flagged[tag] += 1
            ref = V.srgb_to_linear_ref(x) if tag == "D" else V.linear_to_srgb_ref(x)
            if V.bits(V.r32(v)) != V.bits(ref):
                exc[tag].append((b, V.bits(ref)))
        else:
            k = tag[1]
            if k == "D":
                exact = ((D(x) + D("0.055")) / D("1.055")) ** D("2.4")
            else:
                exact = D("1.055") * (D(x).ln() * 5 / 12).exp() - D("0.055")
            if math.isfinite(v) and exact != 0:
                worst[k] = max(worst[k], float(abs((D(v) - exact) / exact)))
    done = any(l.startswith("P14 flagged") for l in lines)
    print(f"P14 log: {'complete' if done else 'INCOMPLETE'}; flagged decode {flagged['D']}, encode {flagged['E']}")
    for k, name in (("D", "decode"), ("E", "encode")):
        w = worst[k]
        print(f"{name}: sampled f64 relative error at most {w:.3g} (2^{math.log2(w):.1f})" if w else f"{name}: no samples")
        print(f"{name}: {len(exc[k])} exceptions (narrowing the f64 is wrong):")
        for b, r in exc[k]:
            print(f"    (0x{b:08X}, 0x{r:08X}),   # {V.from_bits(b)!r}")
    return 0 if done else 1


if __name__ == "__main__":
    sys.exit(main())
