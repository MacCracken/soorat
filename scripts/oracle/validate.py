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
import json as _json
import decimal
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


def g_tokens(template, values):
    """`template` (a golden row's outputs) with its `g:` tokens replaced, in order, by `values`: the sequence
    structure (`v:3 … v:3 …` for normal_to_basis) stays the golden row's."""
    it = iter(values)
    return " ".join("g:%08x" % bits(next(it)) if t.startswith("g:") else t for t in template.split(" "))


def same(a, b):
    return all((math.isnan(x) and math.isnan(y)) or bits(x) == bits(y) for x, y in zip(a, b)) and len(a) == len(b)


# ── color: Rust's f32 sequence (D4 rule a), re-derived ──────────────────────────────

F32_255 = 255.0
LUMA = (from_bits(0x3E59B3D0), from_bits(0x3F371759), from_bits(0x3D93DD98))   # 0.2126f32, 0.7152f32, 0.0722f32


def clamp32(x, lo, hi):
    """Rust's f32::clamp: lo when x < lo, hi when x > hi, else x (a NaN and a -0 are kept)."""
    if x < lo:
        return lo
    if x > hi:
        return hi
    return x


def round_away(x):
    """Rust's f32::round on a finite x: halfway cases away from zero, the sign kept."""
    r = math.floor(abs(x) + 0.5)
    return math.copysign(r, x)


def to_u8(c):
    if math.isnan(c):
        return 0
    v = round_away(r32(clamp32(c, 0.0, 1.0) * F32_255))
    return int(min(max(v, 0.0), 255.0))


def heat_map(t):
    t = 0.0 if math.isnan(t) else clamp32(t, 0.0, 1.0)
    if t < 0.25:
        return [0.0, r32(t / 0.25), 1.0, 1.0]
    if t < 0.5:
        s = r32(r32(t - 0.25) / 0.25)
        return [0.0, 1.0, r32(1.0 - s), 1.0]
    if t < 0.75:
        return [r32(r32(t - 0.5) / 0.25), 1.0, 0.0, 1.0]
    s = r32(r32(t - 0.75) / 0.25)
    return [1.0, r32(1.0 - s), 0.0, 1.0]


def signed_color(v):
    v = 0.0 if math.isnan(v) else clamp32(v, -1.0, 1.0)
    if v >= 0.0:
        return [v, 0.0, 0.0, 1.0]
    return [0.0, 0.0, -v, 1.0]


def i_values(tokens):
    return [int(t[2:]) for t in tokens.split(" ") if t.startswith("i:")]


def f64_bits_tokens(tokens):
    return [int(t[2:], 16) for t in tokens.split(" ") if t.startswith("f:")]


def check_color():
    """Re-derives every `color` row; returns the number of rows that differ."""
    path = os.path.join(GOLDEN, "color.tsv")
    if not os.path.exists(path):
        return 0
    _, body = rows(path)
    bad = 0
    for fn, _cls, ins, outs, note in body:
        x = g_values(ins)
        if fn == "Color::from_rgba8":
            ok = same([r32(b / F32_255) for b in i_values(ins)], g_values(outs))
        elif fn == "Color::from_hex":
            h = i_values(ins)[0]
            ok = same([r32(((h >> sh) & 0xFF) / F32_255) for sh in (24, 16, 8, 0)], g_values(outs))
        elif fn == "Color::to_rgba8":
            ok = [to_u8(c) for c in x] == i_values(outs)
        elif fn == "Color::to_wgpu":
            ok = [struct.unpack("<Q", struct.pack("<d", c))[0] for c in x] == f64_bits_tokens(outs)
        elif fn == "Color::luminance":
            lum = r32(r32(r32(LUMA[0] * x[0]) + r32(LUMA[1] * x[1])) + r32(LUMA[2] * x[2]))
            ok = same([lum], g_values(outs))
        elif fn == "Color::lerp":
            a, b, t = x[0:4], x[4:8], clamp32(x[8], 0.0, 1.0)
            ok = same([r32(p + r32(r32(q - p) * t)) for p, q in zip(a, b)], g_values(outs))
        elif fn == "visualization_heat_map":
            ok = same(heat_map(x[0]), g_values(outs))
        elif fn == "signed_value_color":
            ok = same(signed_color(x[0]), g_values(outs))
        elif fn == "Color::default":
            ok = same([1.0, 1.0, 1.0, 1.0], g_values(outs))
        else:
            ok = False
            print(f"color: {fn}: no re-derivation in validate.py")
        if not ok:
            bad += 1
            print(f"color: {fn} {note}: re-derived outputs differ from the corpus")
    return bad


# ── lights: copies, and the spot cones' cos correctly rounded (ADR 0002 #16) ──────────

F32_0_1 = from_bits(0x3DCCCCCD)   # 0.1f32


def cos_cr(x):
    """The correctly rounded f32 cos of an f32: glibc's f64 cos (correctly rounded) rounded once."""
    if math.isnan(x) or math.isinf(x):
        return math.nan
    return r32(math.cos(x))


def light_default():
    return [1.0, 1.0, 1.0, F32_0_1] + [0.0] * 132


def side_table(name, what, rows_out, plat):
    """The text of tests/golden/<name>: a side table in .e.tsv's format."""
    return "\n".join([
        f"# soorat {what}, format 1: {name.split('.')[0]}",
        f"# recorded: soorat 1.2.0 (rust-old/, oracle feature); libm: {plat}. Generated by scripts/oracle/validate.py; "
        "do not edit.",
        "# regenerate: python3 -I -B scripts/oracle/validate.py . --write",
        "# columns: fn<TAB>inputs<TAB>outputs<TAB>note (fn, inputs: the golden row's, byte for byte)",
        f"# rows: {len(rows_out)}",
    ] + rows_out) + "\n"


def check_lights(plat):
    """Re-derives every `lights` row; returns (rows that differ, the lights.cr.tsv rows)."""
    path = os.path.join(GOLDEN, "lights.tsv")
    if not os.path.exists(path):
        return 0, []
    _, body = rows(path)
    bad, cr = 0, []
    for fn, _cls, ins, outs, note in body:
        x = g_values(ins)
        want = g_values(outs)
        if fn == "GpuLight::directional":
            mine = x[0:3] + [0.0] + [0.0] * 4 + x[3:6] + [x[6]] + [0.0] * 4
        elif fn == "GpuLight::point":
            mine = x[0:3] + [1.0] + [0.0, 0.0, 0.0, x[7]] + x[3:6] + [x[6]] + [0.0] * 4
        elif fn == "GpuLight::spot":
            rust = x[0:3] + [2.0] + x[3:6] + [x[10]] + x[6:9] + [x[9]]
            mine = rust + [cos_cr(x[11]), cos_cr(x[12]), 0.0, 0.0]
            if not same(rust, want[0:12]):
                bad += 1
                print(f"lights: {fn} {note}: re-derived copies differ from the corpus")
                continue
            if not same(mine, want):
                got = " ".join(f"g:{bits(v):08x}" for v in want[12:14])
                cr.append(f"{fn}\t{ins}\t{g_tokens(outs, mine)}\t{note}; Rust 1.2.0 (glibc cosf) {got}")
            continue
        elif fn == "LightArrayUniforms::default":
            mine = light_default()
        elif fn == "LightArrayUniforms::set_lights":
            n = i_values(ins)[0]
            mine = light_default()
            k = min(n, 8)
            mine[4] = float(k)
            mine[8:8 + 16 * k] = x[0:16 * k]
        elif fn == "LightArrayUniforms::set_ambient":
            mine = x[0:3] + [x[3]] + light_default()[4:]
        else:
            mine = None
            print(f"lights: {fn}: no re-derivation in validate.py")
        if mine is None or not same(mine, want):
            bad += 1
            print(f"lights: {fn} {note}: re-derived outputs differ from the corpus")
    return bad, cr


# ── lod: f32 squares and comparisons (D4 rule a) ─────────────────────────────────────

def seqs(tokens):
    """The token list split into its `v:` sequences (each a list of tokens) and the trailing scalars."""
    out, cur, left = [], None, 0
    for t in tokens.split(" "):
        if t.startswith("v:"):
            cur, left = [], int(t[2:])
            out.append(cur)
            if left == 0:
                cur = None
            continue
        if cur is not None and left > 0:
            cur.append(t)
            left -= 1
            if left == 0:
                cur = None
        else:
            out.append(t)
    return out


def tok_f32(t):
    return from_bits(int(t[2:], 16))


def lod_select(sq, q):
    for i, s in enumerate(sq):
        if q <= s:
            return i
    return len(sq) - 1 if sq else 0


def check_lod():
    path = os.path.join(GOLDEN, "lod.tsv")
    if not os.path.exists(path):
        return 0
    _, body = rows(path)
    bad = 0
    for fn, _cls, ins, outs, note in body:
        parts = seqs(ins)
        if fn in ("LodChain::new", "LodChain::level_count"):
            d = [tok_f32(t) for t in parts[0]]
            sq = [r32(v * v) for v in d]
            if fn == "LodChain::new":
                ok = i_values(outs) == list(range(len(d))) and same(sq, g_values(outs))
            else:
                ok = i_values(outs) == [len(d)]
        elif fn == "LodChain::select":
            d = [tok_f32(t) for t in parts[0]]
            ok = i_values(outs) == [lod_select([r32(v * v) for v in d], tok_f32(parts[1]))]
        elif fn == "LodChain::select_from_positions":
            d, cam, obj = ([tok_f32(t) for t in p] for p in parts[0:3])
            dx, dy, dz = (r32(a - b) for a, b in zip(cam, obj))
            q = r32(r32(r32(dx * dx) + r32(dy * dy)) + r32(dz * dz))
            ok = i_values(outs) == [lod_select([r32(v * v) for v in d], q)]
        elif fn == "TerrainLod::new":
            ok = outs == ("i:1" if len(parts[0]) == len(parts[1]) else "E:Pipeline")
        elif fn == "TerrainLod::select_resolution":
            res = [int(t[2:]) for t in parts[0]]
            d = [tok_f32(t) for t in parts[1]]
            q = tok_f32(parts[2])
            pick = next((res[i] for i, v in enumerate(d) if q <= v), res[-1] if res else 8)
            ok = i_values(outs) == [pick]
        else:
            ok = False
            print(f"lod: {fn}: no re-derivation in validate.py")
        if not ok:
            bad += 1
            print(f"lod: {fn} {note}: re-derived outputs differ from the corpus")
    return bad


# ── tables: f16 (Python's IEEE half) and sRGB (powf correctly rounded, ADR 0002 #16) ──

def f16_to_f32_ref(h):
    """IEEE binary16 → binary32: struct's `e` for every finite or infinite half; a NaN's bits kept (sign, payload)."""
    if (h >> 10) & 0x1F == 0x1F and h & 0x3FF:
        return from_bits(((h & 0x8000) << 16) | 0x7F800000 | ((h & 0x3FF) << 13))
    return struct.unpack("<e", struct.pack("<H", h))[0]


def f32_to_f16_ref(x):
    """IEEE binary32 → binary16, round to nearest even, overflow → ±inf, a NaN → the quiet NaN 0x7e00 (sign kept)."""
    if math.isnan(x):
        return (0x8000 if math.copysign(1.0, x) < 0 else 0) | 0x7E00
    try:
        return struct.unpack("<H", struct.pack("<e", x))[0]
    except OverflowError:
        return (0x8000 if x < 0 else 0) | 0x7C00


SRGB_KNEE = from_bits(0x3D25AEE6)       # 0.040_45f32
SRGB_12_92 = from_bits(0x414EB852)
SRGB_0_055 = from_bits(0x3D6147AE)
SRGB_1_055 = from_bits(0x3F870A3D)
SRGB_2_4 = from_bits(0x4019999A)
SRGB_INV_2_4 = from_bits(0x3ED55555)    # 1.0f32 / 2.4f32, folded in f32
SRGB_LKNEE = from_bits(0x3B4D2E1C)      # 0.003_130_8f32


def powf_cr(b, e):
    """The correctly rounded f32 pow of two f32s (glibc's f64 pow, correctly rounded, rounded once)."""
    try:
        return r32(math.pow(b, e))
    except (ValueError, OverflowError):
        return math.nan if b < 0 else math.inf


def ulp_step(x, k):
    return from_bits((bits(x) + k) & 0xFFFFFFFF) if not math.isnan(x) else x


def srgb_decode(c, pw=powf_cr):
    if c <= SRGB_KNEE:
        return r32(c / SRGB_12_92)
    return pw(r32(r32(c + SRGB_0_055) / SRGB_1_055), SRGB_2_4)


def srgb_encode(l, pw=powf_cr):
    if l <= SRGB_LKNEE:
        return r32(l * SRGB_12_92)
    return r32(r32(SRGB_1_055 * pw(l, SRGB_INV_2_4)) - SRGB_0_055)


