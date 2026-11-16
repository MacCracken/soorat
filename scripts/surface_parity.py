"""Surface parity: every public item of the Rust 1.2.0 oracle has a Cyrius counterpart, or a reason.

Port plan §4 step 7 (the M0 tooling) and §7 (a gate at the end of every milestone; M10 is the full surface diff).
Adapted from pravash's scripts/surface_parity.py (2.0.0; itself impetus's, kimiya's and tanmatra's) and bijli's
M0 form of it (a milestone per module, everything pending until its milestone is done). Usage, from the repo root:

    python3 -I -B scripts/surface_parity.py .             # summary by module and status
    python3 -I -B scripts/surface_parity.py . --check     # the gate: exit 1 on a problem (below)
    python3 -I -B scripts/surface_parity.py . --list color math_util   # every item of those modules, with its name
    python3 -I -B scripts/surface_parity.py . --done M1   # preview: also treat M1 as done

**What it reads.** Every file under rust-old/src, outside its `#[cfg(test)]` modules (skipped by brace depth,
wherever they sit) and skipping an item whose own attributes include `#[cfg(test)]`. A public item is one declared
`pub` (not `pub(crate)` / `pub(super)`): `pub fn`, `pub struct` and its `pub` fields, `pub enum` and its variants,
`pub const`, `pub type`, `pub trait` and its methods, the `pub fn`s and `pub const`s of `impl` blocks of public types,
the trait impls of public types, a module's `pub use` re-exports, and lib.rs's re-exports. `math_util` and `viz` are
`pub(crate)` modules, but their `pub` items are public within them and §3.1 counts them: they are included.

**The Cyrius name** each one must have (port plan §9.3, §2.5: `Sr` types, `sr_` functions, `SR_` constants,
`SR_ERR_*` error codes, and never `soorat_`):
- `pub struct S`, `pub enum E`, `pub trait T`, `pub type A`   -> `struct SrS` / `enum SrE` / `trait SrT` / a type `SrA`
- each `pub` field f of S                                       -> a field f of `struct SrS` (or FIELD_RENAMES' fields)
- each variant V of E                                           -> `SR_<SNAKE(E)>_<SNAKE(V)>` (ENUM_PREFIX: `RenderError`
                                                                   -> `SR_ERR_<SNAKE(V)>`), an enum member or a const
- free `pub fn f`                                               -> `fn sr_f(`
- `impl T { pub fn m(..) }`, with or without self              -> `SrT_m`: `fn m(` inside `impl SrT {` (Cyrius mangles it
                                                                   to `SrT_m`), or a free `fn SrT_m(`
- `pub trait T { fn m }`                                        -> `fn m` declared in `trait SrT {`, or `fn SrT_m(`
- free `pub const X`; `impl T { pub const X }`                  -> `SR_X`; `SR_<SNAKE(T)>_X` (a const, a global or a member)
- `impl Default for T`                                          -> `SrT_default`; any other trait impl needs a
                                                                   TRAIT_RENAMES or EXCEPTIONS entry once its milestone is done
- `pub use <crate>::{A, f}` of another crate (mabda)            -> `SrA` / `sr_f`, as if soorat defined them
- `pub use` of a soorat item (in lib.rs or a module)            -> the source item's name and status
A name the rules give otherwise goes in TYPE_RENAMES, FN_RENAMES, FIELD_RENAMES or TRAIT_RENAMES.

**Milestones.** Every rust-old/src file has a milestone (MODULE_MILESTONE, from plan §3.1), is not carried
(NOT_CARRIED_MODULES, §3.2) or is not surface (NOT_SURFACE_MODULES). SPLITS fixes, item by item, the modules §3.1
splits between two milestones (§3.1 asks this script to fix the estimates at M0), and every `#[repr(C)]` GPU-layout
struct with its fields belongs to M1 (§3.1: "the 15 `#[repr(C)]` types", `src/layout.cyr`). **A milestone is done when
its row in the plan's §14 Tracking table has ✅ in its Status cell**: that table is the one machine-readable source
every tool in scripts/ reads (`--done` adds milestones for a preview run). An item is
- **ported** when src/*.cyr defines its Cyrius name (at any milestone, early or not);
- **pending** when it isn't and its milestone is not done;
- **missing** when it isn't and its milestone is done (a problem);
- **n/a** when EXCEPTIONS gives the reason it is not carried (§3.2, ADR 0002), or its module is not carried;
- **deferred** when DEFERRED gives the reason it waits past 2.0.0 (an upstream request, D7, ...).

**Problems** (`--check` exits 1 on any): a missing item; a rust-old/src file with no classification; a table entry
(MODULE_MILESTONE, SPLITS, EXCEPTIONS, DEFERRED, *_RENAMES) that names no Rust file or item, or an EXCEPTIONS /
DEFERRED entry whose item is in fact ported (drop the entry); a `pub use` that names no public item; a milestone
label that isn't M1–M10. Stdlib only.
"""
import argparse
import re
import sys
from pathlib import Path

PLAN = "docs/development/soorat-port-plan.md"
MILESTONES = [f"M{n}" for n in range(0, 11)]  # M0 (prepare and initialize) ports no items

