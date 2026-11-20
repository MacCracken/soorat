#!/usr/bin/env python3
"""P15's checker (Part 0c, ADR 0003): prakash 2.15.2's f64 RGB conversions against the exact value.

    python3 -I -B scripts/probes/math/p15_check.py build/logs/p15-prakash-rgb.log

P15 (`p15_prakash_rgb.tcyr`) prints prakash's f64 r, g, b for 20,000 seeded kelvins and 20,000 wavelengths. Each
channel is compared with scripts/oracle/validate.py's exact reference, rounded to f64:
- `temperature_exact` with prakash's own green exponent (PRAKASH_GREEN_EXP), so that only the arithmetic is measured;
- `wavelength_exact`.
It prints, per channel, how many are the correctly rounded f64, the worst error in f64 ulps, and the blue channel's
worst by kelvin band (its fit cancels near its zero at about 1,905 K). It also counts the f32s that narrowing gets
wrong. Stdlib only.
"""
import math
import os
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
sys.argv = [sys.argv[0], ROOT] + sys.argv[1:]
sys.path.insert(0, os.path.join(ROOT, "scripts", "oracle"))
import validate as V  # noqa: E402

BLUE_BANDS = ((1900, 2000), (2000, 2500), (2500, 3000), (3000, 6600))


def f64(b):
    return struct.unpack("<d", struct.pack("<Q", int(b, 16)))[0]


def ulps(got, want):
    return 0 if got == want else abs(round((got - want) / math.ulp(want)))


def main():
    with open(sys.argv[2], encoding="utf-8") as f:
        lines = [l.strip() for l in f]
    stat, band, f32bad = {}, {b: [0, 0] for b in BLUE_BANDS}, 0
    for i, tag in enumerate(lines):
        if tag not in ("T", "W") or i + 4 >= len(lines):
            continue
        x = f64(lines[i + 1])
        got = [f64(lines[i + 2 + j]) for j in range(3)]
        ex = V.temperature_exact(x, V.PRAKASH_GREEN_EXP) if tag == "T" else V.wavelength_exact(x)
        for ch, (g, e) in enumerate(zip(got, ex)):
            want = float(e)
            d = ulps(g, want)
            s = stat.setdefault((tag, "rgb"[ch]), [0, 0, 0, None])
            s[0] += 1
            s[1] += d == 0
            if d > s[2]:
                s[2], s[3] = d, x
            f32bad += V.r32(g) != V.r32(want)
            if tag == "T" and ch == 2:
                for b in BLUE_BANDS:
                    if b[0] <= x < b[1]:
                        band[b][0] = max(band[b][0], d)
                        band[b][1] += d > 4
    done = "P15 done" in lines
    print(f"P15 log: {'complete' if done else 'INCOMPLETE'}")
    for (tag, ch), (n, exact, worst, at) in sorted(stat.items()):
        name = "temperature" if tag == "T" else "wavelength"
        print(f"{name} {ch}: {n} channels, {exact} correctly rounded ({100 * exact / n:.0f}%), worst {worst} ulp"
              + (f" at {at!r}" if at is not None else ""))
    for b, (worst, over) in band.items():
        print(f"temperature b, {b[0]}-{b[1]} K: worst {worst} ulp, {over} channels over 4 ulp")
    print(f"f32s that narrowing gets wrong: {f32bad}")
    return 0 if done else 1


if __name__ == "__main__":
    sys.exit(main())
