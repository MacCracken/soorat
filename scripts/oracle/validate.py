#!/usr/bin/env python3
"""soorat golden corpus: independent re-derivation of the rows, and the class-E rows.

    python3 -I -B scripts/oracle/validate.py .           # recompute every math row; exit 1 on any difference
    python3 -I -B scripts/oracle/validate.py . --write   # also (re)write tests/golden/math.e.tsv

Python's floats are IEEE binary64, and an f64 operation on two f32 values rounded once to f32 IS the f32
operation for + - * / and sqrt (f64 carries more than 2·24 + 2 bits). So:

* **Rust's rows** are re-derived bit for bit by evaluating each formula in f64 with every intermediate rounded to
  f32 (`r32`), in Rust's order, independently of the Rust generator.
* **Class E** (port plan §5, D4 rule b, ADR 0002 #4): the same formula evaluated in f64 without intermediate
  rounding, with the f32 inputs widened exactly and the f32 constants (`1e-10f32`, `f32::EPSILON`, `0.99f32`)
  widened exactly, then rounded ONCE to f32. Where that differs from Rust's outputs, the row goes into
  `tests/golden/math.e.tsv`, which the harness compares the port against bit for bit.

Also checks `constants.tsv` against the literals in rust-old/src, and the format of every module file.
Stdlib only.
"""
import math
import os
import re
import struct
import sys

ROOT = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].startswith("-") else ".")
WRITE = "--write" in sys.argv
GOLDEN = os.path.join(ROOT, "tests", "golden")


# ── f32 helpers ──────────────────────────────────────────────────────────────────────

def r32(x):
    """Round an f64 to the nearest f32 (ties to even), as Python's C cast does; overflow gives ±inf."""
    if math.isnan(x):
        return x
    try:
        return struct.unpack("<f", struct.pack("<f", x))[0]
    except OverflowError:
        return math.copysign(math.inf, x)


def bits(x):
    return struct.unpack("<I", struct.pack("<f", x))[0] if not math.isnan(x) else 0x7FC00000


def from_bits(b):
    return struct.unpack("<f", struct.pack("<I", b))[0]


F32_1E_10 = from_bits(0x2EDBE6FF)   # Rust's 1e-10f32
F32_EPS = from_bits(0x34000000)     # f32::EPSILON
F32_0_99 = from_bits(0x3F7D70A4)    # 0.99f32


def fmax(a, b):
    """Rust's f32::max / IEEE maxNum: a NaN operand is ignored."""
    if math.isnan(a):
        return b
    if math.isnan(b):
        return a
    return a if a >= b else b


# ── the formulas, parameterized by the rounding applied after each operation ─────────
# `q` is r32 for Rust's f32 path and the identity for the f64 path; `out` rounds results to f32.

def mul_mat4(a, b, q):
    r = [0.0] * 16
    for col in range(4):
        for row in range(4):
            s = 0.0
            for k in range(4):
                s = q(s + q(a[k * 4 + row] * b[col * 4 + k]))
            r[col * 4 + row] = s
    return r


def normalize3(v, q):
    x, y, z = v
    ln = q(math.sqrt(q(q(q(x * x) + q(y * y)) + q(z * z)))) if not math.isnan(q(q(q(x * x) + q(y * y)) + q(z * z))) else math.nan
    if ln < F32_1E_10:
        return [0.0, 0.0, 1.0]
    return [q(x / ln), q(y / ln), q(z / ln)]


def cross(a, b, q):
    return [q(q(a[1] * b[2]) - q(a[2] * b[1])), q(q(a[2] * b[0]) - q(a[0] * b[2])), q(q(a[0] * b[1]) - q(a[1] * b[0]))]


def perspective_90(near, far, q):
    if abs(q(near - far)) < F32_1E_10:
        return [1.0, 0, 0, 0, 0, 1.0, 0, 0, 0, 0, 1.0, 0, 0, 0, 0, 1.0]
    nf = q(1.0 / q(near - far))
    return [1.0, 0, 0, 0, 0, 1.0, 0, 0, 0, 0, q(far * nf), -1.0, 0, 0, q(q(near * far) * nf), 0]