# Rust module (rust-old/src/<module>.rs) -> the milestone that ports it: port plan §3.1. A module §3.1 splits has its
# default here and its other part in SPLITS.
MODULE_MILESTONE = {
    # M1 Foundation
    "error": "M1", "math_util": "M1", "color": "M1", "vertex": "M1", "instancing": "M1",
    "lights": "M1", "lod": "M1",
    # material: a Material owns a GPU texture and bind group (texture.rs, M6), so it ports with them, not in M1
    # as §3.1 first placed it (moved at M1 part c).
    "material": "M6",
    # M2 2D (debug_draw's line pipeline is M6: SPLITS)
    "sprite": "M2", "batch": "M2", "text": "M2", "ui": "M2", "debug_draw": "M2",
    # M3 Geometry and animation
    "primitives": "M3", "terrain": "M3", "viz": "M3", "animation": "M3", "gltf_loader": "M3",
    # M4 Lighting math and infrastructure (shadow's maps and passes are M7: SPLITS)
    "shadow": "M4", "pbr_material": "M4", "render_graph": "M4", "profiler": "M4", "capabilities": "M4",
    # M5 Science visualization. em_render: D7 — if bijli 2.0 (its M7) hasn't shipped when M5 closes, move it to
    # DEFERRED_MODULES with that reason, and it follows in 2.1.0.
    "acoustic_render": "M5", "thermo_render": "M5", "fluid_render": "M5", "em_render": "M5",
    # M6 GPU core. gpu.rs is in no §3.1 row: it only re-exports mabda's GpuContext, which EXCEPTIONS covers
    # (ADR 0002 #2, #3); M6, where the GPU work starts, is its milestone.
    "texture": "M6", "render_target": "M6", "compute": "M6", "pipeline": "M6", "mesh_pipeline": "M6", "gpu": "M6",
    # M7 Shadows, IBL, PBR. environment's CPU half (§3.1's M4 row: f16, sRGB, downsample, mips) is all `pub(crate)`,
    # so not surface: the golden corpus checks it through the oracle hooks. Its public items are the GPU half.
    "environment": "M7",
    # M8 Post, particles, screenshots
    "hdr": "M8", "postprocess": "M8", "ssao": "M8", "gpu_particles": "M8", "screenshot": "M8",
    # M9 Integration (window's winit half is not carried: EXCEPTIONS)
    "window": "M9", "drm": "M9",
}
# Modules §3.2 doesn't carry: counted as n/a, never checked.
NOT_CARRIED_MODULES = {
    "egui_bridge": "Cyrius has no egui (D8, §3.2, ADR 0002 #10)",
    "oracle": "the hidden `oracle` hooks of the frozen crate: Rust-only port tooling (D1, §4 step 0)",
}
# Files that hold no surface of their own.
NOT_SURFACE_MODULES = {
    "lib": "the crate root: its `pub use` re-exports are checked, as the items they name",
    "test_support": "`#[cfg(test)]` GLB fixtures; they port with the gltf tests (M3)",
}
# Modules whose milestone waits past 2.0.0: module -> the reason (D7's em_render may land here).
DEFERRED_MODULES = {}
# The §3.1 splits, item by item: module -> {key: milestone}. A key is a type (its fields, variants, methods, consts
# and trait impls with it), `Type::member`, or a free fn or const. Fixed here at M0, as §3.1 asks.
SPLITS = {
    # debug_draw: the geometry half (LineBatch, LineVertex's layout) is M2; the line pipeline and the wgpu vertex
    # layout descriptor are M6 (MB1).
    "debug_draw": {"LinePipeline": "M6", "LineVertex::layout": "M6"},
    # shadow: matrices, splits, atlas tiling, point-light faces and the uniforms' defaults are CPU (M4); the depth
    # maps, the atlas texture and the pass are GPU (M7, MB2). CascadedShadowMap owns its maps, so it is M7 whole.
    "shadow": {"ShadowMap": "M7", "ShadowPipeline": "M7", "CascadedShadowMap": "M7", "ShadowAtlas": "M7"},
    # error: `From<mabda::GpuError>` arrives with mabda, a dependency from M6 (`sr_error_from_gpu`).
    "error": {"RenderError::impl From<mabda::GpuError> for RenderError": "M6"},
    # instancing: InstanceBuffer is mabda's GPU buffer object (M6, with mabda); InstanceData is a layout (M1).
    "instancing": {"InstanceBuffer": "M6"},
    # terrain: the CPU mesh (generate_terrain, flat_heightmap, TerrainConfig, TerrainData) is M3;
    # create_terrain_mesh uploads it with mabda (M6).
    "terrain": {"create_terrain_mesh": "M6"},
    # gltf_loader: the CPU loaders (load_gltf_meshes, MeshData) are M3; Model and load_model own GPU meshes and textures
    # (mabda), M6.
    "gltf_loader": {"Model": "M6", "load_model": "M6"},
}
# Every `#[repr(C)]` struct and its fields: §3.1's M1 row ("the 15 `#[repr(C)]` types", src/layout.cyr, probe P1).
# Its methods and trait impls stay with its module.
LAYOUT_MILESTONE = "M1"

