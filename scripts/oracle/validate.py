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


def serde_color(doc):
    """What serde_json::from_str::<Color>(doc) gives: [r, g, b, a], or None for an error."""
    known = ("r", "g", "b", "a")

    def pairs(ps):
        seen = {}
        for k, val in ps:
            if k in known and k in seen:
                raise ValueError("duplicate field")
            seen[k] = val
        return seen

    def no_const(_c):
        raise ValueError("not JSON")
    try:
        v = _json.loads(doc, object_pairs_hook=pairs, parse_int=lambda t: _Num(t, True),
                        parse_float=lambda t: _Num(t, False), parse_constant=no_const)
    except (ValueError, RecursionError):
        return None
    if isinstance(v, dict):
        fields = [v.get(k) for k in known]
    elif isinstance(v, list) and len(v) == 4:
        fields = v
    else:
        return None
    out = [_serde_f32(f) for f in fields]
    return None if any(o is None for o in out) else out


def check_json():
    path = os.path.join(GOLDEN, "json.tsv")
    if not os.path.exists(path):
        return 0
    _, body = rows(path)
    bad = 0
    for fn, _cls, ins, outs, note in body:
        if fn.startswith("Color::from_json"):
            doc = re.sub(r"%([0-9A-F]{2})", lambda m: chr(int(m.group(1), 16)), ins[2:]).encode("latin-1").decode("utf-8")
            got = serde_color(doc)
            ok = outs == "E:Deserialize" if got is None else same(got, g_values(outs))
        else:
            ok = False
            print(f"json: {fn}: no re-derivation in validate.py")
        if not ok:
            bad += 1
            print(f"json: {fn} {note}: re-derived outputs differ from the corpus")
    return bad


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
    lbad, lcr = check_lights(plat)
    tbad, tcr = check_tables(plat)
    failures += lbad + tbad
    sides = [
        ("lights.cr.tsv", "correctly rounded rows (ADR 0002 #16): Rust 1.2.0's outputs with each f32 cos correctly "
         "rounded (glibc's f64 cos, rounded once), where glibc's cosf differs", lcr),
        ("tables.cr.tsv", "correctly rounded rows (ADR 0002 #16): Rust 1.2.0's sRGB transfer functions with powf "
         "correctly rounded (glibc's f64 pow, rounded once), where glibc's powf differs by 1 ulp", tcr),
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