def check_tables(plat):
    """Re-derives every `tables` row; returns (rows that differ, the tables.cr.tsv rows). A transfer-function row
    that differs from the correctly rounded value must be Rust's formula with powf 1 ulp off (glibc's misrounding),
    or it is a failure."""
    path = os.path.join(GOLDEN, "tables.tsv")
    if not os.path.exists(path):
        return 0, []
    _, body = rows(path)
    bad, cr = 0, []
    for fn, _cls, ins, outs, note in body:
        if fn == "f16_to_f32":
            ok = same([f16_to_f32_ref(h) for h in i_values(ins)], g_values(outs))
        elif fn == "f32_to_f16":
            ok = [f32_to_f16_ref(x) for x in g_values(ins)] == i_values(outs)
        elif fn == "srgb8_decode_table":
            ok = same([srgb_decode(r32(i / 255.0)) for i in range(256)], g_values(outs))
        elif fn in ("srgb_to_linear", "linear_to_srgb"):
            f = srgb_decode if fn == "srgb_to_linear" else srgb_encode
            x = g_values(ins)[0]
            want = g_values(outs)
            mine = [f(x)]
            ok = same(mine, want)
            if not ok:
                # glibc's powf 1 ulp off, either way, explains the row: record the correctly rounded value.
                off = [f(x, lambda b, e, k=k: ulp_step(powf_cr(b, e), k)) for k in (-1, 1)]
                if any(same([o], want) for o in off):
                    cr.append(f"{fn}\t{ins}\t{g_tokens(outs, mine)}\t{note}; Rust 1.2.0 (glibc powf) {outs}")
                    ok = True
        else:
            ok = False
            print(f"tables: {fn}: no re-derivation in validate.py")
        if not ok:
            bad += 1
            print(f"tables: {fn} {note}: re-derived outputs differ from the corpus")
    return bad, cr


# ── color_optics: prakash 1.2.0's formulas (glibc's pow and log, as Rust's), then Color::from_prakash ──────────

def clamp64(x, lo, hi):
    """Rust's f64::clamp: lo when x < lo, hi when x > hi, else x (NaN and -0 kept)."""
    if x < lo:
        return lo
    if x > hi:
        return hi
    return x


def from_prakash(rgb, alpha):
    return [r32(clamp64(c, 0.0, 1.0)) for c in rgb] + [alpha]


def temperature_rgb(kelvin):
    """prakash 1.2.0's color_temperature_to_rgb (Tanner Helland's fit), in Rust's order."""
    def pw(x, y):
        return math.nan if math.isnan(x) else math.pow(x, y)

    def ln(x):
        return math.nan if math.isnan(x) else math.log(x)
    temp = clamp64(kelvin / 100.0, 10.0, 400.0)
    r = 1.0 if temp <= 66.0 else clamp64(329.698_727_446 * pw(temp - 60.0, -0.133_204_759_2) / 255.0, 0.0, 1.0)
    if temp <= 66.0:
        g = clamp64(99.470_802_586_1 * ln(temp) - 161.119_568_166_1, 0.0, 255.0) / 255.0
    else:
        g = clamp64(288.122_169_528_3 * pw(temp - 60.0, -0.075_514_849_37) / 255.0, 0.0, 1.0)
    if temp >= 66.0:
        b = 1.0
    elif temp <= 19.0:
        b = 0.0
    else:
        b = clamp64(138.517_731_223_1 * ln(temp - 10.0) - 305.044_792_730_7, 0.0, 255.0) / 255.0
    return [r, g, b]


def wavelength_rgb(w):
    """prakash 1.2.0's wavelength_to_rgb, or None outside 380-780 nm (NaN included)."""
    if not (380.0 <= w <= 780.0):
        return None
    if w < 440.0:
        r, g, b = -(w - 440.0) / (440.0 - 380.0), 0.0, 1.0
    elif w < 490.0:
        r, g, b = 0.0, (w - 440.0) / (490.0 - 440.0), 1.0
    elif w < 510.0:
        r, g, b = 0.0, 1.0, -(w - 510.0) / (510.0 - 490.0)
    elif w < 580.0:
        r, g, b = (w - 510.0) / (580.0 - 510.0), 1.0, 0.0
    elif w < 645.0:
        r, g, b = 1.0, -(w - 645.0) / (645.0 - 580.0), 0.0
    else:
        r, g, b = 1.0, 0.0, 0.0
    if w < 420.0:
        factor = 0.3 + 0.7 * (w - 380.0) / (420.0 - 380.0)
    elif w > 700.0:
        factor = 0.3 + 0.7 * (780.0 - w) / (780.0 - 700.0)
    else:
        factor = 1.0
    return [r * factor, g * factor, b * factor]


def f64_tok_values(tokens):
    return [struct.unpack("<d", struct.pack("<Q", b))[0] for b in f64_bits_tokens(tokens)]


def check_color_optics():
    path = os.path.join(GOLDEN, "color_optics.tsv")
    if not os.path.exists(path):
        return 0
    _, body = rows(path)
    bad = 0
    for fn, _cls, ins, outs, note in body:
        if fn == "Color::from_prakash":
            ok = same(from_prakash(f64_tok_values(ins), g_values(ins)[0]), g_values(outs))
        elif fn == "Color::to_prakash":
            x = g_values(ins)
            ok = [struct.unpack("<Q", struct.pack("<d", v))[0] for v in x[0:3]] == f64_bits_tokens(outs)
        elif fn == "Color::from_temperature":
            ok = same(from_prakash(temperature_rgb(f64_tok_values(ins)[0]), 1.0), g_values(outs))
        elif fn == "Color::from_wavelength":
            rgb = wavelength_rgb(f64_tok_values(ins)[0])
            ok = outs == "n:" if rgb is None else same(from_prakash(rgb, 1.0), g_values(outs))
        else:
            ok = False
            print(f"color_optics: {fn}: no re-derivation in validate.py")
        if not ok:
            bad += 1
            print(f"color_optics: {fn} {note}: re-derived outputs differ from the corpus")
    return bad


# ── json: serde's derive semantics for Color, as serde_json 1.0.154 reads ────────────────────────────────



def int_to_f32(n):
    """The f32 nearest the integer n, ties to even (Rust's `n as f32`), exactly: no f64 step."""
    if n == 0:
        return 0.0
    sign = -1.0 if n < 0 else 1.0
    m = abs(n)
    bl = m.bit_length()
    if bl <= 24:
        return sign * float(m)
    sh = bl - 24
    mant, rem, half = m >> sh, m & ((1 << sh) - 1), 1 << (sh - 1)
    if rem > half or (rem == half and mant & 1):
        mant += 1
    return r32(sign * float(mant) * 2.0 ** sh)


class _Num:
    def __init__(self, text, is_int):
        self.text, self.is_int = text, is_int


def _serde_f32(v):
    """A serde f32 field: a number (the integer `n as f32`, the integer -0 as -0.0, a float's f64 `as f32`, refused
    when that f64 is infinite), else None (refused)."""
    if not isinstance(v, _Num):
        return None
    if v.is_int:
        if v.text.lstrip("-") == "0" and v.text.startswith("-"):
            return -0.0
        return int_to_f32(int(v.text))
    x = float(v.text)
    if math.isinf(x):
        return None
    return r32(x)


class _Obj:
    """A JSON object as its (key, value) pairs in document order."""
    def __init__(self, pairs):
        self.pairs = pairs


def _serde_tree(doc):
    """The document as Python values, objects as _Obj and numbers as _Num, or None when it is not one JSON value."""
    def no_const(_c):
        raise ValueError("not JSON")
    try:
        return _json.loads(doc, object_pairs_hook=_Obj, parse_int=lambda t: _Num(t, True),
                           parse_float=lambda t: _Num(t, False), parse_constant=no_const)
    except (ValueError, RecursionError):
        return None


def _serde_fields(v, names):
    """A struct node's fields, as serde's derive reads them: from an object by name (unknown keys ignored, a
    duplicated known field refused), or from an array of exactly len(names); None otherwise."""
    if isinstance(v, _Obj):
        got = {}
        for k, val in v.pairs:
            if k in names:
                if k in got:
                    return None
                got[k] = val
        return [got.get(k) for k in names]
    if isinstance(v, list) and len(v) == len(names):
        return v
    return None


def _serde_f32s(v, names):
    f = _serde_fields(v, names)
    if f is None:
        return None
    out = [_serde_f32(x) for x in f]
    return None if any(o is None for o in out) else out


def _serde_int(v, lo, hi):
    if not isinstance(v, _Num) or not v.is_int or v.text.startswith("-0") and v.text.lstrip("-") == "0":
        return None
    n = int(v.text)
    return n if lo <= n <= hi else None


def serde_color(doc):
    """What serde_json::from_str::<Color>(doc) gives: [r, g, b, a], or None for an error."""
    return _serde_f32s(_serde_tree(doc), ("r", "g", "b", "a"))


def serde_uv(doc):
    return _serde_f32s(_serde_tree(doc), ("u_min", "v_min", "u_max", "v_max"))


def serde_sprite(doc):
    """What serde_json::from_str::<Sprite>(doc) gives: 13 f32, then [texture_id, z_order]; or None."""
    f = _serde_fields(_serde_tree(doc), ("x", "y", "width", "height", "rotation", "color", "texture_id", "z_order", "uv"))
    if f is None:
        return None
    head = [_serde_f32(x) for x in f[0:5]]
    color = _serde_f32s(f[5], ("r", "g", "b", "a"))
    uv = _serde_f32s(f[8], ("u_min", "v_min", "u_max", "v_max"))
    tex = _serde_int(f[6], 0, 2 ** 64 - 1)
    z = _serde_int(f[7], -2 ** 31, 2 ** 31 - 1)
    if any(x is None for x in head) or color is None or uv is None or tex is None or z is None:
        return None
    return head + color + uv, [tex, z]


def check_json():
    path = os.path.join(GOLDEN, "json.tsv")
    if not os.path.exists(path):
        return 0
    _, body = rows(path)
    bad = 0
    for fn, _cls, ins, outs, note in body:
        doc = re.sub(r"%([0-9A-F]{2})", lambda m: chr(int(m.group(1), 16)), ins[2:]).encode("latin-1").decode("utf-8")
        if fn.startswith("Color::from_json"):
            got = serde_color(doc)
            ok = outs == "E:Deserialize" if got is None else same(got, g_values(outs))
        elif fn.startswith("UvRect::from_json"):
            got = serde_uv(doc)
            ok = outs == "E:Deserialize" if got is None else same(got, g_values(outs))
        elif fn.startswith("Sprite::from_json"):
            got = serde_sprite(doc)
            ok = outs == "E:Deserialize" if got is None else (same(got[0], g_values(outs)) and got[1] == i_values(outs))
        else:
            ok = False
            print(f"json: {fn}: no re-derivation in validate.py")
        if not ok:
            bad += 1
            print(f"json: {fn} {note}: re-derived outputs differ from the corpus")
    return bad


# ── sprite: UvRect, Sprite, sort_by_z, and the quads (f32 sin/cos correctly rounded, ADR 0002 #16) ─────────

def trig_cr(fn, x):
    if math.isnan(x) or math.isinf(x):
        return math.nan
    return r32(fn(x))


def quad(s, sn_cs=None):
    """Rust's sprite_quad and emit_quad for the 13-f32 sprite s: 4 vertices of 8 f32. sn_cs overrides (sin, cos)."""
    x, y, w, h, rot = s[0:5]
    hw, hh = r32(w * 0.5), r32(h * 0.5)
    cx, cy = r32(x + hw), r32(y + hh)
    if sn_cs is not None:
        sn, cs = sn_cs
    elif rot != 0.0:
        sn, cs = trig_cr(math.sin, rot), trig_cr(math.cos, rot)
    else:
        sn, cs = 0.0, 1.0
    uv = [(s[9], s[10]), (s[11], s[10]), (s[11], s[12]), (s[9], s[12])]
    out = []
    for (kx, ky), (u, v) in zip([(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)], uv):
        px = r32(r32(r32(kx * cs) - r32(ky * sn)) + cx)
        py = r32(r32(r32(kx * sn) + r32(ky * cs)) + cy)
        out += [px, py, u, v] + list(s[5:9])
    return out