# Items not carried (§3.2, ADR 0002): "<module>::<item>" or "<module>::<Type>::<member>" -> why. A type's key covers
# its members. An entry that names no Rust item, or whose item is ported after all, is a problem.
EXCEPTIONS = {
    "error::type Result": "the stdlib's value-form Result, its Err payload an SR_ERR_* code (ADR 0002 #1)",
    "gpu::GpuContext": "mabda's GpuContext, which the application creates and passes in (ADR 0002 #2, #3, D5); "
                       "Cyrius has one namespace, so soorat re-exports nothing",
    "gpu::GpuContextBuilder": "mabda's context builder: the application builds the context (ADR 0002 #2, #3, D5)",
    "capabilities::webgpu": "a re-exported mabda module, not an item: Cyrius has no module paths, and its functions "
                            "are mabda's own (ADR 0002 #3)",
    "window::Window::new": "winit window creation and its GPU context (§3.2, D5, ADR 0002 #2): the application owns "
                           "both and calls `new_with_gpu`'s counterpart",
    "window::Window::winit_window": "the winit window (§3.2, D5): the application owns the window",
    "window::Window::request_redraw": "winit's redraw request (§3.2, D5): the application owns the event loop",
    "window::run": "the winit event loop (§3.2, D5, ADR 0002 #2)",
    "window::WindowConfig::fullscreen": "`WindowConfig.fullscreen` is not carried (§3.2, D5): a window property the "
                                        "application sets",
}
# Items that wait past 2.0.0: key as EXCEPTIONS -> why, and what brings them.
DEFERRED = {
    "drm::list_render_nodes": "agnodrm 1.8.1 has no render-node query: the reduced drm subset (D8, ADR 0002 #11) "
                              "until L10 lands",
    "drm::list_connectors": "agnodrm 1.8.1 has no connector query (D8, ADR 0002 #11): until L10 lands",
    "drm::ConnectorSummary": "the connector query's result type (D8, ADR 0002 #11): until L10 lands",
}

# A variant's constant prefix, where it isn't SR_<SNAKE(enum)>_.
ENUM_PREFIX = {"RenderError": "SR_ERR_"}
# Rust type -> its Cyrius name, where it isn't Sr<Name>.
TYPE_RENAMES = {}
# Rust fn (`f` or `Type::m`) -> its Cyrius name, where it isn't sr_f / SrType_m.
FN_RENAMES = {}
# (Rust type, field) -> the Cyrius fields that carry it.
FIELD_RENAMES = {
    # Cyrius has no array fields: `f: [T; n]` is the indexed fields f_0 … f_<n−1> (src/layout.cyr, generated).
    ("JointUniforms", "joints"): [f"joints_{k}" for k in range(128)],
    ("LightArrayUniforms", "lights"): [f"lights_{k}" for k in range(8)],
    ("CascadeUniforms", "matrices"): [f"matrices_{k}" for k in range(4)],
    ("PointShadowMap", "face_matrices"): [f"face_matrices_{k}" for k in range(6)],
}
# A trait impl's header as printed (`impl From<[f32; 4]> for Color`) -> the Cyrius function that provides it.
TRAIT_RENAMES = {
    "impl From<[f32; 4]> for Color": "SrColor_from_array4",
    "impl From<[f32; 3]> for Color": "SrColor_from_array3",
    "impl From<Color> for [f32; 4]": "SrColor_to_array",
    "impl From<mabda::GpuError> for RenderError": "sr_error_from_gpu",
}


def snake(camel):
    """`ToneMapMode` -> `TONE_MAP_MODE`, `AcesFilmic` -> `ACES_FILMIC`, `Rgba16Float` -> `RGBA16_FLOAT`."""
    out = []
    for k, ch in enumerate(camel):
        if ch.isupper() and k > 0:
            prev = camel[k - 1]
            nxt = camel[k + 1] if k + 1 < len(camel) else ""
            if prev.islower() or ((prev.isupper() or prev.isdigit()) and nxt.islower()):
                out.append("_")
        out.append(ch.upper())
    return "".join(out)


def milestones_done(root, extra):
    """The milestones whose plan §14 Tracking row has ✅ in its Status cell, plus `extra`; and a problem, if any."""
    text = (root / PLAN).read_text(encoding="utf-8")
    at = text.find("\n## 14. Tracking")
    if at < 0:
        return set(extra), [f"{PLAN} has no '## 14. Tracking' table, so no milestone reads as done"]
    done = set()
    for line in text[at + 1:].splitlines()[1:]:
        if line.startswith("## "):
            break
        m = re.match(r"^\| (M\d+)\b[^|]*\| ([^|]*)\|", line)
        if m and "✅" in m.group(2):
            done.add(m.group(1))
    return done | set(extra), []


# ── reading the Rust source ──────────────────────────────────────────────────────────────────────────────────────


