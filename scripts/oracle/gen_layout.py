#!/usr/bin/env python3
"""soorat's GPU layouts: generates src/layout.cyr and tests/layout_probe.cyr from one table (port plan §3.1, M1).

    python3 -I -B scripts/oracle/gen_layout.py .           # write both files
    python3 -I -B scripts/oracle/gen_layout.py . --check   # exit 1 if either is stale or the table disagrees

TABLE lists every GPU-facing `#[repr(C)]` type of Rust 1.2.0 with its fields' Cyrius types, as probe P1 found them
(port-parity §1.1: packed nested f32 structs reproduce all 217 offsets). Before writing anything the script lays the
table out (packed, no padding: every field is 4-byte aligned already) and checks every size and offset against
tests/golden/layout.tsv, the oracle's `size_of` / `offset_of` rows. The written files:
- **src/layout.cyr**: the struct declarations. Cyrius has no array fields, so `joints: [[f32; 16]; 128]` becomes the
  indexed fields `joints_0` … `joints_127` (scripts/surface_parity.py's FIELD_RENAMES lists them).
- **tests/layout_probe.cyr**: one function per type that measures it in Cyrius (P1's technique: a marker written
  into each field's first scalar, then found in the bytes), for tests/parity.tcyr's `layout` rows and
  tests/layout.tcyr. So the replay checks what the compiler lays out, not this table.
Stdlib only.
"""
import os
import sys

ROOT = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].startswith("-") else ".")
CHECK = "--check" in sys.argv

# The building blocks: Rust's [f32; 2|3|4], [u32; 4] and [[f32; 4]; 4] / [f32; 16].
BASE = [
    ("SrV2", [("x", "f32"), ("y", "f32")]),
    ("SrV3", [("x", "f32"), ("y", "f32"), ("z", "f32")]),
    ("SrV4", [("x", "f32"), ("y", "f32"), ("z", "f32"), ("w", "f32")]),
    ("SrU4", [("x", "u32"), ("y", "u32"), ("z", "u32"), ("w", "u32")]),
    ("SrM4", [("c0", "SrV4"), ("c1", "SrV4"), ("c2", "SrV4"), ("c3", "SrV4")]),
]

# (Rust type, its module, [(field, Cyrius type, array length or 0)]), in tests/golden/layout.tsv's order.
TABLE = [
    ("GpuParticle", "gpu_particles", [("position", "SrV4", 0), ("velocity", "SrV4", 0), ("color", "SrV4", 0)]),
    ("SimParams", "gpu_particles",
     [("delta_time", "f32", 0), ("gravity_y", "f32", 0), ("damping", "f32", 0), ("particle_count", "u32", 0)]),
    ("PostProcessUniforms", "postprocess", [("params", "SrV4", 0)]),
    ("JointUniforms", "animation", [("joint_count", "SrV4", 0), ("joints", "SrM4", 128)]),
    ("BloomUniforms", "hdr", [("params", "SrV4", 0), ("texel_size", "SrV4", 0)]),
    ("LineVertex", "debug_draw", [("position", "SrV3", 0), ("color", "SrV4", 0)]),
    ("GpuLight", "lights",
     [("position_type", "SrV4", 0), ("direction_range", "SrV4", 0), ("color_intensity", "SrV4", 0),
      ("spot_params", "SrV4", 0)]),
    ("LightArrayUniforms", "lights", [("ambient", "SrV4", 0), ("light_count", "SrV4", 0), ("lights", "SrGpuLight", 8)]),
    ("MaterialUniforms", "pbr_material",
     [("base_color_factor", "SrV4", 0), ("metallic", "f32", 0), ("roughness", "f32", 0), ("_pad0", "f32", 0),
      ("_pad1", "f32", 0)]),
    ("SsaoUniforms", "ssao", [("params", "SrV4", 0), ("projection", "SrM4", 0), ("inv_projection", "SrM4", 0)]),
    ("CameraUniforms", "mesh_pipeline",
     [("view_proj", "SrM4", 0), ("model", "SrM4", 0), ("camera_pos", "SrV4", 0), ("normal_matrix_0", "SrV4", 0),
      ("normal_matrix_1", "SrV4", 0), ("normal_matrix_2", "SrV4", 0)]),
    ("LightUniforms", "mesh_pipeline",
     [("ambient_color", "SrV4", 0), ("light_direction", "SrV4", 0), ("light_color", "SrV4", 0),
      ("light_view_proj", "SrM4", 0)]),
    ("ShadowPassUniforms", "mesh_pipeline", [("light_view_proj", "SrM4", 0), ("shadow_map_size", "SrV4", 0)]),
    ("ShadowUniforms", "shadow", [("light_view_proj", "SrM4", 0), ("model", "SrM4", 0)]),
    ("CascadeUniforms", "shadow", [("splits", "SrV4", 0), ("matrices", "SrM4", 4)]),
    ("Vertex2D", "vertex", [("position", "SrV2", 0), ("tex_coords", "SrV2", 0), ("color", "SrV4", 0)]),
    ("Vertex3D", "vertex",
     [("position", "SrV3", 0), ("normal", "SrV3", 0), ("tex_coords", "SrV2", 0), ("color", "SrV4", 0)]),
    ("SkinnedVertex3D", "vertex",
     [("position", "SrV3", 0), ("normal", "SrV3", 0), ("tex_coords", "SrV2", 0), ("color", "SrV4", 0),
      ("tangent", "SrV4", 0), ("joints", "SrU4", 0), ("weights", "SrV4", 0)]),
    ("InstanceData", "instancing", [("model", "SrM4", 0), ("color", "SrV4", 0)]),
]