def fnv_halves(data):
    h = 0xCBF29CE484222325
    for b in data:
        h = ((h ^ b) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return [h >> 32, h & 0xFFFFFFFF]


def digest_row(n, u32):
    """The digest outputs for n sprites Sprite::new(k, 0, 1, 1): counts, then the vertex and index FNV-1a halves."""
    m = n if u32 else min(n, 16383)
    vb = bytearray()
    ib = bytearray()
    one = [1.0, 1.0, 1.0, 1.0]
    for k in range(m):
        vb += struct.pack("<32f", *quad([r32(float(k)), 0.0, 1.0, 1.0, 0.0] + one + [0.0, 0.0, 1.0, 1.0]))
        base = k * 4
        idx = [base, base + 1, base + 2, base + 2, base + 3, base]
        ib += struct.pack("<6I" if u32 else "<6H", *idx)
    return [m * 4, m * 6] + fnv_halves(vb) + fnv_halves(ib)


def check_sprite():
    """Re-derives every `sprite` row; returns (rows that differ, the sprite.cr.tsv rows)."""
    path = os.path.join(GOLDEN, "sprite.tsv")
    if not os.path.exists(path):
        return 0, []
    _, body = rows(path)
    bad, cr = 0, []
    for fn, _cls, ins, outs, note in body:
        x = g_values(ins)
        ok = True
        if fn == "UvRect::from_pixel_rect":
            px, py, pw, ph, aw, ah = i_values(ins)
            mine = [0.0, 0.0, 1.0, 1.0] if aw == 0 or ah == 0 else \
                [r32(px / aw), r32(py / ah), r32((px + pw) / aw), r32((py + ph) / ah)]
            ok = same(mine, g_values(outs))
        elif fn == "UvRect::corners":
            ok = same([x[0], x[1], x[2], x[1], x[2], x[3], x[0], x[3]], g_values(outs))
        elif fn == "Sprite::center":
            ok = same([r32(x[0] + r32(x[2] / 2.0)), r32(x[1] + r32(x[3] / 2.0))], g_values(outs))
        elif fn == "Sprite::bounds":
            ok = same([x[0], x[1], r32(x[0] + x[2]), r32(x[1] + x[3])], g_values(outs))
        elif fn == "Sprite::default":
            ok = same([0.0, 0.0, 64.0, 64.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0], g_values(outs)) and \
                i_values(outs) == [0, 0]
        elif fn == "SpriteBatch::sort_by_z":
            zs = i_values(ins)
            ok = sorted(range(len(zs)), key=lambda k: zs[k]) == i_values(outs)
        elif fn in ("batch_to_vertices", "batch_to_vertices_u32"):
            n = i_values(ins)[0]
            sprites = [x[13 * k:13 * k + 13] for k in range(n)]
            want = g_values(outs)
            m = n if fn.endswith("u32") or n <= 16383 else 16383
            mine, explained = [], True
            for k, sp in enumerate(sprites[:m]):
                block = quad(sp)
                got = want[32 * k:32 * k + 32]
                if not same(block, got):
                    sn, cs = trig_cr(math.sin, sp[4]), trig_cr(math.cos, sp[4])
                    alts = [quad(sp, (ulp_step(sn, a), ulp_step(cs, b))) for a in (-1, 0, 1) for b in (-1, 0, 1)]
                    explained = explained and any(same(alt, got) for alt in alts)
                mine += block
            idx = []
            for k in range(m):
                idx += [4 * k, 4 * k + 1, 4 * k + 2, 4 * k + 2, 4 * k + 3, 4 * k]
            if i_values(outs) != idx or len(want) != 32 * m:
                ok = False
            elif not same(mine, want):
                if explained:
                    cr.append(f"{fn}\t{ins}\t{g_tokens(outs, mine)}\t{note}: glibc's sinf/cosf misround")
                else:
                    ok = False
        elif fn in ("batch_to_vertices_digest", "batch_to_vertices_u32_digest"):
            ok = digest_row(i_values(ins)[0], fn.startswith("batch_to_vertices_u32")) == i_values(outs)
        else:
            ok = False
            print(f"sprite: {fn}: no re-derivation in validate.py")
        if not ok:
            bad += 1
            print(f"sprite: {fn} {note}: re-derived outputs differ from the corpus")
    return bad, cr


# ── text: glyph rectangles and the sprites text and UI batches hold ──────────────────

def glyph_uv(fnt, ch):
    _tex, gw, gh, cols, aw, ah, first, count = fnt
    if aw == 0 or ah == 0 or cols == 0:
        return [0.0, 0.0, 1.0, 1.0]
    index = ch - first if ch >= first and ch - first < count else 0
    x = min(index % cols * gw, 0xFFFFFFFF)
    y = min(index // cols * gh, 0xFFFFFFFF)
    return [r32(x / aw), r32(y / ah), r32((x + gw) / aw), r32((y + gh) / ah)]


def sprite_tokens(x, y, w, h, color, uv, tex, z):
    return [x, y, w, h, 0.0] + color + uv, [tex, z]


def check_text():
    path = os.path.join(GOLDEN, "text.tsv")
    if not os.path.exists(path):
        return 0
    _, body = rows(path)
    bad = 0
    for fn, _cls, ins, outs, note in body:
        toks = ins.split(" ")
        if fn == "BitmapFont::glyph_uv":
            ints = i_values(ins)
            ok = same(glyph_uv(ints[0:8], ints[8]), g_values(outs))
        elif fn == "TextBatch::draw_text_z":
            fnt = [int(t[2:]) for t in toks[1:9]]
            text = re.sub(r"%([0-9A-F]{2})", lambda m: chr(int(m.group(1), 16)), toks[9][2:]).encode("latin-1").decode("utf-8")
            x, y, scale = (from_bits(int(t[2:], 16)) for t in toks[10:13])
            color = [from_bits(int(t[2:], 16)) for t in toks[14:18]]
            z = int(toks[18][2:])
            cw, chh = r32(r32(float(fnt[1])) * scale), r32(r32(float(fnt[2])) * scale)
            fs, ints = [], []
            for i, ch in enumerate(text):
                if ch == " ":
                    continue
                f, k = sprite_tokens(r32(x + r32(r32(float(i)) * cw)), y, cw, chh, color, glyph_uv(fnt, ord(ch)), fnt[0], z)
                fs += f
                ints += k
            ok = same(fs, g_values(outs)) and i_values(outs) == [len(ints) // 2] + ints
        elif fn == "UiBatch::add_panel":
            v = g_values(ins)
            tex, z = i_values(ins)
            f, k = sprite_tokens(v[0], v[1], v[2], v[3], v[4:8], [0.0, 0.0, 1.0, 1.0], tex, z)
            ok = same(f, g_values(outs)) and i_values(outs) == [1] + k
        else:
            ok = False
            print(f"text: {fn}: no re-derivation in validate.py")
        if not ok:
            bad += 1
            print(f"text: {fn} {note}: re-derived outputs differ from the corpus")
    return bad


# ── debug: LineBatch geometry (f32 sin/cos correctly rounded, ADR 0002 #16) ───────────

TAU32 = from_bits(0x40C90FDB)


def cpr(c, r, k):
    return r32(c + r32(r * k))


def seg_lines(line_list, color):
    out = []
    for a, b in line_list:
        out += list(a) + color + list(b) + color
    return out


def box_lines(lo, hi):
    x0, y0, z0 = lo
    x1, y1, z1 = hi
    pts = [((x0, y0, z0), (x1, y0, z0)), ((x1, y0, z0), (x1, y0, z1)), ((x1, y0, z1), (x0, y0, z1)),
           ((x0, y0, z1), (x0, y0, z0)), ((x0, y1, z0), (x1, y1, z0)), ((x1, y1, z0), (x1, y1, z1)),
           ((x1, y1, z1), (x0, y1, z1)), ((x0, y1, z1), (x0, y1, z0)), ((x0, y0, z0), (x0, y1, z0)),
           ((x1, y0, z0), (x1, y1, z0)), ((x1, y0, z1), (x1, y1, z1)), ((x0, y0, z1), (x0, y1, z1))]
    return pts


def circle_step(c, r, prev, nxt, sphere):
    (ps, pc), (ns, nc) = prev, nxt
    lines = [((cpr(c[0], r, pc), c[1], cpr(c[2], r, ps)), (cpr(c[0], r, nc), c[1], cpr(c[2], r, ns)))]
    if sphere:
        lines.append(((cpr(c[0], r, pc), cpr(c[1], r, ps), c[2]), (cpr(c[0], r, nc), cpr(c[1], r, ns), c[2])))
        lines.append(((c[0], cpr(c[1], r, pc), cpr(c[2], r, ps)), (c[0], cpr(c[1], r, nc), cpr(c[2], r, ns))))
    return lines


def explain_circle(c, r, n, color, sphere, rust):
    """(the correctly rounded vertices, whether Rust's are those with each angle's sin/cos at most 1 ulp off)."""
    if n == 0:
        return [], rust == []
    step = r32(TAU32 / int_to_f32(n))
    cr = [(0.0, 1.0)] + [(trig_cr(math.sin, a), trig_cr(math.cos, a))
                         for a in (r32(step * int_to_f32(i)) for i in range(1, n + 1))]
    per = 14 * (3 if sphere else 1)
    correct = []
    for i in range(1, n + 1):
        correct += seg_lines(circle_step(c, r, cr[i - 1], cr[i], sphere), color)
    if len(rust) != len(correct):
        return correct, False
    trig = [(0.0, 1.0)]
    for i in range(1, n + 1):
        want = rust[per * (i - 1):per * i]
        found = None
        for ds in (0, -1, 1):
            for dc in (0, -1, 1):
                cand = (ulp_step(cr[i][0], ds), ulp_step(cr[i][1], dc))
                if same(seg_lines(circle_step(c, r, trig[i - 1], cand, sphere), color), want):
                    found = cand
                    break
            if found:
                break
        if found is None:
            return correct, False
        trig.append(found)
    return correct, True


def capsule(c, hh, r, n, color, rust):
    top = (c[0], r32(c[1] + hh), c[2])
    bot = (c[0], r32(c[1] - hh), c[2])
    k = 14 * n if n else 0
    t, et = explain_circle(top, r, n, color, False, rust[0:k])
    b, eb = explain_circle(bot, r, n, color, False, rust[k:2 * k])
    lines = [((cpr(top[0], r, dx), top[1], cpr(top[2], r, dz)), (cpr(bot[0], r, dx), bot[1], cpr(bot[2], r, dz)))
             for dx, dz in ((1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0))]
    tail = seg_lines(lines, color)
    return t + b + tail, et and eb and same(tail, rust[2 * k:])


def grid_vertices(h, sp, color):
    if sp <= 0.0 or h <= 0.0:
        return []
    q = r32(h / sp)
    count = 0 if math.isnan(q) else max(-2 ** 31, min(2 ** 31 - 1, math.ceil(q)))
    lines = []
    for i in range(-count, count + 1):
        pos = r32(int_to_f32(i) * sp)
        lines.append(((pos, 0.0, -h), (pos, 0.0, h)))
        lines.append(((-h, 0.0, pos), (h, 0.0, pos)))
    return seg_lines(lines, color)


def check_debug(name):
    """Re-derives every row of `name` (debug or debug_collider); returns (rows that differ, its .cr.tsv rows)."""
    path = os.path.join(GOLDEN, name + ".tsv")
    if not os.path.exists(path):
        return 0, []
    _, body = rows(path)
    bad, cr = 0, []
    for fn, _cls, ins, outs, note in body:
        x = g_values(ins)
        rust = g_values(outs)
        explained = True
        if fn == "LineBatch::line":
            mine = seg_lines([(x[0:3], x[3:6])], x[6:10])
        elif fn == "LineBatch::wire_box":
            mine = seg_lines(box_lines(x[0:3], x[3:6]), x[6:10])
        elif fn == "LineBatch::grid":
            mine = grid_vertices(x[0], x[1], x[2:6])
        elif fn in ("LineBatch::wire_circle", "LineBatch::wire_sphere"):
            n = i_values(ins)[0]
            mine, explained = explain_circle(x[0:3], x[3], n, x[4:8], fn.endswith("sphere"), rust)
        elif fn == "LineBatch::wire_capsule":
            n = i_values(ins)[0]
            mine, explained = capsule(x[0:3], x[3], x[4], n, x[5:9], rust)
        elif fn == "LineBatch::collider":
            kind = i_values(ins)[0]
            dims = f64_tok_values(ins)
            pos, color = x[0:3], x[3:7]
            if kind == 0:
                he = [r32(v) for v in dims]
                mine = seg_lines(box_lines([r32(p - h) for p, h in zip(pos, he)], [r32(p + h) for p, h in zip(pos, he)]), color)
            elif kind == 1:
                mine, explained = explain_circle(pos, r32(dims[0]), 16, color, True, rust)
            elif kind == 2:
                mine, explained = capsule(pos, r32(dims[0]), r32(dims[1]), 16, color, rust)
            elif kind == 6:
                a = [r32(r32(v) + p) for v, p in zip(dims[0:3], pos)]
                b = [r32(r32(v) + p) for v, p in zip(dims[3:6], pos)]
                mine = seg_lines([(a, b)], color)
            else:
                mine = []
        else:
            mine, explained = None, False
            print(f"{name}: {fn}: no re-derivation in validate.py")
        if mine is None or (not same(mine, rust) and not explained):
            bad += 1
            print(f"{name}: {fn} {note}: re-derived outputs differ from the corpus")
        elif not same(mine, rust):
            cr.append(f"{fn}\t{ins}\t{g_tokens(outs, mine)}\t{note}: glibc's sinf/cosf misround")
    return bad, cr


# ── primitives, viz, terrain: geometry in two paths (Rust's f32 sequence; f64 once for class E) ─────────────

PI32 = from_bits(0x40490FDB)
EPS32 = from_bits(0x34000000)
WHITE4 = [1.0, 1.0, 1.0, 1.0]


def _q(f32):
    """The rounding after each operation: r32 for Rust's f32 path, none for the f64 path."""
    return r32 if f32 else (lambda x: x)


def vtx(pos, nrm, uv, col):
    return [r32(x) for x in list(pos) + list(nrm) + list(uv)] + list(col)


def sphere_mesh(seg, rings, f32, trig=None):
    q = _q(f32)
    seg, rings = max(seg, 3), max(rings, 2)
    vs = []
    for y in range(rings + 1):
        v = q(y / rings)
        phi = q(v * PI32)
        for x in range(seg + 1):
            u = q(x / seg)
            th = q(u * TAU32)
            if trig is not None:
                sp, cp, st, ct = trig(y, x, phi, th)
            elif f32:
                sp, cp, st, ct = (trig_cr(f, a) for f, a in ((math.sin, phi), (math.cos, phi), (math.sin, th), (math.cos, th)))
            else:
                sp, cp, st, ct = math.sin(phi), math.cos(phi), math.sin(th), math.cos(th)
            nx, ny, nz = q(sp * ct), cp, q(sp * st)
            vs.append(vtx((q(nx * 0.5), q(ny * 0.5), q(nz * 0.5)), (nx, ny, nz), (u, v), WHITE4))
    idx = lattice(rings, seg)
    return vs, idx


def lattice(rows, cols):
    out, stride = [], cols + 1
    for r in range(rows):
        for c in range(cols):
            tl = r * stride + c
            out += [tl, tl + 1, tl + stride, tl + 1, tl + stride + 1, tl + stride]
    return out


def plane_mesh(divs, f32):
    q = _q(f32)
    divs = max(divs, 1)
    cols = divs + 1
    vs = []
    for z in range(cols):
        for x in range(cols):
            tx, tz = q(x / divs), q(z / divs)
            vs.append(vtx((q(tx - 0.5), 0.0, q(tz - 0.5)), (0.0, 1.0, 0.0), (tx, tz), WHITE4))
    idx = []
    for z in range(divs):
        for x in range(divs):
            tl = z * cols + x
            idx += [tl, tl + cols, tl + 1, tl + 1, tl + cols, tl + cols + 1]
    return vs, idx


def cylinder_mesh(seg, f32, trig=None):
    q = _q(f32)
    seg = max(seg, 3)
    step = q(TAU32 / seg)

    def sc(i):
        a = q(step * i)
        if trig is not None:
            return trig(i, a)
        if f32:
            return trig_cr(math.sin, a), trig_cr(math.cos, a)
        return math.sin(a), math.cos(a)
    vs = []
    for yi in (0, 1):
        y = yi - 0.5
        for i in range(seg + 1):
            sn, cs = sc(i)
            vs.append(vtx((q(cs * 0.5), y, q(sn * 0.5)), (cs, 0.0, sn), (q(i / seg), float(yi)), WHITE4))
    idx = []
    ring = seg + 1
    for i in range(seg):
        idx += [i, i + ring, i + 1, i + ring, i + ring + 1, i + 1]
    for cy, ny, top in ((0.5, 1.0, True), (-0.5, -1.0, False)):
        center = len(vs)
        vs.append(vtx((0.0, cy, 0.0), (0.0, ny, 0.0), (0.5, 0.5), WHITE4))
        for i in range(seg):
            sn, cs = sc(i)
            c, s2 = q(cs * 0.5), q(sn * 0.5)
            vs.append(vtx((c, cy, s2), (0.0, ny, 0.0), (q(c + 0.5), q(s2 + 0.5)), WHITE4))
        for i in range(seg):
            nxt = i + 1 if i + 1 < seg else 0
            idx += [center, center + 1 + nxt, center + 1 + i] if top else [center, center + 1 + i, center + 1 + nxt]
    return vs, idx


def xz_mesh(nx, nz, ox, oz, gy, dx, dz, f32):
    q = _q(f32)
    if nx == 0 or nz == 0:
        return [], []
    inx, inz = q(1.0 / nx), q(1.0 / nz)
    vs, idx = [], []
    for iz in range(nz):
        z0 = q(oz + q(iz * dz))
        z1 = q(z0 + dz)
        v0, v1 = q(iz * inz), q((iz + 1) * inz)
        for ix in range(nx):
            col = [r32(ix / 16.0), r32(iz / 16.0), 0.5, 1.0]
            x0 = q(ox + q(ix * dx))
            x1 = q(x0 + dx)
            u0, u1 = q(ix * inx), q((ix + 1) * inx)
            base = len(vs)
            for pos, uv in (((x0, gy, z0), (u0, v0)), ((x1, gy, z0), (u1, v0)), ((x1, gy, z1), (u1, v1)), ((x0, gy, z1), (u0, v1))):
                vs.append(vtx(pos, (0.0, 1.0, 0.0), uv, col))
            idx += [base + i for i in (0, 3, 2, 0, 2, 1)]
    return vs, idx


def hf_mesh(cols, rows, ox, oz, dx, dz, height, color, f32):
    q = _q(f32)
    if cols == 0 or rows == 0:
        return [], []
    hs = []
    for iz in range(rows):
        for ix in range(cols):
            h = height(ix, iz)
            hs.append(h if math.isfinite(h) else 0.0)

    def slope(base, i, stride, n, step):
        lo, hi = max(i - 1, 0), min(i + 1, n - 1)
        span = q((hi - lo) * step)
        if math.isnan(span) or abs(span) <= EPS32:
            return 0.0
        return q(q(hs[base + lo * stride] - hs[base + hi * stride]) / span)
    inu, inv = q(1.0 / max(cols - 1, 1)), q(1.0 / max(rows - 1, 1))
    vs = []
    for iz in range(rows):
        for ix in range(cols):
            y = hs[iz * cols + ix]
            sx = slope(iz * cols, ix, 1, cols, dx)
            sz = slope(ix, iz, cols, rows, dz)
            ln = q(math.sqrt(q(q(q(sx * sx) + 1.0) + q(sz * sz))))
            vs.append(vtx((q(ox + q(ix * dx)), y, q(oz + q(iz * dz))), (q(sx / ln), q(1.0 / ln), q(sz / ln)),
                          (q(ix * inu), q(iz * inv)), color(ix, iz, y)))
    idx = []
    for r in range(rows - 1):
        for c in range(cols - 1):
            tl = r * cols + c
            idx += [tl, tl + cols, tl + 1, tl + 1, tl + cols, tl + cols + 1]
    return vs, idx


def flat(vs):
    return [x for v in vs for x in v]


def explain_vertices(rust, mine, alts):
    """True when every 12-float vertex of Rust's that differs from mine equals one of alts(k)'s candidates."""
    for k in range(len(mine) // 12):
        got = rust[12 * k:12 * k + 12]
        if not same(mine[12 * k:12 * k + 12], got) and not any(same(a, got) for a in alts(k)):
            return False
    return True


def unit_t_ref(v, mn, mx):
    rng = mx - mn
    if not math.isfinite(v) or math.isnan(rng) or rng <= 2.220446049250313e-16:
        return 0.0
    return clamp32(r32((v - mn) / rng), 0.0, 1.0)


def abs_scale(mn, mx):
    a, b = abs(mn), abs(mx)
    if math.isnan(a):
        return b
    if math.isnan(b):
        return a
    return a if a >= b else b


def signed_t_ref(v, mn, mx, mag):
    sc = abs_scale(mn, mx)
    if not math.isfinite(v) or math.isnan(sc) or sc <= 2.220446049250313e-16:
        return 0.0
    return clamp32(r32(abs(v) / sc), 0.0, 1.0) if mag else clamp32(r32(v / sc), -1.0, 1.0)


def check_geom(name, plat):
    """Re-derives every row of primitives, viz or terrain; returns (rows that differ, its class-E rows)."""
    path = os.path.join(GOLDEN, name + ".tsv")
    if not os.path.exists(path):
        return 0, []
    _, body = rows(path)
    bad, erows = 0, []
    for fn, cls, ins, outs, note in body:
        x, iv = g_values(ins), i_values(ins)
        rust_f, rust_i = g_values(outs), i_values(outs)
        once = None
        ok = True
        if fn == "cube":
            vs, idx = cube_ref()
            ok = same(flat(vs), rust_f) and idx == rust_i
        elif fn == "sphere":
            seg, rg = iv
            vs, idx = sphere_mesh(seg, rg, True)
            seg2, rg2 = max(seg, 3), max(rg, 2)

            def alts(k, seg2=seg2, rg2=rg2):
                yy, xx = divmod(k, seg2 + 1)
                v = r32(yy / rg2)
                phi = r32(v * PI32)
                u = r32(xx / seg2)
                th = r32(u * TAU32)
                base = [trig_cr(f, a) for f, a in ((math.sin, phi), (math.cos, phi), (math.sin, th), (math.cos, th))]
                out = []
                for d in [(a, b, c, e) for a in (-1, 0, 1) for b in (-1, 0, 1) for c in (-1, 0, 1) for e in (-1, 0, 1)]:
                    sp, cp, st, ct = (ulp_step(b, s) for b, s in zip(base, d))
                    nx, nz = r32(sp * ct), r32(sp * st)
                    out.append(vtx((r32(nx * 0.5), r32(cp * 0.5), r32(nz * 0.5)), (nx, cp, nz), (u, v), WHITE4))
                return out
            ok = idx == rust_i and explain_vertices(rust_f, flat(vs), alts)
            once = flat(sphere_mesh(seg, rg, False)[0])
        elif fn == "plane":
            vs, idx = plane_mesh(iv[0], True)
            ok = same(flat(vs), rust_f) and idx == rust_i
            once = flat(plane_mesh(iv[0], False)[0])
        elif fn == "cylinder":
            vs, idx = cylinder_mesh(iv[0], True)
            seg = max(iv[0], 3)
            fv = flat(vs)

            def alts(k, seg=seg, fv=fv):
                # Any vertex of this cylinder is a function of one angle's (sin, cos): rebuild the vertex from each
                # ±1 ulp variant of every angle's pair that could have produced it (the angle is found by trying all).
                out = []
                step = r32(TAU32 / seg)
                for i in range(seg + 1):
                    a = r32(step * i)
                    s0, c0 = trig_cr(math.sin, a), trig_cr(math.cos, a)
                    for ds in (-1, 0, 1):
                        for dc in (-1, 0, 1):
                            sn, cs = ulp_step(s0, ds), ulp_step(c0, dc)
                            for yi in (0, 1):
                                out.append(vtx((r32(cs * 0.5), yi - 0.5, r32(sn * 0.5)), (cs, 0.0, sn), (r32(i / seg), float(yi)), WHITE4))
                            for cy, ny in ((0.5, 1.0), (-0.5, -1.0)):
                                c, s2 = r32(cs * 0.5), r32(sn * 0.5)
                                out.append(vtx((c, cy, s2), (0.0, ny, 0.0), (r32(c + 0.5), r32(s2 + 0.5)), WHITE4))
                return out
            ok = idx == rust_i and explain_vertices(rust_f, fv, alts)
            once = flat(cylinder_mesh(iv[0], False)[0])
        elif fn in ("unit_t", "signed_t", "magnitude_t"):
            v, mn, mx = f64_tok_values(ins)
            got = unit_t_ref(v, mn, mx) if fn == "unit_t" else signed_t_ref(v, mn, mx, fn == "magnitude_t")
            ok = same([got], rust_f)
        elif fn == "finite_min_max":
            vals = [v for v in f64_tok_values(ins) if math.isfinite(v)]
            lo = hi = None
            for v in vals:   # Rust's lo.min(v) / hi.max(v): a tie (±0) takes v
                lo = v if lo is None or not lo < v else lo
                hi = v if hi is None or not hi > v else hi
            want = [] if lo is None else [struct.unpack("<Q", struct.pack("<d", z))[0] for z in (lo, hi)]
            ok = outs == "n:" if not vals else f64_bits_tokens(outs) == want
        elif fn == "grid_len":
            prod = 1
            for d in iv:
                prod *= d
            ok = outs == "n:" if prod >= 2 ** 64 else rust_i == [prod]
        elif fn == "grid_is_backed":
            dims, data_len = iv[:-1], iv[-1]
            prod = 1
            for d in dims:
                prod *= d
            ok = rust_i == [1 if 0 < prod < 2 ** 64 and prod <= data_len else 0]
        elif fn in ("heat_rgba", "signed_rgba"):
            c = heat_map(x[0]) if fn == "heat_rgba" else signed_color(x[0])
            ok = same(c[0:3] + [x[1]], rust_f)
        elif fn == "with_alpha":
            a = clamp32(x[4], 0.0, 1.0) if math.isfinite(x[4]) else 0.0
            ok = same(x[0:3] + [a], rust_f)
        elif fn == "xz_heatmap_mesh":
            nx, nz = iv
            vs, idx = xz_mesh(nx, nz, *x[0:5], True)
            ok = same(flat(vs), rust_f) and idx == rust_i
            once = flat(xz_mesh(nx, nz, *x[0:5], False)[0])
        elif fn == "heightfield_mesh":
            cols, rws = iv
            hs = x[4:]

            def hgt(ix, iz, hs=hs, cols=cols):
                return hs[iz * cols + ix]

            def col(ix, iz, y):
                return [y, float(ix), float(iz), 1.0]
            vs, idx = hf_mesh(cols, rws, *x[0:4], hgt, col, True)
            ok = same(flat(vs), rust_f) and idx == rust_i
            once = flat(hf_mesh(cols, rws, *x[0:4], hgt, col, False)[0])
        elif fn == "lattice_indices":
            ok = rust_i == lattice(iv[0], iv[1])
        elif fn == "TerrainConfig::default":
            ok = rust_i == [64, 64] and same([64.0, 64.0, 10.0], rust_f)
        elif fn == "generate_terrain":
            w, dp = iv
            ww, wd, hsc = x[0:3]
            heights = x[3:]
            if w == 0 or dp == 0:
                vs, idx = [], []
                ok = rust_f == [] and rust_i == []
            else:
                cols = w + 1

                def hgt(ix, iz, heights=heights, cols=cols, hsc=hsc):
                    i = iz * cols + ix
                    return r32(heights[i] * hsc) if i < len(heights) else 0.0
                args = (cols, dp + 1, r32(-ww * 0.5), r32(-wd * 0.5), r32(ww / w), r32(wd / dp), hgt, lambda a, b, c: WHITE4)
                vs, idx = hf_mesh(*args, True)
                ok = same(flat(vs), rust_f) and idx == rust_i
                once = flat(hf_mesh(*args, False)[0])
        elif fn == "flat_heightmap":
            ok = rust_i == [(iv[0] + 1) * (iv[1] + 1)]
        else:
            ok = False
            print(f"{name}: {fn}: no re-derivation in validate.py")
        if not ok:
            bad += 1
            print(f"{name}: {fn} {note}: re-derived outputs differ from the corpus")
        elif cls == "E" and once is not None and not same(once, rust_f):
            erows.append(f"{fn}\t{ins}\t{g_tokens(outs, once)}\t{note}: f64 once vs Rust f32")
    return bad, erows


# ── animation (M3 part b): brackets and lerp in Rust's f32 (A); slerp and joint chains in f64 once (E) ──────────

F32_9995 = from_bits(0x3F7FDF3B)     # 0.9995f32
F32_1E_12 = from_bits(0x2B8CBCCC)    # 1e-12f32
IDENT4 = [0.0, 0.0, 0.0, 1.0]
IDENT16 = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]


class Rd:
    """Reads a token list in order (scripts/oracle/src/m_anim.rs's encodings)."""

    def __init__(self, text):
        self.t = [] if text == "-" else text.split(" ")
        self.k = 0

    def tok(self, kind):
        t = self.t[self.k]
        self.k += 1
        assert t.startswith(kind + ":"), (kind, t)
        return t[2:]

    def i(self):
        return int(self.tok("i"))

    def g(self):
        return from_bits(int(self.tok("g"), 16))

    def gs(self):
        return [self.g() for _ in range(int(self.tok("v")))]

    def track(self):
        return [(self.g(), self.gs()) for _ in range(self.i())]

    def joint(self):
        return {"parent": self.i(), "ib": self.gs(), "t": self.gs(), "r": self.gs(), "s": self.gs()}

    def skeleton(self):
        return [self.joint() for _ in range(self.i())]


def partition_point(keys, time):
    """Rust 1.100's `partition_point(|k| k.time <= time)`: its branch-free binary search, so an unsorted track gives
    Rust's index too."""
    size, base = len(keys), 0
    while size > 1:
        half = size // 2
        if keys[base + half][0] <= time:
            base += half
        size -= half
    return base + (1 if keys[base][0] <= time else 0)


def bracket_ref(keys, time):
    """("At", value) / ("Between", a, b, t), or None: Rust's `bracket` in f32."""
    if not keys:
        return None
    if len(keys) == 1 or math.isnan(time) or time <= keys[0][0]:
        return ("At", keys[0][1])
    if time >= keys[-1][0]:
        return ("At", keys[-1][1])
    hi = min(max(partition_point(keys, time), 1), len(keys) - 1)
    a, b = keys[hi - 1], keys[hi]
    dt = r32(b[0] - a[0])
    t = clamp32(r32(r32(time - a[0]) / dt), 0.0, 1.0) if abs(dt) > F32_EPS else 0.0
    return ("Between", a[1], b[1], t)


def lerp_keys(br):
    if br is None:
        return []
    if br[0] == "At":
        return list(br[1])
    _, a, b, t = br
    return [r32(x + r32(r32(y - x) * t)) for x, y in zip(a, b)]


def slerp32(a, b, t, d=(0, 0, 0, 0)):
    """Rust's f32 slerp, each sinf/acosf correctly rounded and then moved d[k] ulps (glibc may differ by 1)."""
    c = r32(r32(r32(r32(a[0] * b[0]) + r32(a[1] * b[1])) + r32(a[2] * b[2])) + r32(a[3] * b[3]))
    if c < 0.0:
        b = [-x for x in b]
        c = -c
    if c > F32_9995:
        wa, wb = r32(1.0 - t), t
    else:
        cc = clamp32(c, -1.0, 1.0)
        th = ulp_step(math.nan if math.isnan(cc) else r32(math.acos(cc)), d[0])
        sn = ulp_step(trig_cr(math.sin, th), d[1])
        wa = r32(ulp_step(trig_cr(math.sin, r32(r32(1.0 - t) * th)), d[2]) / sn) if sn != 0.0 else math.nan
        wb = r32(ulp_step(trig_cr(math.sin, r32(t * th)), d[3]) / sn) if sn != 0.0 else math.nan
    q = [r32(r32(wa * x) + r32(wb * y)) for x, y in zip(a, b)]
    ss = r32(r32(r32(r32(q[0] * q[0]) + r32(q[1] * q[1])) + r32(q[2] * q[2])) + r32(q[3] * q[3]))
    ln = math.nan if math.isnan(ss) else r32(math.sqrt(ss))
    if math.isfinite(ln) and ln > F32_1E_12:
        return [r32(x / ln) for x in q]
    return list(IDENT4)


def slerp32_alts(a, b, t):
    """Every output Rust's slerp can give when glibc's sinf/acosf are each within 1 ulp of correctly rounded."""
    return [slerp32(a, b, t, (p, q, r, s)) for p in (0, -1, 1) for q in (0, -1, 1) for r in (0, -1, 1) for s in (0, -1, 1)]


_DEC_PREC = 80
_DEC_PI = None


def _dec_pi():
    """π to 80+ digits (Machin's formula), computed once."""
    global _DEC_PI
    if _DEC_PI is None:
        with decimal.localcontext() as ctx:
            ctx.prec = _DEC_PREC + 10

            def arctan_inv(n):
                x = decimal.Decimal(1) / n
                x2, s, t, k = x * x, decimal.Decimal(0), x, 0
                while t != 0:
                    s += t / (2 * k + 1) if k % 2 == 0 else -t / (2 * k + 1)
                    t *= x2
                    k += 1
                    if abs(t) < decimal.Decimal(10) ** -(_DEC_PREC + 8):
                        break
                return s
            _DEC_PI = 4 * (4 * arctan_inv(5) - arctan_inv(239))
    return _DEC_PI


def _dec_sin_cos(x):
    """(sin x, cos x) of the Decimal x to about 80 digits: reduced by 2π, then Taylor."""
    pi = _dec_pi()
    x = x - 2 * pi * (x / (2 * pi)).to_integral_value()
    s, c = decimal.Decimal(0), decimal.Decimal(0)
    term, k = x, 1
    while term != 0 and abs(term) > decimal.Decimal(10) ** -(_DEC_PREC + 5):
        s += term
        term = -term * x * x / ((2 * k) * (2 * k + 1))
        k += 1
    term, k = decimal.Decimal(1), 1
    while term != 0 and abs(term) > decimal.Decimal(10) ** -(_DEC_PREC + 5):
        c += term
        term = -term * x * x / ((2 * k - 1) * (2 * k))
        k += 1
    return s, c


def cr_sin(x):
    """sin x correctly rounded to f64 (exact decimal arithmetic, then Python's correctly rounded conversion)."""
    if not math.isfinite(x):
        return math.nan
    if x == 0.0:
        return x
    with decimal.localcontext() as ctx:
        ctx.prec = _DEC_PREC
        return float(_dec_sin_cos(decimal.Decimal(x))[0])


def cr_acos(x):
    """acos x correctly rounded to f64: Newton on cos θ = x from glibc's acos, in 80-digit decimals."""
    if math.isnan(x) or abs(x) > 1.0:
        return math.nan
    if x == 1.0:
        return 0.0
    with decimal.localcontext() as ctx:
        ctx.prec = _DEC_PREC
        if x == -1.0:
            return float(_dec_pi())
        dx = decimal.Decimal(x)
        th = decimal.Decimal(math.acos(x))
        for _ in range(8):
            s, c = _dec_sin_cos(th)
            th = th + (c - dx) / s
        return float(th)


def sin64(x):
    return cr_sin(x)


def slerp64(a, b, t):
    """Rust's slerp in f64 from the f32 inputs (constants widened), with correctly rounded acos and sin (cr_acos,
    cr_sin: src/crmath.cyr's), rounded once (D4 rule b; src/animation.cyr)."""
    c = ((a[0] * b[0] + a[1] * b[1]) + a[2] * b[2]) + a[3] * b[3]
    if c < 0.0:
        b = [-x for x in b]
        c = -c
    if c > F32_9995:
        wa, wb = 1.0 - t, t
    else:
        cc = clamp32(c, -1.0, 1.0)
        th = cr_acos(cc)
        sn = sin64(th)
        wa = sin64((1.0 - t) * th) / sn if sn != 0.0 else math.nan
        wb = sin64(t * th) / sn if sn != 0.0 else math.nan
    q = [wa * x + wb * y for x, y in zip(a, b)]
    ln = math.sqrt(((q[0] * q[0] + q[1] * q[1]) + q[2] * q[2]) + q[3] * q[3]) if not math.isnan(q[0] + q[1] + q[2] + q[3]) else math.nan
    if math.isfinite(ln) and ln > F32_1E_12:
        return [r32(x / ln) for x in q]
    return list(IDENT4)


def rotation_keys(keys, time, once):
    """Rust's `interpolate_rotation`: (the f32 value or the f64-once value, every value Rust's glibc could give)."""
    br = bracket_ref(keys, time)
    if br is not None and br[0] == "Between" and len(br[1]) >= 4 and len(br[2]) >= 4:
        a, b, t = br[1][0:4], br[2][0:4], br[3]
        if once:
            return slerp64(a, b, t), None
        return slerp32(a, b, t), (lambda a=a, b=b, t=t: slerp32_alts(a, b, t))
    v = lerp_keys(br)
    return v, None


def world_ref(joints, q, resolve):
    """World transforms of the first MAX_JOINTS joints: Rust's (q = r32) or f64 throughout (q = ident)."""
    js = joints[:128]
    n = len(js)
    local = [compose_trs(j["t"], j["r"], j["s"], q) for j in js]
    world = [None] * n
    if not resolve and all(j["parent"] < 0 or j["parent"] < i for i, j in enumerate(js)):
        for i, j in enumerate(js):
            world[i] = mul_mat4(world[j["parent"]], local[i], q) if j["parent"] >= 0 else local[i]
        return world

    def parent_of(i):
        p = js[i]["parent"]
        return p if 0 <= p < n and p != i else None
    state = [0] * n
    for start in range(n):
        if state[start] == 2:
            continue
        chain, j = [], start
        while True:
            state[j] = 1
            chain.append(j)
            p = parent_of(j)
            if p is not None and state[p] == 0:
                j = p
            else:
                break
        for k in reversed(chain):
            p = parent_of(k)
            world[k] = mul_mat4(world[p], local[k], q) if p is not None and state[p] == 2 else local[k]
            state[k] = 2
    return world


def rem_euclid32(x, y):
    r = math.fmod(x, y)
    return r32(r + abs(y)) if r < 0.0 else r


def sample_ref(dur, channels, joints, time, once):
    """Rust's `AnimationClip::sample`: each joint's (translation, rotation, scale), and per joint the alternatives
    Rust's glibc could give for its rotation (None when exact)."""
    t = rem_euclid32(time, dur) if dur > 0.0 and math.isfinite(time) else 0.0
    pose = [[list(j["t"]), list(j["r"]), list(j["s"])] for j in joints]
    alts = [None] * len(joints)
    for ji, prop, keys in channels:
        if ji >= len(joints):
            continue
        if prop == 1:
            v, alt = rotation_keys(keys, t, once)
            if len(v) >= 4:
                pose[ji][1], alts[ji] = v[0:4], alt
        else:
            v = lerp_keys(bracket_ref(keys, t))
            if len(v) >= 3:
                pose[ji][0 if prop == 0 else 2] = v[0:3]
    return pose, alts


def build_ref(mode, times, width, flat_values):
    values = [flat_values[i:i + width] for i in range(0, len(flat_values), width)] if width else []
    if mode == 0:
        return list(zip(times, values))
    if mode == 2:
        return list(zip(times, values[1::3]))
    n = min(len(times), len(values))
    out = []
    for k in range(n):
        if k > 0:
            out.append((times[k], values[k - 1]))
        out.append((times[k], values[k]))
    return out


def check_anim(plat):
    """Re-derives every `animation` row; returns (rows that differ, its class-E rows, {fn: (rows, differing)})."""
    path = os.path.join(GOLDEN, "animation.tsv")
    if not os.path.exists(path):
        return 0, [], {}
    _, body = rows(path)
    bad, erows, gains = 0, [], {}
    for fn, cls, ins, outs, note in body:
        rd, want = Rd(ins), Rd(outs)
        once, ok = None, True
        if fn == "Joint::default":
            j = want.joint()
            ok = j == {"parent": -1, "ib": IDENT16, "t": [0.0] * 3, "r": IDENT4, "s": [1.0] * 3}
        elif fn == "JointUniforms::default":
            ok = want.gs() == [0.0] * 4 and want.gs() == IDENT16 * 128
        elif fn == "JointUniforms::set_joints":
            n = rd.i()
            ms = rd.gs()
            c = min(n, 128)
            ok = same(want.gs(), [float(c), 0.0, 0.0, 0.0]) and same(want.gs(), ms[:16 * c] + IDENT16 * (128 - c))
        elif fn == "slerp":
            a, b, t = rd.gs(), rd.gs(), rd.g()
            got = want.gs()
            ok = same(slerp32(a, b, t), got) or any(same(x, got) for x in slerp32_alts(a, b, t))
            once = slerp64(a, b, t)
        elif fn in ("bracket", "interpolate_keyframes", "interpolate_rotation"):
            keys, time = rd.track(), rd.g()
            br = bracket_ref(keys, time)
            if fn == "bracket":
                if br is None:
                    ok = outs == "n:"
                elif br[0] == "At":
                    ok = want.tok("s") == "At" and same(want.gs(), br[1])
                else:
                    ok = want.tok("s") == "Between" and same(want.gs(), br[1]) and same(want.gs(), br[2]) and same([want.g()], [br[3]])
            elif fn == "interpolate_keyframes":
                ok = same(want.gs(), lerp_keys(br))
            else:
                got = want.gs()
                v, alt = rotation_keys(keys, time, False)
                ok = same(v, got) or (alt is not None and any(same(x, got) for x in alt()))
                once = rotation_keys(keys, time, True)[0]
        elif fn == "build_keyframes":
            mode, times, width, vals = rd.i(), rd.gs(), rd.i(), rd.gs()
            got = want.track()
            ref = build_ref(mode, times, width, vals)
            ok = len(got) == len(ref) and all(same([g[0]], [r[0]]) and same(g[1], r[1]) for g, r in zip(got, ref))
        elif fn == "local_transform":
            j = rd.joint()
            ok = same(want.gs(), [r32(v) for v in compose_trs(j["t"], j["r"], j["s"], r32)])
            once = [r32(v) for v in compose_trs(j["t"], j["r"], j["s"], ident)]
        elif fn in ("world_transforms", "world_transforms_resolved", "Skeleton::compute_joint_matrices"):
            joints = rd.skeleton()
            resolve = fn == "world_transforms_resolved"

            def mats(q, joints=joints, resolve=resolve, fn=fn):
                w = world_ref(joints, q, resolve)
                if fn == "Skeleton::compute_joint_matrices":
                    w = [mul_mat4(m, j["ib"], q) for m, j in zip(w, joints)]
                return [r32(v) for m in w for v in m]
            ok = same(want.gs(), mats(r32))
            once = mats(ident)
        elif fn == "AnimationClip::sample":
            dur = rd.g()
            channels = [(rd.i(), rd.i(), rd.track()) for _ in range(rd.i())]
            joints, time = rd.skeleton(), rd.g()
            got = want.gs()
            pose, alts = sample_ref(dur, channels, joints, time, False)
            for k, (tt, rr, ss) in enumerate(pose):
                g = got[10 * k:10 * k + 10]
                if not (same(g[0:3], tt) and same(g[7:10], ss)):
                    ok = False
                elif not same(g[3:7], rr) and not (alts[k] is not None and any(same(x, g[3:7]) for x in alts[k]())):
                    ok = False
            ok = ok and len(got) == 10 * len(pose)
            once = [v for p in sample_ref(dur, channels, joints, time, True)[0] for part in p for v in part]
        else:
            ok = False
            print(f"animation: {fn}: no re-derivation in validate.py")
        if not ok:
            bad += 1
            print(f"animation: {fn} {note}: re-derived outputs differ from the corpus")
        elif cls == "E" and once is not None:
            n, d = gains.get(fn, (0, 0))
            if not same(once, g_values(outs)):
                erows.append(f"{fn}\t{ins}\t{g_tokens(outs, once)}\t{note}: f64 once vs Rust f32")
                d += 1
            gains[fn] = (n + 1, d)
    return bad, erows, gains


# ── gltf (M3 part c): the accepted documents re-derived from the fixture bytes ──────────────────────────────

def f32_div(a, b):
    if b == 0.0:
        if a == 0.0 or math.isnan(a):
            return math.nan
        return math.copysign(math.inf, a) * math.copysign(1.0, b)
    return r32(a / b)


def f32_sqrt(x):
    return math.nan if math.isnan(x) or x < 0 else r32(math.sqrt(x))


def f32_max(a, b):
    if math.isnan(a):
        return b
    if math.isnan(b):
        return a
    return b if a < b else a


class Glb:
    """A fixture's JSON document (Python's json: decimals correctly rounded to f64) and BIN chunk."""

    def __init__(self, data):
        self.bin = b""
        text = data
        if data[:4] == b"glTF":
            jl = struct.unpack_from("<I", data, 12)[0]
            text = data[20:20 + jl]
            rest = data[20 + jl:]
            if len(rest) >= 8:
                bl = struct.unpack_from("<I", rest, 0)[0]
                self.bin = rest[8:8 + bl]
        self.doc = _json.loads(text.decode("utf-8").rstrip("\x00 \t\r\n"))

    def coll(self, key):
        v = self.doc.get(key)
        return v if isinstance(v, list) else []

    def view_slice(self, vi):
        v = self.coll("bufferViews")[vi]
        b = self.coll("buffers")[v["buffer"] & 0xFFFFFFFF]
        data = self.bin if b.get("uri") is None else b""
        off = v.get("byteOffset") or 0
        ln = v["byteLength"]
        if off > len(data) or ln > len(data) - off:
            return None
        return data[off:off + ln]

    def items(self, vi, start, count, esize):
        sl = self.view_slice(vi)
        if sl is None or count <= 0:
            return None
        stride = self.coll("bufferViews")[vi].get("byteStride") or esize
        span = stride * (count - 1) + esize
        if start > len(sl) or span > len(sl) - start:
            return None
        if stride < esize:
            raise RuntimeError("stride below the element size")
        return [sl[start + k * stride:start + k * stride + esize] for k in range(count)]

    def read(self, ai, esize):
        a = self.coll("accessors")[ai]
        count, bv, off, sp = a["count"], a.get("bufferView"), a.get("byteOffset") or 0, a.get("sparse")
        if sp is None:
            return None if bv is None else self.items(bv & 0xFFFFFFFF, off, count, esize)
        base = self.items(bv & 0xFFFFFFFF, off, count, esize) if bv is not None else [bytes(esize)] * count
        if base is None:
            return None
        si, sv, sc = sp["indices"], sp["values"], sp["count"]
        isz = {5121: 1, 5123: 2, 5125: 4}[si["componentType"] & 0xFFFFFFFF]
        ix = self.items(si["bufferView"] & 0xFFFFFFFF, si.get("byteOffset") or 0, sc, isz)
        vals = self.items(sv["bufferView"] & 0xFFFFFFFF, sv.get("byteOffset") or 0, sc, esize)
        if ix is None or vals is None:
            return None
        nxt = 0
        out = list(base)
        for c in range(count):
            if nxt < sc and int.from_bytes(ix[nxt], "little") == c:
                out[c] = vals[nxt]
                nxt += 1
        return out


R255, R65535, R127, R32767 = (r32(1.0 / k) for k in (255.0, 65535.0, 127.0, 32767.0))


def gl_norm(ct, item, c):
    if ct == 5126:
        return struct.unpack_from("<f", item, 4 * c)[0]
    if ct == 5121:
        return r32(item[c] * R255)
    if ct == 5123:
        return r32(struct.unpack_from("<H", item, 2 * c)[0] * R65535)
    if ct == 5120:
        return f32_max(r32(struct.unpack_from("<b", item, c)[0] * R127), -1.0)
    return f32_max(r32(struct.unpack_from("<h", item, 2 * c)[0] * R32767), -1.0)


def gl_semantic(k):
    for pre, kind in (("COLOR_", 4), ("TEXCOORD_", 5), ("JOINTS_", 6), ("WEIGHTS_", 7)):
        if k.startswith(pre):
            d = k[len(pre):]
            d = d[1:] if d.startswith("+") else d
            return (kind, int(d)) if d.isdigit() and int(d) < 2 ** 32 else (0, 0)
    return ({"POSITION": 1, "NORMAL": 2, "TANGENT": 3}.get(k, 0), 0)


def gl_attr(at, kind, sset=0):
    found = None
    for k, v in at.items():
        if gl_semantic(k) == (kind, sset):
            found = v & 0xFFFFFFFF
    return found


def gltf_meshes_ref(g):
    images = []
    for im in g.coll("images"):
        if im.get("bufferView") is not None:
            sl = g.view_slice(im["bufferView"] & 0xFFFFFFFF)
            images.append(list(sl) if sl is not None else [])
        else:
            images.append([])
    meshes = []
    for mesh in g.coll("meshes"):
        for p in mesh["primitives"]:
            if (p.get("mode", 4) & 0xFFFFFFFF) != 4:
                continue
            at = p["attributes"]
            pos = g.read(gl_attr(at, 1), 12)
            if not pos:
                continue
            n = len(pos)
            ix = None
            if p.get("indices") is not None:
                ii = p["indices"] & 0xFFFFFFFF
                isz = {5121: 1, 5123: 2, 5125: 4}[g.coll("accessors")[ii]["componentType"]]
                r = g.read(ii, isz)
                if r is not None:
                    ix = [int.from_bytes(x, "little") for x in r]
            if ix is None:
                ix = list(range(n))
            if any(i >= n for i in ix):
                continue
            na = gl_attr(at, 2)
            nrm = g.read(na, 12) if na is not None else None
            ua = gl_attr(at, 5, 0)
            uv, uct = None, 0
            if ua is not None:
                uct = g.coll("accessors")[ua]["componentType"]
                uv = g.read(ua, {5126: 8, 5121: 2, 5123: 4}[uct])
            vs = []
            for j in range(n):
                vs += list(struct.unpack("<3f", pos[j]))
                vs += list(struct.unpack("<3f", nrm[j])) if nrm and j < len(nrm) else [0.0, 1.0, 0.0]
                vs += [gl_norm(uct, uv[j], 0), gl_norm(uct, uv[j], 1)] if uv and j < len(uv) else [0.0, 0.0]
                vs += [1.0, 1.0, 1.0, 1.0]
            mat = p.get("material")
            meshes.append((vs, ix, None if mat is None else mat & 0xFFFFFFFF))
    return meshes, images


def gl_f32(v):
    return r32(float(v))


def gl_decompose(m):
    t = m[12:15]
    xx, xy, xz, yx, yy, yz, zx, zy, zz = m[0], m[1], m[2], m[4], m[5], m[6], m[8], m[9], m[10]

    def mag(a, b, c):
        return f32_sqrt(r32(r32(r32(a * a) + r32(b * b)) + r32(c * c)))
    sx, sy = mag(xx, xy, xz), mag(yx, yy, yz)
    c1 = r32(r32(yy * zz) - r32(zy * yz))
    c2 = r32(r32(xy * zz) - r32(zy * xz))
    c3 = r32(r32(xy * yz) - r32(yy * xz))
    det = r32(r32(r32(xx * c1) - r32(yx * c2)) + r32(zx * c3))
    sg = math.nan if math.isnan(det) else math.copysign(1.0, det)
    sz = r32(sg * mag(zx, zy, zz))
    rx, ry, rz = f32_div(1.0, sx), f32_div(1.0, sy), f32_div(1.0, sz)

    def mul(a, b):
        return math.nan if (a == 0 and math.isinf(b)) or (b == 0 and math.isinf(a)) else r32(a * b)
    xx, xy, xz = mul(xx, rx), mul(xy, rx), mul(xz, rx)
    yx, yy, yz = mul(yx, ry), mul(yy, ry), mul(yz, ry)
    zx, zy, zz = mul(zx, rz), mul(zy, rz), mul(zz, rz)
    tr = r32(r32(xx + yy) + zz)
    if tr >= 0:
        s = f32_sqrt(r32(1.0 + tr))
        w, s2 = r32(0.5 * s), f32_div(0.5, s)
        x, y, z = r32(r32(yz - zy) * s2), r32(r32(zx - xz) * s2), r32(r32(xy - yx) * s2)
    elif xx > yy and xx > zz:
        s = f32_sqrt(r32(r32(r32(xx - yy) - zz) + 1.0))
        x, s2 = r32(0.5 * s), f32_div(0.5, s)
        y, z, w = r32(r32(yx + xy) * s2), r32(r32(xz + zx) * s2), r32(r32(yz - zy) * s2)
    elif yy > zz:
        s = f32_sqrt(r32(r32(r32(yy - xx) - zz) + 1.0))
        y, s2 = r32(0.5 * s), f32_div(0.5, s)
        z, x, w = r32(r32(zy + yz) * s2), r32(r32(yx + xy) * s2), r32(r32(zx - xz) * s2)
    else:
        s = f32_sqrt(r32(r32(r32(zz - xx) - yy) + 1.0))
        z, s2 = r32(0.5 * s), f32_div(0.5, s)
        x, y, w = r32(r32(xz + zx) * s2), r32(r32(zy + yz) * s2), r32(r32(xy - yx) * s2)
    return list(t), [x, y, z, w], [sx, sy, sz]


def gltf_anims_ref(g):
    nodes = g.coll("nodes")
    ident = [1.0, 0, 0, 0, 0, 1.0, 0, 0, 0, 0, 1.0, 0, 0, 0, 0, 1.0]
    skels, first = [], None
    for sk in g.coll("skins"):
        ibm = []
        if sk.get("inverseBindMatrices") is not None:
            r = g.read(sk["inverseBindMatrices"] & 0xFFFFFFFF, 64)
            ibm = [list(struct.unpack("<16f", x)) for x in r] if r else []
        js = [j & 0xFFFFFFFF for j in sk["joints"]]
        jon = {}
        for q, nd in enumerate(js):
            jon[nd] = q
        par = [-1] * len(js)
        for q, nd in enumerate(js):
            for c in nodes[nd].get("children") or []:
                if (c & 0xFFFFFFFF) in jon:
                    par[jon[c & 0xFFFFFFFF]] = q
        joints = []
        for q, nd in enumerate(js):
            node = nodes[nd]
            if node.get("matrix") is not None:
                t, r, s_ = gl_decompose([gl_f32(v) for v in node["matrix"]])
            else:
                t = [gl_f32(v) for v in node.get("translation") or [0, 0, 0]]
                r = [gl_f32(v) for v in node.get("rotation") or [0, 0, 0, 1]]
                s_ = [gl_f32(v) for v in node.get("scale") or [1, 1, 1]]
            joints.append((par[q], ibm[q] if q < len(ibm) else ident, t, r, s_))
        skels.append(joints)
        if first is None:
            first = jon
    clips = []
    for an in g.coll("animations"):
        chans, dur = [], 0.0
        for c in an["channels"]:
            node = c["target"]["node"] & 0xFFFFFFFF
            ji = (first or {}).get(node)
            if ji is None:
                continue
            prop = {"translation": 0, "rotation": 1, "scale": 2}.get(c["target"]["path"])
            if prop is None:
                continue
            sm = an["samplers"][c["sampler"] & 0xFFFFFFFF]
            ti = g.read(sm["input"] & 0xFFFFFFFF, 4)
            times = [struct.unpack("<f", x)[0] for x in ti] if ti else []
            oa = sm["output"] & 0xFFFFFFFF
            oct_, width = (g.coll("accessors")[oa]["componentType"], 4) if prop == 1 else (5126, 3)
            osz = {5126: 4 * width, 5120: 4, 5121: 4, 5122: 8, 5123: 8}[oct_]
            ro = g.read(oa, osz)
            if ro is None:
                continue
            vals = [[gl_norm(oct_, x, k) for k in range(width)] for x in ro]
            if times and math.isfinite(times[-1]):
                dur = f32_max(dur, times[-1])
            mode = {"STEP": 1, "CUBICSPLINE": 2}.get(sm.get("interpolation"), 0)
            chans.append((ji, prop, build_ref(mode, times, width, [v for x in vals for v in x])))
        clips.append((an.get("name") or "unnamed", dur, chans))
    return skels, clips


def pct_decode(s):
    out, i = bytearray(), 0
    while i < len(s):
        if s[i] == "%":
            out.append(int(s[i + 1:i + 3], 16))
            i += 3
        else:
            out += s[i].encode()
            i += 1
    return out.decode("utf-8")


def check_gltf():
    """Re-derives the accepted glTF documents; returns (rows that differ, the gltf.cr.tsv rows, {verdict: rows})."""
    path = os.path.join(GOLDEN, "gltf.tsv")
    if not os.path.exists(path):
        return 0, [], {}
    _, body = rows(path)
    bad, cr, tally = 0, [], {}
    for fn, _cls, ins, outs, note in body:
        name = pct_decode(ins[2:])
        verdict = "refused" if outs.startswith("E:") else ("panics" if outs.startswith("s:panic") else "re-derived")
        tally[verdict] = tally.get(verdict, 0) + 1
        if verdict != "re-derived":
            continue
        with open(os.path.join(GOLDEN, "gltf", name), "rb") as f:
            g = Glb(f.read())
        want = Rd(outs)
        mine = []
        try:
            if fn == "load_gltf_meshes":
                meshes, images = gltf_meshes_ref(g)
                ok = want.i() == len(meshes)
                for vs, ix, mat in meshes:
                    got_v = want.gs()
                    mine += vs
                    ok = ok and same(got_v, vs) and [want.i() for _ in range(int(want.tok("v")))] == ix
                    t = want.t[want.k]
                    want.k += 1
                    ok = ok and (t == "n:" if mat is None else t == f"i:{mat}")
                ok = ok and want.i() == len(images)
                for im in images:
                    ok = ok and [want.i() for _ in range(int(want.tok("v")))] == im
            else:
                skels, clips = gltf_anims_ref(g)
                ok = want.i() == len(skels)
                for joints in skels:
                    ok = ok and want.i() == len(joints)
                    for par, ib, t, r, s_ in joints:
                        ok = ok and want.i() == par
                        for part in (ib, t, r, s_):
                            mine += part
                            got = want.gs()
                            if not same(got, part):
                                ok = ok and (name == "float_stress.glb" and len(got) == len(part))
                ok = ok and want.i() == len(clips)
                for cname, dur, chans in clips:
                    ok = ok and pct_decode(want.tok("s")) == cname and same([want.g()], [dur]) and want.i() == len(chans)
                    mine.append(dur)
                    for ji, prop, keys in chans:
                        ok = ok and want.i() == ji and want.i() == prop and want.i() == len(keys)
                        for tm, val in keys:
                            mine.append(tm)
                            mine += val
                            ok = ok and same([want.g()], [tm]) and same(want.gs(), val)
        except (KeyError, IndexError, ValueError, TypeError, RuntimeError, AssertionError) as e:
            ok = False
            print(f"gltf: {fn} {name}: the re-derivation failed ({e!r})")
        if not ok:
            bad += 1
            print(f"gltf: {fn} {note}: re-derived outputs differ from the corpus")
        elif not same(mine, g_values(outs)):
            cr.append(f"{fn}\t{ins}\t{g_tokens(outs, mine)}\t{note}: joint TRS decimals correctly rounded (bayan) where "
                      "serde_json misrounds")
    return bad, cr, tally


# ── shadow (M4 part a): the matrices and splits in f64 once (E), the atlas exact (A) ─────────────────────────

F32_0_001 = from_bits(0x3A83126F)    # 0.001f32


def cr_pow(x, y):
    """x^y correctly rounded to f64 for the splits' x ≥ 1 (decimal ln and exp at 60 digits)."""
    if math.isnan(x) or math.isnan(y):
        return math.nan
    if y == 0.0 or x == 1.0:
        return 1.0
    if math.isinf(x):
        return math.inf if y > 0 else 0.0
    with decimal.localcontext() as ctx:
        ctx.prec = 60
        return float((decimal.Decimal(x).ln() * decimal.Decimal(y)).exp())


def directional_ref(d, c, e, n, f, q):
    if math.isnan(e) or e <= 0:
        return list(IDENT16)
    span = abs(q(f - n))
    if math.isnan(span) or span < F32_1E_10:
        return list(IDENT16)
    dd = normalize3(d, q)
    up_ref = [1.0, 0.0, 0.0] if abs(dd[1]) > F32_0_99 else [0.0, 1.0, 0.0]
    right = normalize3(cross(dd, up_ref, q), q)
    up = cross(right, dd, q)
    sx = q(1.0 / e)
    sz = q(1.0 / q(f - n))
    return [q(sx * right[0]), q(sx * up[0]), q(sz * dd[0]), 0.0,
            q(sx * right[1]), q(sx * up[1]), q(sz * dd[1]), 0.0,
            q(sx * right[2]), q(sx * up[2]), q(sz * dd[2]), 0.0,
            q(-sx * dot3(right, c, q)), q(-sx * dot3(up, c, q)), q(sz * q(-dot3(dd, c, q) - n)), 1.0]


def splits_ref(n, f, cnt, lam, q, pw):
    """compute_practical_splits; pw(x, y, i) is the pow of split i."""
    near = n if (n > 0 and math.isfinite(n)) else F32_0_001
    far = fmax(f, near) if math.isfinite(f) else near
    lam = 0.5 if math.isnan(lam) else clamp32(lam, 0.0, 1.0)
    count = max(cnt, 1)
    out = [near]
    for i in range(1, count):
        ratio = q(i / count)
        log = q(near * pw(q(far / near), ratio, i))
        uni = q(near + q(q(far - near) * ratio))
        out.append(q(q(lam * log) + q(q(1.0 - lam) * uni)))
    out.append(far)
    return out


def check_shadow():
    """Re-derives every `shadow` row; returns (rows that differ, its class-E rows, {fn: (rows, differing)})."""
    path = os.path.join(GOLDEN, "shadow.tsv")
    if not os.path.exists(path):
        return 0, [], {}
    _, body = rows(path)
    bad, erows, gains = 0, [], {}
    sat = lambda x: min(x, 0xFFFFFFFF)
    f32u = lambda k: r32(float(k))
    for fn, cls, ins, outs, note in body:
        rd, want = Rd(ins), g_values(outs)
        once, ok = None, True
        if fn == "ShadowUniforms::default":
            ok = same(want, IDENT16 * 2)
        elif fn == "CascadeUniforms::default":
            ok = same(want, [10.0, 30.0, 100.0, 500.0] + IDENT16 * 4)
        elif fn in ("directional_light_matrix", "directional_light_matrix_at"):
            d = rd.gs()
            c = rd.gs() if fn == "directional_light_matrix_at" else [0.0, 0.0, 0.0]
            e, n, f = rd.g(), rd.g(), rd.g()
            ok = same(directional_ref(d, c, e, n, f, r32), want)
            once = [r32(v) for v in directional_ref(d, c, e, n, f, ident)]
        elif fn == "compute_practical_splits":
            n, f, cnt, lam = rd.g(), rd.g(), rd.i(), rd.g()

            def powf_alts(i, d):
                return lambda x, y, k: ulp_step(r32(math.pow(x, y)) if not math.isinf(x) else math.inf, d if k == i else 0)
            base = splits_ref(n, f, cnt, lam, r32, powf_alts(0, 0))
            ok = len(base) == len(want)
            for i in range(len(base)):
                if ok and not same([base[i]], [want[i]]):
                    ok = any(same([splits_ref(n, f, cnt, lam, r32, powf_alts(i, d))[i]], [want[i]]) for d in (-1, 1))
            once = [r32(v) for v in splits_ref(n, f, cnt, lam, ident, lambda x, y, k: cr_pow(x, y))]
        elif fn == "PointShadowMap::new":
            p, n, f = rd.gs(), rd.g(), rd.g()
            faces = [([1.0, 0.0, 0.0], [0.0, -1.0, 0.0]), ([-1.0, 0.0, 0.0], [0.0, -1.0, 0.0]),
                     ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0]), ([0.0, -1.0, 0.0], [0.0, 0.0, -1.0]),
                     ([0.0, 0.0, 1.0], [0.0, -1.0, 0.0]), ([0.0, 0.0, -1.0], [0.0, -1.0, 0.0])]

            def cube(q):
                return [v for dr, up in faces for v in mul_mat4(perspective_90(n, f, q), look_at(p, dr, up, q), q)]
            ok = same([r32(v) for v in cube(r32)], want)
            once = [r32(v) for v in cube(ident)]
        elif fn == "atlas_columns":
            sz, t = rd.i(), rd.i()
            ok = i_values(outs) == [max(sz // max(t, 1), 1)]
        elif fn in ("tile_viewport_in", "tile_viewport", "tile_uv_in", "tile_uv"):
            if fn in ("tile_viewport", "tile_uv"):
                sz, t, i = rd.i(), rd.i(), rd.i()
                cols = max(sz // max(t, 1), 1)
            elif fn == "tile_viewport_in":
                cols, t, i = rd.i(), rd.i(), rd.i()
                sz = 0
            else:
                cols, t, sz, i = rd.i(), rd.i(), rd.i(), rd.i()
            cols = max(cols, 1)
            col, row = i % cols, i // cols
            if fn.startswith("tile_viewport"):
                ok = i_values(outs) == [sat(col * t), sat(row * t), t, t]
            else:
                scale = r32(f32u(t) / f32u(max(sz, 1)))
                ok = same([r32(f32u(col) * scale), r32(f32u(row) * scale), scale, scale], want)
        else:
            ok = False
            print(f"shadow: {fn}: no re-derivation in validate.py")
        if not ok:
            bad += 1
            print(f"shadow: {fn} {note}: re-derived outputs differ from the corpus")
        elif cls == "E" and once is not None:
            n_, d_ = gains.get(fn, (0, 0))
            if not same(once, want):
                erows.append(f"{fn}\t{ins}\t{g_tokens(outs, once)}\t{note}: f64 once vs Rust f32")
                d_ += 1
            gains[fn] = (n_ + 1, d_)
    return bad, erows, gains


# ── env, pbr_material, pbr_optics (M4 part b) ─────────────────────────────────────────────────────────────────

def _srgb8_table():
    _, body = rows(os.path.join(GOLDEN, "tables.tsv"))
    for fn, _cls, _ins, outs, _note in body:
        if fn == "srgb8_decode_table":
            return g_values(outs)
    raise KeyError("srgb8_decode_table")


def env_decode(fmt, b):
    out = []
    if fmt == 0:
        tab = _srgb8_table()
        for k in range(len(b) // 4):
            px = b[4 * k:4 * k + 4]
            out += [tab[px[0]], tab[px[1]], tab[px[2]], r32(px[3] / 255.0)]
    else:
        for k in range(len(b) // 8):
            px = b[8 * k:8 * k + 8]
            out += [f16_to_f32_ref(px[i] | (px[i + 1] << 8)) for i in (0, 2, 4, 6)]
    return out


def env_encode(fmt, t, pw=powf_cr):
    out = []
    for k in range(len(t) // 4):
        px = t[4 * k:4 * k + 4]
        if fmt == 0:
            out += [to_u8(srgb_encode(px[0], pw)), to_u8(srgb_encode(px[1], pw)), to_u8(srgb_encode(px[2], pw)), to_u8(px[3])]
        else:
            for c in px:
                h = f32_to_f16_ref(c)
                out += [h & 0xFF, h >> 8]
    return out


def env_encode_glibc_ok(fmt, t, got):
    """True when every byte of Rust's encoding equals the correctly rounded one or one made with powf 1 ulp off."""
    cr = env_encode(fmt, t)
    if len(cr) != len(got):
        return False
    lo, hi = (env_encode(fmt, t, lambda b, e, d=d: ulp_step(powf_cr(b, e), d)) for d in (-1, 1))
    return all(g == c or g == x or g == y for g, c, x, y in zip(got, cr, lo, hi))


def env_downsample(t, size):
    if size == 0 or len(t) < 4 * size * size:
        return []
    last, dst, out = size - 1, max(size // 2, 1), []
    for y in range(dst):
        y0, y1 = min(2 * y, last), min(2 * y + 1, last)
        for x in range(dst):
            x0, x1 = min(2 * x, last), min(2 * x + 1, last)
            for c in range(4):
                a, b = t[4 * (y0 * size + x0) + c], t[4 * (y0 * size + x1) + c]
                d, e = t[4 * (y1 * size + x0) + c], t[4 * (y1 * size + x1) + c]
                out.append(r32(0.25 * r32(r32(r32(a + b) + d) + e)))
    return out


def mip_count(w, h):
    return 1 if w == 0 or h == 0 else max(w, h).bit_length()


def env_chain(fmt, face, size, pw=powf_cr):
    levels, n = [], mip_count(size, size)
    if n <= 1:
        return levels
    level, ls = env_decode(fmt, face), size
    for _ in range(1, n):
        level = env_downsample(level, ls)
        ls = max(ls // 2, 1)
        levels.append((ls, env_encode(fmt, level, pw)))
    return levels


def chain_toks(levels):
    t = [len(levels)]
    for sz, b in levels:
        t += [sz, len(b)] + b
    return t


def check_env():
    """Re-derives every `env` row; returns (rows that differ, the env.cr.tsv rows)."""
    path = os.path.join(GOLDEN, "env.tsv")
    if not os.path.exists(path):
        return 0, []
    _, body = rows(path)
    bad, cr = 0, []
    for fn, _cls, ins, outs, note in body:
        rd = Rd(ins)
        fix = None
        if fn == "mip_level_count":
            ok = i_values(outs) == [mip_count(rd.i(), rd.i())]
        elif fn == "decode_face":
            fmt = rd.i()
            b = [rd.i() for _ in range(int(rd.tok("v")))]
            ok = same(env_decode(fmt, b), g_values(outs))
        elif fn == "encode_face":
            fmt, t = rd.i(), rd.gs()
            got = i_values(outs)
            want = env_encode(fmt, t)
            ok = got == want or (fmt == 0 and env_encode_glibc_ok(fmt, t, got))
            if ok and got != want:
                fix = " ".join(["v:%d" % len(want)] + ["i:%d" % x for x in want])
        elif fn == "downsample":
            size, t = rd.i(), rd.gs()
            ok = same(env_downsample(t, size), g_values(outs))
        elif fn == "face_mip_chain":
            fmt, size = rd.i(), rd.i()
            face = [rd.i() for _ in range(int(rd.tok("v")))]
            got = [int(x[2:]) for x in outs.split(" ")]
            want = chain_toks(env_chain(fmt, face, size))
            ok = got == want
            if not ok and fmt == 0 and len(got) == len(want):
                # Rust's chain with powf 1 ulp off somewhere: each level's bytes explained one by one.
                lo = chain_toks(env_chain(fmt, face, size, lambda b, e: ulp_step(powf_cr(b, e), -1)))
                hi = chain_toks(env_chain(fmt, face, size, lambda b, e: ulp_step(powf_cr(b, e), 1)))
                ok = all(g == w or g == x or g == y for g, w, x, y in zip(got, want, lo, hi))
                if ok:
                    fix = " ".join(("i:%d" % v) if not t.startswith("v:") else ("v:%d" % v)
                                   for t, v in zip(outs.split(" "), want))
        else:
            ok = False
            print(f"env: {fn}: no re-derivation in validate.py")
        if not ok:
            bad += 1
            print(f"env: {fn} {note}: re-derived outputs differ from the corpus")
        elif fix is not None:
            cr.append(f"{fn}\t{ins}\t{fix}\t{note}: sRGB encoded with powf correctly rounded, where glibc's is 1 ulp off")
    return bad, cr


def material_ref(c, rough, metallic, pad0=0.0):
    return list(c) + [metallic, rough, pad0, 0.0]


def f64_div(a, b):
    if b == 0.0:
        if a == 0.0 or math.isnan(a):
            return math.nan
        return math.copysign(math.inf, a) * math.copysign(1.0, b)
    return a / b


def ggx_g1(ndv, rough):
    r = rough + 1.0
    k = (r * r) / 8.0
    nd = min(max(ndv, 0.0), 1.0)
    return nd / ((nd * (1.0 - k) + k) + 1e-15)


def brdf_integrate(ndv0, rough, n):
    """prakash 1.2.0's integrate_brdf_lut, in Python's (glibc's) f64."""
    ndv = min(max(ndv0, 0.001), 1.0)
    v = [math.sqrt(1.0 - ndv * ndv), 0.0, ndv]
    scale = bias = 0.0
    inv = f64_div(1.0, float(n))
    for i in range(n):
        xi1 = i * inv
        xi2 = int(f"{i:032b}"[::-1], 2) * 2.3283064365386963e-10
        a = rough * rough
        a2 = a * a
        ct = math.sqrt((1.0 - xi1) / (1.0 + (a2 - 1.0) * xi1))
        st2 = 1.0 - ct * ct
        st = math.sqrt(st2) if st2 >= 0 else 0.0
        phi = math.tau * xi2
        h = [st * math.cos(phi), st * math.sin(phi), ct]
        vdh = max(v[0] * h[0] + v[1] * h[1] + v[2] * h[2], 0.0)
        l2 = 2.0 * vdh * h[2] - v[2]
        ndl, ndh = max(l2, 0.0), max(h[2], 0.0)
        if ndl > 0.0:
            g = ggx_g1(ndv, rough) * ggx_g1(ndl, rough)
            gvis = (g * vdh) / (ndh * ndv + 1e-15)
            x = 1.0 - vdh
            fc = x * ((x * x) * (x * x))
            scale += gvis * (1.0 - fc)
            bias += gvis * fc
    return scale * inv, bias * inv


def lut_q8(v):
    """`(v.clamp(0, 1) * 255.0).round() as u8` for an f64: halves away from zero, the fraction exact."""
    if math.isnan(v):
        return 0
    v = min(max(v, 0.0), 1.0) * 255.0
    r = math.floor(v)
    return int(r) + (1 if v - r >= 0.5 else 0)


def check_pbr():
    """Re-derives every `pbr_material` and `pbr_optics` row; returns the rows that differ."""
    bad = 0
    for name in ("pbr_material", "pbr_optics"):
        path = os.path.join(GOLDEN, name + ".tsv")
        if not os.path.exists(path):
            continue
        _, body = rows(path)
        for fn, _cls, ins, outs, note in body:
            rd = Rd(ins)
            if fn == "MaterialUniforms::default":
                ok = same(g_values(outs), [1.0, 1.0, 1.0, 1.0, 0.0, 0.5, 0.0, 0.0])
            elif fn in ("MaterialUniforms::dielectric", "MaterialUniforms::metal"):
                c, rough = rd.gs(), rd.g()
                ok = same(g_values(outs), material_ref(c, rough, 1.0 if fn.endswith("metal") else 0.0))
            elif fn == "MaterialUniforms::from_ior":
                c = rd.gs()
                ior = struct.unpack("<d", struct.pack("<Q", int(rd.tok("f"), 16)))[0]
                rough = rd.g()
                q = f64_div(1.0 - ior, 1.0 + ior)
                ok = same(g_values(outs), material_ref(c, rough, 0.0, r32(q * q)))
            elif fn == "generate_brdf_lut":
                size, n = rd.i(), rd.i()
                want = []
                for y in range(size):
                    rough = (y + 0.5) / size
                    for x in range(size):
                        sc, bi = brdf_integrate((x + 0.5) / size, rough, n)
                        want += [lut_q8(sc), lut_q8(bi), 0, 255]
                ok = i_values(outs) == want
            else:
                ok = False
                print(f"{name}: {fn}: no re-derivation in validate.py")
            if not ok:
                bad += 1
                print(f"{name}: {fn} {note}: re-derived outputs differ from the corpus")
    return bad


def cube_ref():
    faces = [((0, 0, 1), [(-1, -1, 1), (1, -1, 1), (1, 1, 1), (-1, 1, 1)]),
             ((0, 0, -1), [(1, -1, -1), (-1, -1, -1), (-1, 1, -1), (1, 1, -1)]),
             ((1, 0, 0), [(1, -1, 1), (1, -1, -1), (1, 1, -1), (1, 1, 1)]),
             ((-1, 0, 0), [(-1, -1, -1), (-1, -1, 1), (-1, 1, 1), (-1, 1, -1)]),
             ((0, 1, 0), [(-1, 1, 1), (1, 1, 1), (1, 1, -1), (-1, 1, -1)]),
             ((0, -1, 0), [(-1, -1, -1), (1, -1, -1), (1, -1, 1), (-1, -1, 1)])]
    uvs = [(0.0, 1.0), (1.0, 1.0), (1.0, 0.0), (0.0, 0.0)]
    vs, idx = [], []
    for n, ps in faces:
        base = len(vs)
        for p, uv in zip(ps, uvs):
            vs.append(vtx(tuple(0.5 * c for c in p), tuple(float(c) for c in n), uv, WHITE4))
        idx += [base, base + 1, base + 2, base + 2, base + 3, base]
    return vs, idx


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
                e_rows.append(f"{fn}\t{ins}\t{g_tokens(outs, once)}\t{note}: f64 once vs Rust f32")
                d += 1
            gains[fn] = (n + 1, d)

    failures += check_color()
    failures += check_lod()
    failures += check_color_optics()
    failures += check_json()
    failures += check_text()
    sides_e = []
    lbad, lcr = check_lights(plat)
    tbad, tcr = check_tables(plat)
    sbad, scr = check_sprite()
    dbad, dcr = check_debug("debug")
    cbad, ccr = check_debug("debug_collider")
    failures += lbad + tbad + sbad + dbad + cbad
    for gname in ("primitives", "viz", "terrain"):
        gbad, gerows = check_geom(gname, plat)
        failures += gbad
        sides_e.append((gname + ".e.tsv", "class-E rows (ADR 0002 #4, D4 rule b): Rust's formula in f64, rounded once to "
                        "f32, where that differs from Rust's f32 outputs", gerows))
    envbad, envcr = check_env()
    failures += envbad + check_pbr()
    shbad, sherows, shgain = check_shadow()
    failures += shbad
    sides_e.append(("shadow.e.tsv", "class-E rows (ADR 0002 #4, D4 rule b): Rust's formula in f64 (pow correctly "
                    "rounded), rounded once to f32, where that differs from Rust's f32 outputs", sherows))
    glbad, glcr, gltally = check_gltf()
    failures += glbad
    abad, aerows, again = check_anim(plat)
    failures += abad
    sides_e.append(("animation.e.tsv", "class-E rows (ADR 0002 #4, D4 rule b): Rust's formula in f64, rounded once to "
                    "f32, where that differs from Rust's f32 outputs", aerows))
    sides = sides_e + [
        ("lights.cr.tsv", "correctly rounded rows (ADR 0002 #16): Rust 1.2.0's outputs with each f32 cos correctly "
         "rounded (glibc's f64 cos, rounded once), where glibc's cosf differs", lcr),
        ("tables.cr.tsv", "correctly rounded rows (ADR 0002 #16): Rust 1.2.0's sRGB transfer functions with powf "
         "correctly rounded (glibc's f64 pow, rounded once), where glibc's powf differs by 1 ulp", tcr),
        ("sprite.cr.tsv", "correctly rounded rows (ADR 0002 #16): Rust 1.2.0's sprite quads with each rotation's sinf "
         "and cosf correctly rounded (glibc's f64 sin/cos, rounded once), where glibc's differ by 1 ulp", scr),
        ("debug.cr.tsv", "correctly rounded rows (ADR 0002 #16): Rust 1.2.0's debug circles with each angle's sinf and "
         "cosf correctly rounded (glibc's f64 sin/cos, rounded once), where glibc's differ by 1 ulp", dcr),
        ("debug_collider.cr.tsv", "correctly rounded rows (ADR 0002 #16): Rust 1.2.0's collider outlines with each "
         "angle's sinf and cosf correctly rounded, where glibc's differ by 1 ulp", ccr),
        ("env.cr.tsv", "correctly rounded rows (ADR 0002 #16): Rust 1.2.0's sRGB face encoding with powf correctly "
         "rounded (glibc's f64 pow, rounded once), where glibc's is 1 ulp off and a byte changes", envcr),
        ("gltf.cr.tsv", "correctly rounded rows (ADR 0002 #14): Rust 1.2.0's glTF animations with each JSON decimal read "
         "correctly rounded (the decimal's f64, narrowed), where serde_json misrounds", glcr),
    ]
    for name, what, side_rows in sides:
        side_path = os.path.join(GOLDEN, name)
        text = side_table(name, what, side_rows, plat)
        if WRITE:
            with open(side_path, "w", encoding="utf-8") as f:
                f.write(text)
            print(f"wrote {side_path} ({len(side_rows)} rows)")
        else:
            try:
                with open(side_path, encoding="utf-8") as f:
                    if f.read() != text:
                        failures += 1
                        print(f"{name}: stale (run with --write)")
            except FileNotFoundError:
                failures += 1
                print(f"{name}: missing (run with --write)")

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
    print("gltf: " + ", ".join(f"{v} {k}" for k, v in sorted(gltally.items())) + " (refusals and panics are the oracle's)")
    for fn, (n, d) in sorted(shgain.items()):
        print(f"shadow {fn:<30} class E rows {n:4d}: f64-once differs from Rust's f32 on {d:4d} ({100 * d / n:5.1f} %)")
    for fn, (n, d) in sorted(again.items()):
        print(f"animation {fn:<33} class E rows {n:4d}: f64-once differs from Rust's f32 on {d:4d} ({100 * d / n:5.1f} %)")

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