def library_lines(text):
    """The (line number, text) pairs outside every `#[cfg(test)] mod … { … }` block (brace depth, wherever it sits)."""
    lines = text.split("\n")
    out, i = [], 0
    while i < len(lines):
        if lines[i].strip() == "#[cfg(test)]" and i + 1 < len(lines) and re.match(r"\s*(pub(\(\w+\))? )?mod \w+\s*\{",
                                                                                 lines[i + 1]):
            depth, j = 0, i + 1
            while j < len(lines):
                depth += lines[j].count("{") - lines[j].count("}")
                if depth <= 0:
                    break
                j += 1
            i = j + 1
            continue
        out.append((i + 1, lines[i]))
        i += 1
    return out


def use_names(stmt):
    """[(exported name, source path)] of a `pub use a::b::{C, d as e};` statement (one level of braces)."""
    body = stmt.strip()
    body = re.sub(r"^pub use\s+", "", body).rstrip(";").strip()
    out = []
    m = re.match(r"^(.*?)::\{(.*)\}$", body, re.S)
    if m:
        prefix, inner = m.group(1), m.group(2)
        parts = [p.strip() for p in inner.replace("\n", " ").split(",") if p.strip()]
    else:
        prefix, _, last = body.rpartition("::")
        parts = [last]
    for p in parts:
        src, _, alias = p.partition(" as ")
        path = f"{prefix}::{src.strip()}" if prefix else src.strip()
        out.append(((alias or src).strip().split("::")[-1], path))
    return out


def rust_items(root, module):
    """(kind, owner, name, info) for each public item of rust-old/src/<module>.rs. kind is one of struct, enum, trait,
    type, field, variant, fn, trait fn, const, impl, use. For `use`, info is the source path; for `impl`, the header;
    for struct, whether it is `#[repr(C)]`."""
    text = (root / "rust-old/src" / f"{module}.rs").read_text(encoding="utf-8")
    lines = library_lines(text)
    items = []
    public_types = set()
    for _, line in lines:
        m = re.match(r"pub (?:struct|enum|trait|type) (\w+)", line)
        if m:
            public_types.add(m.group(1))
    ctx, owner = None, None  # ctx: struct / enum / trait / impl / mod / private / None; owner: its public name
    attrs = []
    k = 0
    while k < len(lines):
        no, line = lines[k]
        s = line.strip()
        k += 1
        if s.startswith("#[") or s.startswith("#!["):
            attrs.append(s)
            continue
        if s.startswith("//") or not s:
            continue
        test_only = "#[cfg(test)]" in attrs
        repr_c = any(a.startswith("#[repr(C") for a in attrs)
        attrs = []
        col0 = not line[:1].isspace()
        if col0 and s.startswith("}"):
            ctx, owner = None, None
            continue
        if col0:
            if test_only:
                # A test-only item at the top level: skip its body.
                if s.endswith("{") or ("{" in s and "}" not in s):
                    ctx, owner = "private", None
                continue
            if (m := re.match(r"pub use\b", s)):
                stmt = s
                while not stmt.rstrip().endswith(";") and k < len(lines):
                    stmt += " " + lines[k][1].strip()
                    k += 1
                for name, path in use_names(stmt):
                    items.append(("use", None, name, path))
                continue
            if (m := re.match(r"impl(?:<[^>]*>)?\s+(.+?)\s+for\s+(.+?)\s*(?:where\b.*)?\{\s*$", s)):
                trait, target = m.group(1), m.group(2)
                tname = re.match(r"[\w:]+", target)
                tname = tname.group(0).split("::")[-1] if tname else ""
                header = f"impl {trait} for {target}"
                # `impl From<Color> for [f32; 4]` belongs to Color: the public type it converts from.
                if tname not in public_types:
                    src = re.findall(r"\b([A-Z]\w*)\b", trait)
                    tname = next((t for t in src if t in public_types), "")
                if tname in public_types:
                    items.append(("impl", tname, header, header))
                ctx, owner = ("impl", tname) if tname in public_types else ("private", None)
                continue
            if (m := re.match(r"impl(?:<[^>]*>)?\s+([\w:]+)(?:<[^>]*>)?\s*\{", s)):
                tname = m.group(1).split("::")[-1]
                ctx, owner = ("impl", tname) if tname in public_types else ("private", None)
                continue
            if (m := re.match(r"pub struct (\w+)", s)):
                items.append(("struct", None, m.group(1), repr_c))
                ctx, owner = ("struct", m.group(1)) if s.endswith("{") else (None, None)
                continue
            if (m := re.match(r"pub enum (\w+)", s)):
                items.append(("enum", None, m.group(1), None))
                ctx, owner = "enum", m.group(1)
                continue
            if (m := re.match(r"pub trait (\w+)", s)):
                items.append(("trait", None, m.group(1), None))
                ctx, owner = "trait", m.group(1)
                continue
            if (m := re.match(r"pub mod (\w+)\s*\{", s)):
                ctx, owner = "mod", None
                continue
            if (m := re.match(r"pub type (\w+)", s)):
                items.append(("type", None, m.group(1), None))
                continue
            if (m := re.match(r"pub (?:const |async |unsafe )*fn (\w+)", s)):
                items.append(("fn", None, m.group(1), None))
                ctx, owner = ("private", None) if not s.endswith(";") else (None, None)
                continue
            if (m := re.match(r"pub const (\w+)\s*:", s)):
                items.append(("const", None, m.group(1), None))
                continue
            # Anything else at column 0 (a private fn, struct, enum, impl, macro …): its body is not surface.
            if s.endswith("{") or s.endswith("("):
                ctx, owner = "private", None
            continue
        # Indented lines: members of the current block.
        indent = len(line) - len(line.lstrip(" "))
        if test_only:
            continue
        if ctx == "struct" and indent == 4 and (m := re.match(r"pub (\w+)\s*:", s)):
            items.append(("field", owner, m.group(1), None))
        elif ctx == "enum" and indent == 4 and (m := re.match(r"([A-Z]\w*)\s*(?:[,({=]|$)", s)):
            items.append(("variant", owner, m.group(1), None))
        elif ctx == "trait" and indent == 4 and (m := re.match(r"fn (\w+)", s)):
            items.append(("trait fn", owner, m.group(1), None))
        elif ctx == "impl" and indent == 4 and (m := re.match(r"pub (?:const |async |unsafe )*fn (\w+)", s)):
            items.append(("fn", owner, m.group(1), None))
        elif ctx == "impl" and indent == 4 and (m := re.match(r"pub const (\w+)\s*:", s)):
            items.append(("const", owner, m.group(1), None))
        elif ctx == "mod" and indent == 4 and (m := re.match(r"pub (?:const |async |unsafe )*fn (\w+)", s)):
            items.append(("fn", None, m.group(1), None))
        elif ctx == "mod" and indent == 4 and (m := re.match(r"pub const (\w+)\s*:", s)):
            items.append(("const", None, m.group(1), None))
    return items