SIZES = {"f32": 4, "u32": 4}


def size_of(t):
    if t in SIZES:
        return SIZES[t]
    for name, fields in BASE:
        if name == t:
            return sum(size_of(ft) for _, ft in fields)
    for name, _mod, fields in TABLE:
        if "Sr" + name == t:
            return sum(size_of(ft) * max(n, 1) for _, ft, n in fields)
    raise KeyError(t)


def first_scalar(t):
    """The path to a type's first scalar field (`.x`, `.c0.x`, or '' for a scalar) and that scalar's type."""
    if t in SIZES:
        return "", t
    for name, fields in BASE:
        if name == t:
            f, ft = fields[0]
            sub, st = first_scalar(ft)
            return "." + f + sub, st
    for name, _mod, fields in TABLE:
        if "Sr" + name == t:
            f, ft, n = fields[0]
            sub, st = first_scalar(ft)
            return "." + (f"{f}_0" if n else f) + sub, st
    raise KeyError(t)


def golden_rows():
    """{type: (size, [offsets])} from tests/golden/layout.tsv."""
    out = {}
    with open(os.path.join(ROOT, "tests", "golden", "layout.tsv"), encoding="utf-8") as f:
        for line in f:
            if line.startswith("#") or not line.strip():
                continue
            fn, _cls, _ins, outs, _note = line.rstrip("\n").split("\t")
            toks = outs.split(" ")
            out[fn.split("::")[1]] = (int(toks[0][2:]), [int(t[2:]) for t in toks[2:]])
    return out


def check_table():
    problems = []
    rows = golden_rows()
    names = [t for t, _m, _f in TABLE]
    if sorted(names) != sorted(rows):
        problems.append(f"TABLE has {sorted(set(names) ^ set(rows))} where layout.tsv differs")
    for name, _mod, fields in TABLE:
        if name not in rows:
            continue
        size, offs = rows[name]
        at, mine = 0, []
        for _f, ft, n in fields:
            mine.append(at)
            at += size_of(ft) * max(n, 1)
        if at != size or mine != offs:
            problems.append(f"{name}: the table lays out size {at}, offsets {mine}; layout.tsv says {size}, {offs}")
    return problems


def cyrius_fields(fields):
    out = []
    for f, ft, n in fields:
        if n:
            out += [f"{f}_{k}: {ft}" for k in range(n)]
        else:
            out.append(f"{f}: {ft}")
    return out


def wrap_struct(name, decls):
    """`struct Name { a: T; b: T; … }`, wrapped to 120 columns."""
    head = f"struct {name} {{"
    parts = [d + ";" for d in decls]
    line = head + " " + " ".join(parts) + " }"
    if len(line) <= 120:
        return [line]
    lines, cur = [head], "   "
    for p in parts:
        if len(cur) + 1 + len(p) > 120:
            lines.append(cur)
            cur = "   "
        cur += " " + p
    lines.append(cur)
    lines.append("}")
    return lines