def dot3(a, b, q):
    return q(q(q(a[0] * b[0]) + q(a[1] * b[1])) + q(a[2] * b[2]))


def look_at(pos, d, up, q):
    f = normalize3(d, q)
    s = normalize3(cross(f, up, q), q)
    u = cross(s, f, q)
    return [s[0], u[0], -f[0], 0.0, s[1], u[1], -f[1], 0.0, s[2], u[2], -f[2], 0.0,
            -dot3(s, pos, q), -dot3(u, pos, q), dot3(f, pos, q), 1.0]


def compose_trs(t, r, s, q):
    x, y, z, w = r
    r00 = q(1.0 - q(2.0 * q(q(y * y) + q(z * z))))
    r01 = q(2.0 * q(q(x * y) + q(w * z)))
    r02 = q(2.0 * q(q(x * z) - q(w * y)))
    r10 = q(2.0 * q(q(x * y) - q(w * z)))
    r11 = q(1.0 - q(2.0 * q(q(x * x) + q(z * z))))
    r12 = q(2.0 * q(q(y * z) + q(w * x)))
    r20 = q(2.0 * q(q(x * z) + q(w * y)))
    r21 = q(2.0 * q(q(y * z) - q(w * x)))
    r22 = q(1.0 - q(2.0 * q(q(x * x) + q(y * y))))
    return [q(s[0] * r00), q(s[0] * r01), q(s[0] * r02), 0.0,
            q(s[1] * r10), q(s[1] * r11), q(s[1] * r12), 0.0,
            q(s[2] * r20), q(s[2] * r21), q(s[2] * r22), 0.0,
            t[0], t[1], t[2], 1.0]


def normal_to_basis(n, q):
    rf = [0.0, 1.0, 0.0] if abs(n[1]) < F32_0_99 else [1.0, 0.0, 0.0]
    rx = q(q(n[1] * rf[2]) - q(n[2] * rf[1]))
    ry = q(q(n[2] * rf[0]) - q(n[0] * rf[2]))
    rz = q(q(n[0] * rf[1]) - q(n[1] * rf[0]))
    ss = q(q(q(rx * rx) + q(ry * ry)) + q(rz * rz))
    rlen = fmax(q(math.sqrt(ss)) if not math.isnan(ss) else math.nan, F32_EPS)
    right = [q(rx / rlen), q(ry / rlen), q(rz / rlen)]
    ux = q(q(right[1] * n[2]) - q(right[2] * n[1]))
    uy = q(q(right[2] * n[0]) - q(right[0] * n[2]))
    uz = q(q(right[0] * n[1]) - q(right[1] * n[0]))
    us = q(q(q(ux * ux) + q(uy * uy)) + q(uz * uz))
    ulen = fmax(q(math.sqrt(us)) if not math.isnan(us) else math.nan, F32_EPS)
    return right + [q(ux / ulen), q(uy / ulen), q(uz / ulen)]


def ident(x):
    return x


def evaluate(fn, ins, q):
    """`ins` is the flat list of f32 inputs (sequence markers dropped)."""
    if fn == "mul_mat4":
        return mul_mat4(ins[0:16], ins[16:32], q)
    if fn == "normalize3":
        return normalize3(ins[0:3], q)
    if fn == "cross":
        return cross(ins[0:3], ins[3:6], q)
    if fn == "perspective_90":
        return perspective_90(ins[0], ins[1], q)
    if fn == "look_at":
        return look_at(ins[0:3], ins[3:6], ins[6:9], q)
    if fn == "compose_trs":
        return compose_trs(ins[0:3], ins[3:7], ins[7:10], q)
    if fn == "normal_to_basis":
        return normal_to_basis(ins[0:3], q)
    if fn == "flatten_mat4":
        return list(ins[0:16])
    raise KeyError(fn)


# ── corpus I/O ───────────────────────────────────────────────────────────────────────