# ── reading the Cyrius source ────────────────────────────────────────────────────────────────────────────────────


def strip_code(line):
    """A Cyrius line without its string literals and `#` comment (directives start with `#` too: not definitions)."""
    line = re.sub(r'"(?:[^"\\]|\\.)*"', '""', line)
    line = re.sub(r"'(?:[^'\\]|\\.)*'", "''", line)
    at = line.find("#")
    return line if at < 0 else line[:at]


def cyrius_index(root):
    """What src/**/*.cyr defines: fns (an impl method as `Type_m`, a trait method as `Trait_m`), values (consts,
    top-level globals, enum members), types (structs, enums, traits) and each struct's fields."""
    fns, values, types, fields = set(), set(), set(), {}
    for path in sorted((root / "src").rglob("*.cyr")):
        lines = [strip_code(ln) for ln in path.read_text(encoding="utf-8").split("\n")]
        block, bname, btrait, depth = None, None, None, 0
        for line in lines:
            s = line.strip()
            s_def = re.sub(r"^(?:pub|public)\s+", "", s)
            if block is None:
                if (m := re.match(r"impl\s+(\w+)(?:\s+for\s+(\w+))?\s*\{", s_def)):
                    block, depth = "impl", 0
                    bname, btrait = (m.group(2), m.group(1)) if m.group(2) else (m.group(1), None)
                elif (m := re.match(r"trait\s+(\w+)\s*\{", s_def)):
                    block, bname, depth = "trait", m.group(1), 0
                    types.add(m.group(1))
                elif (m := re.match(r"struct\s+(\w+)(?:<[^>]*>)?\s*\{", s_def)):
                    block, bname, depth = "struct", m.group(1), 0
                    types.add(m.group(1))
                    fields.setdefault(m.group(1), set())
                elif (m := re.match(r"struct\s+(\w+)", s_def)):
                    types.add(m.group(1))
                    fields.setdefault(m.group(1), set())
                elif (m := re.match(r"enum\s+(\w+)(?:\s*:\s*\w+)?\s*\{", s_def)):
                    block, bname, depth = "enum", m.group(1), 0
                    types.add(m.group(1))
                elif (m := re.match(r"fn\s+(\w+)", s_def)):
                    fns.add(m.group(1))
                elif (m := re.match(r"const\s+(\w+)", s_def)):
                    values.add(m.group(1))
                elif not line[:1].isspace() and (m := re.match(r"var\s+(\w+)", s_def)):
                    values.add(m.group(1))
            if block is None:
                continue
            # Inside a block: its members, then its braces.
            body = line
            if block in ("struct", "enum") and depth == 0 and "{" in body:
                body = body[body.index("{") + 1:]
            if block == "impl" and (m := re.match(r"(?:pub\s+|public\s+)?fn\s+(\w+)", s)):
                fns.add(f"{bname}_{m.group(1)}")
                if btrait:
                    fns.add(f"{bname}_{btrait}_{m.group(1)}")
            elif block == "trait" and (m := re.match(r"(?:pub\s+|public\s+)?fn\s+(\w+)", s)) and depth >= 1:
                fns.add(f"{bname}_{m.group(1)}")
            elif block == "struct":
                for part in re.split(r"[;\n]", body.split("}")[0]):
                    if (m := re.match(r"\s*(\w+)\s*(?::|$)", part)) and m.group(1):
                        fields[bname].add(m.group(1))
            elif block == "enum":
                for part in body.split("}")[0].split(","):
                    if (m := re.match(r"\s*([A-Za-z_]\w*)", part)):
                        values.add(m.group(1))
            depth += line.count("{") - line.count("}")
            if depth <= 0 and "}" in line:
                block = None
    return fns, values, types, fields