def layout_cyr():
    out = [
        "# layout.cyr — the GPU-facing `#[repr(C)]` types of Rust 1.2.0 (M1 part b): GENERATED by",
        "# scripts/oracle/gen_layout.py from its table, which it checks against tests/golden/layout.tsv. Do not edit.",
        "#",
        "# Each type is a packed nested f32 struct with Rust's size and field offsets (probe P1, port-parity §1.1).",
        "# `[f32; 2 | 3 | 4]` is SrV2 / SrV3 / SrV4, `[u32; 4]` SrU4 and a 4×4 matrix SrM4 (four SrV4 columns). Cyrius",
        "# has no array fields, so an array field `f: [T; n]` is the indexed fields `f_0` … `f_<n−1>`. A vertex or",
        "# uniform buffer is bytes: write an f32 field with store32 at its offset (`sizeof(SrVertex3D)` is the stride);",
        "# tests/layout_probe.cyr measures every offset, and tests/parity.tcyr checks them against Rust's.",
        "",
    ]
    for name, fields in BASE:
        out += wrap_struct(name, [f"{f}: {t}" for f, t in fields])
    out.append("")
    for name, mod, fields in TABLE:
        out.append(f"# Rust `{mod}::{name}`: {size_of('Sr' + name)} bytes.")
        out += wrap_struct("Sr" + name, cyrius_fields(fields))
    return "\n".join(out) + "\n"


def probe_cyr():
    out = [
        "# layout_probe.cyr — measures each GPU type of src/layout.cyr in Cyrius: GENERATED by",
        "# scripts/oracle/gen_layout.py. Do not edit.",
        "#",
        "# Probe P1's technique: zero a value, write a distinct marker into each listed field's first scalar, and find",
        "# the marker's byte offset. `_lp_<Type>(out)` writes the size, then the offset of each field the golden row",
        "# lists (an array field: its first element), as 8-byte words at out, and returns the field count.",
        "",
        "# The offset of the first 4-byte window of [base, base + size) that holds marker, or -1.",
        "fn _lp_find(base, size, marker): i64 {",
        "    var i = 0;",
        "    while (i + 4 <= size) {",
        "        if (load32(base + i) == marker) { return i; }",
        "        i = i + 1;",
        "    }",
        "    return 0 - 1;",
        "}",
        "",
        "# The f32 bits of the integer k (a marker for an f32 field).",
        "fn _lp_mk(k): i64 { return f32_from(f64_from(k)); }",
        "",
    ]
    for name, _mod, fields in TABLE:
        t = "Sr" + name
        out += [f"# Measures `{t}`.", f"fn _lp_{name}(out): i64 {{", f"    var t: {t};", "    var base = &t;",
                f"    memset(base, 0, sizeof({t}));"]
        marks = []
        for k, (f, ft, n) in enumerate(fields):
            path, st = first_scalar(ft)
            field = f"{f}_0" if n else f
            mark = f"_lp_mk({k + 1})" if st == "f32" else str(1000 + k)
            out.append(f"    t.{field}{path} = {mark};")
            marks.append(mark)
        out.append(f"    store64(out, sizeof({t}));")
        for k, mark in enumerate(marks):
            out.append(f"    store64(out + {8 * (k + 1)}, _lp_find(base, sizeof({t}), {mark}));")
        out += [f"    return {len(fields)};", "}", ""]
    out += [
        "# Measures the type named by the golden row's fn (`layout::<Type>`) into out; returns its field count, or -1",
        "# for a type the table doesn't hold.",
        "fn _lp_measure(fp, fl, out): i64 {",
    ]
    for name, _mod, _fields in TABLE:
        out.append(f'    if (_lp_name_is(fp, fl, "layout::{name}") == 1) {{ return _lp_{name}(out); }}')
    out += ["    return 0 - 1;", "}", "",
            "# 1 when [p, p + len) is the C string s.",
            "fn _lp_name_is(p, len, s): i64 {",
            "    if (strlen(s) != len) { return 0; }",
            "    return memeq(p, s, len);",
            "}", ""]
    return "\n".join(out)


def main():
    problems = check_table()
    for p in problems:
        print("layout:", p)
    if problems:
        return 1
    outputs = {os.path.join(ROOT, "src", "layout.cyr"): layout_cyr(),
               os.path.join(ROOT, "tests", "layout_probe.cyr"): probe_cyr()}
    stale = 0
    for path, text in outputs.items():
        rel = os.path.relpath(path, ROOT)
        if CHECK:
            try:
                with open(path, encoding="utf-8") as f:
                    if f.read() != text:
                        stale += 1
                        print(f"{rel}: stale (run scripts/oracle/gen_layout.py . to regenerate)")
            except FileNotFoundError:
                stale += 1
                print(f"{rel}: missing")
        else:
            with open(path, "w", encoding="utf-8") as f:
                f.write(text)
            print(f"wrote {rel}")
    print("gen_layout:", "ok" if stale == 0 else f"{stale} stale file(s)",
          f"({len(TABLE)} types checked against tests/golden/layout.tsv)")
    return 1 if stale else 0


if __name__ == "__main__":
    sys.exit(main())