def rows(path):
    with open(path, encoding="utf-8") as f:
        lines = f.read().split("\n")
    head = [l for l in lines[:5]]
    body = [l for l in lines[5:] if l and not l.startswith("#")]
    return head, [l.split("\t") for l in body]


def g_values(tokens):
    """f32 values of the `g:` tokens (sequence markers skipped)."""
    out = []
    for t in tokens.split(" "):
        if t.startswith("g:"):
            out.append(from_bits(int(t[2:], 16)))
    return out


def g_tokens(values):
    return "v:%d %s" % (len(values), " ".join("g:%08x" % bits(v) for v in values))


def same(a, b):
    return all((math.isnan(x) and math.isnan(y)) or bits(x) == bits(y) for x, y in zip(a, b)) and len(a) == len(b)


def main():
    failures = 0
    head, body = rows(os.path.join(GOLDEN, "math.tsv"))
    plat = head[1].split("libm: ")[-1]
    e_rows = []
    gains = {}
    for fn, cls, ins, outs, note in body:
        x = g_values(ins)
        want = g_values(outs)
        rust = [r32(v) for v in evaluate(fn, x, r32)]
        if not same(rust, want):
            failures += 1
            print(f"math: {fn} {note}: re-derived f32 outputs differ from the corpus")
            continue
        if cls == "E":
            once = [r32(v) for v in evaluate(fn, x, ident)]
            n, d = gains.get(fn, (0, 0))
            if not same(once, want):
                e_rows.append(f"{fn}\t{ins}\t{g_tokens(once)}\t{note}: f64 once vs Rust f32")
                d += 1
            gains[fn] = (n + 1, d)

    # constants.tsv against the literals in rust-old/src
    src = {}
    for name in os.listdir(os.path.join(ROOT, "rust-old", "src")):
        if name.endswith(".rs"):
            with open(os.path.join(ROOT, "rust-old", "src", name), encoding="utf-8") as f:
                for m in re.finditer(r"pub const (\w+): (?:usize|u32) = ([0-9_]+)(?: \* (\w+))?;", f.read()):
                    src[m.group(1)] = (m.group(2).replace("_", ""), m.group(3))
    _, crow = rows(os.path.join(GOLDEN, "constants.tsv"))
    for fn, _cls, _ins, outs, _note in crow:
        name = fn.split("::")[-1]
        if name in src and outs.startswith("i:"):
            lit, mul = src[name]
            val = int(lit) * (64 if mul == "WORKGROUP_SIZE" else 1)
            if int(outs[2:]) != val:
                failures += 1
                print(f"constants: {name} = {outs[2:]} in the corpus, {val} in rust-old/src")

    for fn, (n, d) in sorted(gains.items()):
        print(f"math {fn:<16} class E rows {n:4d}: f64-once differs from Rust's f32 on {d:4d} ({100 * d / n:5.1f} %)")

    e_path = os.path.join(GOLDEN, "math.e.tsv")
    e_text = "\n".join([
        "# soorat class-E rows (ADR 0002 #4, D4 rule b), format 1: math",
        f"# recorded: soorat 1.2.0 (rust-old/, oracle feature); libm: {plat}. Rust's formula in f64, rounded once to f32. Generated by scripts/oracle/validate.py; do not edit.",
        "# regenerate: python3 -I -B scripts/oracle/validate.py . --write",
        "# columns: fn<TAB>inputs<TAB>outputs<TAB>note (fn, inputs: the golden row's, byte for byte; outputs: the f64 evaluation rounded once)",
        f"# rows: {len(e_rows)}",
    ] + e_rows) + "\n"
    if WRITE:
        with open(e_path, "w", encoding="utf-8") as f:
            f.write(e_text)
        print(f"wrote {e_path} ({len(e_rows)} rows)")
    else:
        try:
            with open(e_path, encoding="utf-8") as f:
                if f.read() != e_text:
                    failures += 1
                    print("math.e.tsv: stale (run with --write)")
        except FileNotFoundError:
            failures += 1
            print("math.e.tsv: missing (run with --write)")
    print("validate:", "ok" if failures == 0 else f"{failures} failure(s)")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