# ── the check ────────────────────────────────────────────────────────────────────────────────────────────────────


def type_name(name):
    return TYPE_RENAMES.get(name, f"Sr{name}")


def expected(kind, owner, name, info):
    """(index, wanted) for an item: the Cyrius name the rules give, and which index holds it; None when no rule."""
    if kind in ("struct", "enum", "trait", "type"):
        return "type", type_name(name)
    if kind == "field":
        return "field", (type_name(owner), FIELD_RENAMES.get((owner, name), [name]))
    if kind == "variant":
        return "value", ENUM_PREFIX.get(owner, f"SR_{snake(owner)}_") + snake(name)
    if kind == "fn":
        key = f"{owner}::{name}" if owner else name
        if key in FN_RENAMES:
            return "fn", FN_RENAMES[key]
        return "fn", f"{type_name(owner)}_{name}" if owner else f"sr_{name}"
    if kind == "trait fn":
        return "fn", f"{type_name(owner)}_{name}"
    if kind == "const":
        return "value", f"SR_{snake(owner)}_{name}" if owner else f"SR_{name}"
    if kind == "impl":
        if info in TRAIT_RENAMES:
            return "fn", TRAIT_RENAMES[info]
        if re.match(r"impl (?:\w+::)*Default for ", info):
            return "fn", f"{type_name(owner)}_default"
        return None, None
    if kind == "use":
        return ("type", type_name(name)) if name[:1].isupper() else ("fn", f"sr_{name}")
    return None, None


def label(module, kind, owner, name):
    if kind == "impl":
        return f"{module}::{name}"
    if kind == "type":
        return f"{module}::type {name}"
    return f"{module}::{owner + '::' if owner else ''}{name}"


def table_hit(table, module, kind, owner, name):
    """The key of `table` that covers this item: its own label, or its owner type's."""
    own = label(module, kind, owner, name)
    if own in table:
        return own
    if owner and f"{module}::{owner}" in table:
        return f"{module}::{owner}"
    return None


def split_milestone(module, kind, owner, name, info):
    """The milestone of one item of a module: SPLITS, then the layout rule, then the module's."""
    split = SPLITS.get(module, {})
    if owner and f"{owner}::{name}" in split:
        return split[f"{owner}::{name}"], f"{owner}::{name}"
    if owner and owner in split:
        return split[owner], owner
    if not owner and name in split:
        return split[name], name
    return None, None


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("root", nargs="?", default=".", help="the repository root (default: .)")
    ap.add_argument("--check", action="store_true", help="exit 1 on any problem (the CI gate)")
    ap.add_argument("--list", nargs="*", metavar="MODULE",
                    help="list every item (of the named modules, or all) with its milestone, Cyrius name and status")
    ap.add_argument("--done", default="", metavar="M1,M2",
                    help="also treat these milestones as done (a preview of their gate)")
    args = ap.parse_args()
    root = Path(args.root).resolve()

    problems = []
    extra = {m.strip() for m in args.done.split(",") if m.strip()}
    for m in sorted(extra - set(MILESTONES)):
        problems.append(f"--done names {m}, which is no milestone (M1–M10)")
    done, why = milestones_done(root, extra & set(MILESTONES))
    problems += why

    files = sorted(str(p.relative_to(root / "rust-old/src").with_suffix("")) for p in (root / "rust-old/src").rglob("*.rs"))
    classified = set(MODULE_MILESTONE) | set(NOT_CARRIED_MODULES) | set(NOT_SURFACE_MODULES) | set(DEFERRED_MODULES)
    for f in files:
        if f not in classified:
            problems.append(f"rust-old/src/{f}.rs has no milestone (MODULE_MILESTONE) and no other classification")
    for table, tname in ((MODULE_MILESTONE, "MODULE_MILESTONE"), (NOT_CARRIED_MODULES, "NOT_CARRIED_MODULES"),
                         (NOT_SURFACE_MODULES, "NOT_SURFACE_MODULES"), (DEFERRED_MODULES, "DEFERRED_MODULES"),
                         (SPLITS, "SPLITS")):
        for mod in table:
            if mod not in files:
                problems.append(f"{tname} names {mod}, which is no file in rust-old/src")
    for mod, ms in list(MODULE_MILESTONE.items()) + [(f"{m} split", v) for m, s in SPLITS.items() for v in s.values()]:
        if ms not in MILESTONES:
            problems.append(f"{mod}: milestone {ms} is not one of M1–M10")
    multi = [m for m in classified if sum(m in t for t in (MODULE_MILESTONE, NOT_CARRIED_MODULES,
                                                            NOT_SURFACE_MODULES, DEFERRED_MODULES)) > 1]
    for m in multi:
        problems.append(f"{m} is classified twice")

    fns, values, types, fields = cyrius_index(root)
    index = {"fn": fns, "value": values, "type": types}

    def found(where, want):
        if where == "field":
            return want[0] in fields and all(f in fields[want[0]] for f in want[1])
        if where == "value":
            return want in values or want in fns
        return where is not None and want in index[where]

    # Every module's items, so a `pub use` of a soorat item can be resolved to its source.
    all_items = {m: rust_items(root, m) for m in files if m in classified and m not in NOT_SURFACE_MODULES
                 or m == "lib"}
    used = {"EXCEPTIONS": set(), "DEFERRED": set(), "SPLITS": set(), "FN_RENAMES": set(), "FIELD_RENAMES": set(),
            "TRAIT_RENAMES": set(), "TYPE_RENAMES": set()}
    status_of = {}  # (module, kind, owner, name) -> (milestone, cyrius name, status, note)

    def module_ms(module):
        if module in NOT_CARRIED_MODULES:
            return "—"
        if module in DEFERRED_MODULES:
            return "deferred"
        return MODULE_MILESTONE.get(module, "—")

    def classify(module, item, stack=()):
        kind, owner, name, info = item
        key = (module, kind, owner, name)
        if key in status_of:
            return status_of[key]
        if module in NOT_CARRIED_MODULES:
            where, want = expected(kind, owner, name, info)
            shown = want if where != "field" else f"{want[0]}.{'/'.join(want[1])}"
            status_of[key] = ("—", shown or "—", "n/a", NOT_CARRIED_MODULES[module])
            return status_of[key]
        if kind == "use" and re.match(r"(crate|super|self)::", info) or (kind == "use" and module == "lib"):
            # A soorat item re-exported: its source's name and status.
            path = re.sub(r"^(crate|self)::", "", info)
            if path.startswith("super::"):
                path = path[len("super::"):]
            src_mod, _, src_name = path.rpartition("::")
            src_mod = src_mod.split("::")[0] if src_mod else module
            if src_mod not in all_items:
                res = ("—", "?", "problem", f"re-export of {info}: rust-old/src/{src_mod}.rs is not surface")
            else:
                src = next((it for it in all_items[src_mod] if it[2] == src_name and it[1] is None
                            and it[0] in ("struct", "enum", "trait", "type", "fn", "const", "use")), None)
                if src is None or (src_mod, src[0], src[1], src[2]) in stack:
                    res = ("—", "?", "problem", f"re-export of {info}: {src_mod} has no public item {src_name}")
                else:
                    ms, want, st, note = classify(src_mod, src, stack + (key,))
                    res = (ms, want, st, f"re-export of {src_mod}::{src_name}" + (f"; {note}" if note else ""))
            status_of[key] = res
            return res
        where, want = expected(kind, owner, name, info)
        shown = want if where != "field" else f"{want[0]}.{'/'.join(want[1])}"
        if module in NOT_CARRIED_MODULES:
            res = ("—", shown or "—", "n/a", NOT_CARRIED_MODULES[module])
        elif module in DEFERRED_MODULES:
            res = ("deferred", shown or "—", "deferred", DEFERRED_MODULES[module])
        else:
            ms, skey = split_milestone(module, kind, owner, name, info)
            if skey:
                used["SPLITS"].add((module, skey))
            if ms is None and kind in ("struct", "field"):
                struct = name if kind == "struct" else owner
                if any(it[0] == "struct" and it[2] == struct and it[3] for it in all_items[module]):
                    ms = LAYOUT_MILESTONE
            ms = ms or module_ms(module)
            ex = table_hit(EXCEPTIONS, module, kind, owner, name)
            de = table_hit(DEFERRED, module, kind, owner, name)
            hit = where is not None and found(where, want)
            if ex:
                used["EXCEPTIONS"].add(ex)
                res = (ms, shown or "—", "n/a", EXCEPTIONS[ex])
                if hit and ex == label(module, kind, owner, name):
                    problems.append(f"EXCEPTIONS {ex!r}: {shown} is defined in src/, so the item is ported: drop the "
                                    "entry")
            elif de:
                used["DEFERRED"].add(de)
                res = (ms, shown or "—", "deferred", DEFERRED[de])
                if hit and de == label(module, kind, owner, name):
                    problems.append(f"DEFERRED {de!r}: {shown} is defined in src/, so the item is ported: drop the "
                                    "entry")
            elif hit:
                res = (ms, shown, "ported", "")
            elif where is None:
                note = "no naming rule: add a TRAIT_RENAMES or EXCEPTIONS entry"
                res = (ms, "?", "missing" if ms in done else "pending", note)
            else:
                res = (ms, shown, "missing" if ms in done else "pending", "")
        status_of[key] = res
        return res

    rows = []  # (module, milestone shown, counts)
    statuses = ("ported", "pending", "n/a", "deferred", "missing")
    totals = {s: 0 for s in statuses}
    listing = []
    for module in files:
        if module in NOT_SURFACE_MODULES and module != "lib":
            continue
        if module not in all_items:
            continue
        counts = {s: 0 for s in statuses}
        mss = set()
        for item in all_items[module]:
            if module == "lib" and item[0] != "use":
                continue
            ms, want, st, note = classify(module, item)
            if st == "problem":
                problems.append(note)
                continue
            counts[st] += 1
            totals[st] += 1
            if ms not in ("—", "deferred"):
                mss.add(ms)
            kind, owner, name, info = item
            lab = label(module, kind, owner, name)
            # A re-export of a soorat item is counted, but its source item's line reports it.
            if st == "missing" and not note.startswith("re-export of"):
                problems.append(f"MISSING {lab} ({kind}, {ms} is done) -> {want}" + (f" [{note}]" if note else ""))
            listing.append((module, f"{lab} ({kind})", ms, want, st, note))
        if module == "lib":
            shown = "(source's)"
        elif module in NOT_CARRIED_MODULES:
            shown = "not carried"
        elif module in DEFERRED_MODULES:
            shown = "deferred"
        else:
            shown = "+".join(sorted(mss, key=lambda m: int(m[1:]))) or MODULE_MILESTONE.get(module, "—")
        rows.append((module, shown, counts))

    # Stale table entries.
    for table, tname in ((EXCEPTIONS, "EXCEPTIONS"), (DEFERRED, "DEFERRED")):
        for key in table:
            if key not in used[tname]:
                problems.append(f"{tname} entry {key!r} names no public Rust item (stale)")
    for module, split in SPLITS.items():
        for key in split:
            if (module, key) not in used["SPLITS"]:
                problems.append(f"SPLITS[{module!r}] entry {key!r} names no public Rust item (stale)")
    every = [(m, it) for m, its in all_items.items() for it in its]
    for key in FN_RENAMES:
        if not any(it[0] == "fn" and (f"{it[1]}::{it[2]}" if it[1] else it[2]) == key for _, it in every):
            problems.append(f"FN_RENAMES entry {key!r} names no public Rust fn (stale)")
    for key in FIELD_RENAMES:
        if not any(it[0] == "field" and (it[1], it[2]) == key for _, it in every):
            problems.append(f"FIELD_RENAMES entry {key!r} names no public Rust field (stale)")
    for key in TRAIT_RENAMES:
        if not any(it[0] == "impl" and it[3] == key for _, it in every):
            problems.append(f"TRAIT_RENAMES entry {key!r} names no trait impl of a public type (stale)")
    for key in TYPE_RENAMES:
        if not any(it[0] in ("struct", "enum", "trait", "type") and it[2] == key for _, it in every):
            problems.append(f"TYPE_RENAMES entry {key!r} names no public Rust type (stale)")
    for ms in sorted(done - set(MILESTONES)):
        problems.append(f"plan §14 marks {ms} done, which is no milestone this script knows")

    done_txt = ", ".join(sorted(done, key=lambda m: int(m[1:]))) or "none"
    print(f"surface parity: rust-old/src (Rust 1.2.0) against src/*.cyr; milestones done (plan §14): {done_txt}")
    print()
    print(f"{'module':<18} {'milestone':<10} {'items':>6} {'ported':>7} {'pending':>8} {'n/a':>5} {'deferred':>9} "
          f"{'missing':>8}")
    for module, shown, c in rows:
        n = sum(c.values())
        print(f"{module:<18} {shown:<10} {n:>6} {c['ported']:>7} {c['pending']:>8} {c['n/a']:>5} {c['deferred']:>9} "
              f"{c['missing']:>8}")
    n = sum(totals.values())
    print(f"{'total':<18} {'':<10} {n:>6} {totals['ported']:>7} {totals['pending']:>8} {totals['n/a']:>5} "
          f"{totals['deferred']:>9} {totals['missing']:>8}")
    by_ms = {}
    for module, item, ms, want, st, note in listing:
        if st in ("pending", "missing", "ported") and module != "lib":
            by_ms.setdefault(ms, {"ported": 0, "pending": 0, "missing": 0})[st] += 1
    print()
    print("by milestone (module items; lib.rs re-exports follow their source): " + "; ".join(
        f"{ms} {c['ported']}/{sum(c.values())} ported" + (f", {c['missing']} missing" if c["missing"] else "")
        for ms, c in sorted(by_ms.items(), key=lambda kv: int(kv[0][1:]))))
    if args.list is not None:
        want_mods = set(args.list)
        print()
        for module, item, ms, want, st, note in listing:
            if want_mods and module not in want_mods:
                continue
            print(f"  {st.upper():<8} {ms:<8} {item:<58} -> {want}" + (f"  [{note}]" if note else ""))
    for p in problems:
        print("  " + p)
    n_lib = sum(sum(c.values()) for module, _, c in rows if module == "lib")
    print(f"surface parity: {n} public items ({n - n_lib} in modules, {n_lib} lib.rs re-exports) — "
          f"{totals['ported']} ported, {totals['pending']} pending, {totals['n/a']} n/a, {totals['deferred']} "
          f"deferred, {totals['missing']} missing; {len(problems)} problem(s)")
    return 1 if (args.check and problems) else 0


if __name__ == "__main__":
    sys.exit(main())
