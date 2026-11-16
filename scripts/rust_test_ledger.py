"""The Rust-test ledger: every test of the frozen Rust 1.2.0 oracle is ported, pending, or recorded with a reason.

Port plan §4 step 7 (the M0 tooling) and §7 (a gate at the end of every milestone; M10 accounts for all of them).
After pravash's scripts/rust_test_ledger.py (2.0.0; the mela pattern, as impetus's) and bijli's M0 form of it (a
milestone per file, a test pending until its milestone is done). Usage, from the repo root:

    python3 -I -B scripts/rust_test_ledger.py .            # print the ledger's summary
    python3 -I -B scripts/rust_test_ledger.py . --check    # the gate: exit 1 on a problem (below)
    python3 -I -B scripts/rust_test_ledger.py . --write    # also write docs/development/rust-test-ledger.md
    python3 -I -B scripts/rust_test_ledger.py . --done M1  # preview: also treat M1 as done

**The Rust tests.** Each `#[test]` in rust-old/src/**.rs (inside every `#[cfg(test)]` module, wherever it sits),
each in rust-old/tests/*.rs, and each doc test: a Rust code fence in a `///` or `//!` comment (rustdoc lists an
`ignore`d one as an ignored test; rust-old/src/lib.rs doesn't take README.md as the crate's docs, so the README's
fences are not doc tests). EXPECTED pins the counts the oracle reports (state.md: 460 unit, 12 integration, 9 GPU
smoke, 18 oracle-hook, 2 shader-validation), so a parser slip is a problem, not a quiet miscount.

**The naming convention (tests/README.md).** A Rust test is **ported** when one of the Cyrius test files FILES maps
its Rust file to holds `test_group("<the Rust test's name>")`, as pravash's and bijli's do. Rust names repeat across
files (`heat_map_clamps` is in four modules), so a group counts only in its own file's Cyrius files, and each group
name is defined once there. A doc test's group is `doc_<rust file stem>_<line of its fence>`. A function holding a
`test_group` must be called in its file, or the group never runs.

**Milestones.** FILES gives each Rust file its milestone (port plan §3.1, as scripts/surface_parity.py), and
TEST_MILESTONE moves single tests: the halves of a split module (§3.1's estimates, fixed here) and the integration and
GPU smoke tests, each at the milestone of the latest module it uses. **A milestone is done when its row in the plan's
§14 Tracking table has ✅ in its Status cell** (`--done` adds milestones for a preview). A test that isn't ported is
- **n/a** or **deferred** when REASONS (or WHOLE_FILE, for a file) gives the reason, `n/a: …` or `deferred: …`;
- **pending** while its milestone is open;
- **unaccounted** once its milestone is done: a problem.

**Problems** (`--check` exits 1 on any): an unaccounted test; a Rust file with tests and no FILES entry; a REASONS,
WHOLE_FILE, FILES or TEST_MILESTONE entry that names no Rust test or file, or a REASONS entry for a test that is
ported (a stale reason); a group defined twice; a test function nothing calls; a test count that differs from
EXPECTED; and, when docs/development/rust-test-ledger.md exists, a ledger that is not current. Stdlib only.
"""
import argparse
import re
import sys
from pathlib import Path

PLAN = "docs/development/soorat-port-plan.md"
OUT = "docs/development/rust-test-ledger.md"
MILESTONES = [f"M{n}" for n in range(0, 11)]  # M0 (prepare and initialize) ports no items

# The oracle's test counts by kind (docs/development/state.md; 1.2.0, all features). Doc tests are counted apart:
# rustdoc lists egui_bridge's `ignore` fence as one ignored doc test.
EXPECTED = {"unit": 460, "integration": 12, "gpu smoke": 9, "oracle hooks": 18, "shader validation": 2}
KIND_OF_FILE = {"tests/integration.rs": "integration", "tests/gpu_smoke.rs": "gpu smoke",
                "tests/oracle.rs": "oracle hooks", "tests/shaders.rs": "shader validation"}

# Rust file -> (milestone, the Cyrius test files its tests are ported into). Milestones: plan §3.1. The Cyrius files
# are named after §3.1's Cyrius modules; rename them here if a milestone names them otherwise. "—": no milestone.
FILES = {
    "src/error.rs": ("M1", ["tests/error.tcyr"]),
    "src/math_util.rs": ("M1", ["tests/math.tcyr"]),
    "src/color.rs": ("M1", ["tests/color.tcyr", "tests/color_optics.tcyr"]),
    "src/vertex.rs": ("M1", ["tests/layout.tcyr"]),
    "src/instancing.rs": ("M1", ["tests/layout.tcyr"]),
    "src/lights.rs": ("M1", ["tests/lights.tcyr"]),
    "src/lod.rs": ("M1", ["tests/lod.tcyr"]),
    "src/material.rs": ("M6", ["tests/material.tcyr"]),  # owns a GPU texture and bind group: with texture.rs (M6)
    "src/sprite.rs": ("M2", ["tests/sprite.tcyr"]),
    "src/text.rs": ("M2", ["tests/text.tcyr"]),
    "src/ui.rs": ("M2", ["tests/ui.tcyr"]),
    "src/debug_draw.rs": ("M2", ["tests/debug_geom.tcyr", "tests/line_pipeline.tcyr"]),
    "src/primitives.rs": ("M3", ["tests/primitives.tcyr"]),
    "src/terrain.rs": ("M3", ["tests/terrain.tcyr"]),
    "src/viz.rs": ("M3", ["tests/viz.tcyr"]),
    "src/animation.rs": ("M3", ["tests/animation.tcyr"]),
    "src/gltf_loader.rs": ("M3", ["tests/gltf.tcyr"]),
    # shadow: all 23 tests are CPU (matrices, splits, atlas tiling, point faces, the uniforms' layouts): M4.
    "src/shadow.rs": ("M4", ["tests/shadow_math.tcyr", "tests/shadow.tcyr"]),
    "src/environment.rs": ("M4", ["tests/tables.tcyr", "tests/env_cpu.tcyr", "tests/environment.tcyr"]),
    "src/pbr_material.rs": ("M4", ["tests/pbr_material.tcyr"]),
    "src/render_graph.rs": ("M4", ["tests/render_graph.tcyr"]),
    # profiler and capabilities re-export mabda's types: M6, where mabda becomes a dependency (moved at M4 part d).
    "src/profiler.rs": ("M6", ["tests/profiler.tcyr"]),
    "src/capabilities.rs": ("M6", ["tests/capabilities.tcyr"]),
    "src/acoustic_render.rs": ("M5", ["tests/acoustic_render.tcyr"]),
    "src/thermo_render.rs": ("M5", ["tests/thermo_render.tcyr"]),
    "src/fluid_render.rs": ("M5", ["tests/fluid_render.tcyr"]),
    # em_render: D7 deferred it to 2.1.0 at M5's close (bijli 2.0 had not shipped): WHOLE_FILE, EM.
    "src/em_render.rs": ("—", []),
    "src/texture.rs": ("M6", ["tests/texture.tcyr"]),
    "src/render_target.rs": ("M6", ["tests/render_target.tcyr"]),
    "src/compute.rs": ("M6", ["tests/compute.tcyr"]),
    "src/pipeline.rs": ("M6", ["tests/batch.tcyr", "tests/sprite_pipeline.tcyr"]),
    "src/mesh_pipeline.rs": ("M6", ["tests/mesh_pipeline.tcyr"]),
    "src/gpu.rs": ("M6", []),
    "src/hdr.rs": ("M8", ["tests/hdr.tcyr"]),
    "src/postprocess.rs": ("M8", ["tests/postprocess.tcyr"]),
    "src/ssao.rs": ("M8", ["tests/ssao.tcyr"]),
    "src/gpu_particles.rs": ("M8", ["tests/gpu_particles.tcyr"]),
    "src/screenshot.rs": ("M8", ["tests/screenshot.tcyr"]),
    "src/window.rs": ("M9", ["tests/surface.tcyr"]),
    "src/drm.rs": ("M9", ["tests/drm.tcyr"]),
    "src/egui_bridge.rs": ("—", []),
    "tests/integration.rs": ("M2", ["tests/integration.tcyr"]),
    "tests/gpu_smoke.rs": ("M6", ["tests/gpu_smoke.tcyr"]),
    "tests/oracle.rs": ("—", []),
    "tests/shaders.rs": ("—", []),
}

# (Rust file, test) -> its milestone, where it isn't its file's.
TEST_MILESTONE = {
    # error's `From<mabda::GpuError>` (M6): mabda, and with it the GPU_ERR_* codes, becomes a dependency at M6
    # (`sr_error_from_gpu`, tests/error.tcyr). The rest of error.rs is M1.
    ("src/error.rs", "from_gpu_error_adapter_not_found"): "M6",
    ("src/error.rs", "from_gpu_error_surface_config"): "M6",
    ("src/error.rs", "from_gpu_error_shader"): "M6",
    ("src/error.rs", "from_gpu_error_pipeline"): "M6",
    ("src/error.rs", "from_gpu_error_other_variants_fallback"): "M6",
    # vertex and instancing (M6): their types are mabda's, re-exported; these tests exercise mabda's own methods
    # (the vertex-buffer layout descriptor, InstanceData::default and from_translation), which soorat takes from
    # mabda at M6. Their sizes and bytes are soorat's layouts (tests/layout.tcyr, M1).
    ("src/vertex.rs", "vertex2d_layout_stride"): "M6",
    ("src/vertex.rs", "vertex3d_layout_stride"): "M6",
    ("src/instancing.rs", "instance_data_default"): "M6",
    ("src/instancing.rs", "instance_data_translation"): "M6",
    ("src/instancing.rs", "instance_data_layout"): "M6",
    ("src/instancing.rs", "instance_data_batch_cast"): "M6",
    # pipeline.rs's batch tests (M2 part a): pipeline re-exports batch.rs's functions and tests them
    # (tests/batch.tcyr). Its orthographic projection and frame stats are the GPU sprite pipeline's (M6).
    ("src/pipeline.rs", "batch_to_vertices_empty"): "M2",
    ("src/pipeline.rs", "batch_to_vertices_single_sprite"): "M2",
    ("src/pipeline.rs", "batch_to_vertices_rotated_sprite"): "M2",
    ("src/pipeline.rs", "batch_to_vertices_zero_rotation_matches_unrotated"): "M2",
    ("src/pipeline.rs", "batch_to_vertices_multiple_sprites"): "M2",
    ("src/pipeline.rs", "batch_to_vertices_preserves_color"): "M2",
    ("src/pipeline.rs", "batch_to_vertices_tex_coords"): "M2",
    ("src/pipeline.rs", "batch_to_vertices_into_reuses_buffers"): "M2",
    ("src/pipeline.rs", "batch_to_vertices_with_uv"): "M2",
    ("src/pipeline.rs", "batch_to_vertices_into_matches_batch_to_vertices"): "M2",
    ("src/pipeline.rs", "max_sprites_constant"): "M2",
    ("src/pipeline.rs", "batch_u32_overflow_protection"): "M2",
    ("src/pipeline.rs", "batch_to_vertices_respects_limit"): "M2",
    ("src/pipeline.rs", "batch_to_vertices_u32_no_limit"): "M2",
    ("src/pipeline.rs", "batch_to_vertices_u32_matches_u16_for_small"): "M2",
    ("src/pipeline.rs", "batch_to_vertices_at_exact_limit"): "M2",
    # debug_draw's line pipeline half (M6): the wgpu vertex-buffer layout descriptor.
    ("src/debug_draw.rs", "line_vertex_layout"): "M6",
    # environment's f16 and sRGB conversions (M1 part d, src/tables.cyr, tests/tables.tcyr).
    ("src/environment.rs", "f16_roundtrip_exact_values"): "M1",
    ("src/environment.rs", "f16_special_values"): "M1",
    ("src/environment.rs", "f16_rounds_to_nearest_even"): "M1",
    ("src/environment.rs", "srgb_transfer_roundtrip"): "M1",
    # environment's GPU half (M7): the IBL bind group and the cube map types. Its downsample and mip-chain tests are
    # its CPU half (M4).
    ("src/environment.rs", "ibl_bind_group_layout_types"): "M7",
    ("src/environment.rs", "environment_map_types"): "M7",
    # The integration tests (the rest use sprites and batches: M2).
    ("tests/integration.rs", "color_conversions"): "M1",
    ("tests/integration.rs", "vertex_sizes"): "M1",
    ("tests/integration.rs", "vertex_color_interop"): "M1",
    ("tests/integration.rs", "texture_cache_workflow"): "M6",
    ("tests/integration.rs", "window_config_integration"): "M9",
    # The GPU smoke tests (the sprite pipeline's three are M6).
    ("tests/gpu_smoke.rs", "pbr_mesh_with_shadow_and_ibl_renders"): "M7",
    ("tests/gpu_smoke.rs", "skinned_pbr_shader_compiles_for_backend"): "M7",
    ("tests/gpu_smoke.rs", "post_processing_composites_bloom_when_bound"): "M8",
    ("tests/gpu_smoke.rs", "post_processing_and_debug_pipelines_build_and_run"): "M8",
    ("tests/gpu_smoke.rs", "particle_system_simulates_and_reads_back"): "M8",
    ("tests/gpu_smoke.rs", "particle_damping_is_frame_rate_independent"): "M8",
}

ORACLE = ("n/a: the tests of the `oracle` hooks added to the frozen crate for this port (D1, port plan §4 step 0), not "
          "of soorat 1.2.0's API; scripts/golden/ drives the hooks, and the golden corpus checks what they give")
EGUI = "n/a: `egui_bridge` is not carried: Cyrius has no egui (D8, §3.2, ADR 0002 #10)"
NAGA = ("n/a: naga's offline WGSL validation. Cyrius has no offline validator; wgpu-native validates each shader at "
        "pipeline creation, and the GPU smoke tests create every pipeline (port plan §1.1, §3.2)")
L10 = ("deferred: agnodrm 1.8.1 has no render-node or connector query, so 2.0 carries the reduced drm subset (D8, "
       "ADR 0002 #11) until L10 lands")
# (Rust file, test) -> why it has no port: `n/a: …` (never ported) or `deferred: …` (after 2.0.0).
REASONS = {
    ("src/error.rs", "error_display"): "n/a: thiserror's message text is not carried; errors are SR_ERR_* codes "
                                       "(§3.2, ADR 0002 #1)",
    ("src/error.rs", "error_other_variant"): "n/a: `Other` boxes a foreign error and shows its message; a 2.0 error is "
                                             "an SR_ERR_* code with no payload (§3.2, ADR 0002 #1)",
    ("src/error.rs", "error_is_send_sync"): "n/a: Rust's `Send + Sync` mechanics have no Cyrius counterpart (§3.2)",
    ("src/gpu.rs", "gpu_context_types"): "n/a: checks soorat's re-export of mabda's GpuContext; 2.0 takes the "
                                         "application's mabda context and re-exports nothing (ADR 0002 #2, #3)",
    ("src/drm.rs", "list_render_nodes_returns_vec"): L10,
    ("src/drm.rs", "list_connectors_nonexistent_path"): L10,
    ("src/drm.rs", "connector_summary_debug"): L10,
}
# Rust file -> the reason for each of its tests (the whole file has no port).
EM = ("deferred: D7, applied at M5's close (2026-11-15): bijli 2.0.0, whose M7 carries the visualization types "
      "em_render reads, has not shipped, so 2.0.0 goes without the `em` feature and em_render follows in 2.1.0")
WHOLE_FILE = {"src/egui_bridge.rs": EGUI, "src/em_render.rs": EM, "tests/oracle.rs": ORACLE, "tests/shaders.rs": NAGA}

# Code fence info strings rustdoc compiles or lists as a test; any other tag is another language.
RUST_TAGS = {"rust", "ignore", "no_run", "should_panic", "compile_fail", "edition2015", "edition2018",
             "edition2021", "edition2024", "test_harness", "standalone_crate"}


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


def test_blocks(lines):
    """The lines of every `#[cfg(test)] mod … { … }` block, by brace depth, wherever it sits."""
    out, i = [], 0
    while i < len(lines):
        if lines[i].strip() == "#[cfg(test)]" and i + 1 < len(lines) and re.match(r"\s*(pub(\(\w+\))? )?mod \w+\s*\{",
                                                                                 lines[i + 1]):
            depth, j = 0, i + 1
            while j < len(lines):
                out.append((j + 1, lines[j]))
                depth += lines[j].count("{") - lines[j].count("}")
                if depth <= 0:
                    break
                j += 1
            i = j + 1
            continue
        i += 1
    return out


def rust_tests(root, rel):
    """[(name, kind, ignored)] of rust-old/<rel>: its #[test]s (in its test modules, or anywhere in a tests/ file),
    then its doc tests (`doc test at line N`)."""
    text = (root / "rust-old" / rel).read_text(encoding="utf-8")
    lines = text.split("\n")
    scan = list(enumerate(lines, 1)) if rel.startswith("tests/") else test_blocks(lines)
    kind = KIND_OF_FILE.get(rel, "unit")
    out, pending, ignored = [], False, False
    for _, line in scan:
        s = line.strip()
        if s == "#[test]" or re.fullmatch(r"#\[tokio::test(\(.*\))?\]", s):
            pending, ignored = True, False
            continue
        if pending:
            if s.startswith("#["):
                ignored = ignored or s.startswith("#[ignore")
                continue
            m = re.match(r"(?:pub )?(?:async )?fn (\w+)", s)
            if m:
                out.append((m.group(1), kind, ignored))
            pending = False
    if rel.startswith("src/"):
        inside = False
        for no, line in enumerate(lines, 1):
            m = re.match(r"\s*(?:///|//!)\s?```\s*(.*?)\s*$", line)
            if not m:
                continue
            if inside:
                inside = False
                continue
            inside = True
            tags = {t.strip() for t in m.group(1).split(",") if t.strip()}
            if tags <= RUST_TAGS:
                out.append((f"doc test at line {no}", "doc", "ignore" in tags))
    return out


def group_counts(root, rel):
    """Each test_group name in a Cyrius file, with how often it is defined ({} when the file doesn't exist yet)."""
    path = root / rel
    if not path.exists():
        return {}
    out = {}
    for g in re.findall(r'test_group\("([^"]+)"', path.read_text(encoding="utf-8")):
        out[g] = out.get(g, 0) + 1
    return out


def uncalled(root, rel):
    """The functions of a Cyrius file that hold a test_group and are called nowhere in it."""
    path = root / rel
    if not path.exists():
        return []
    text = path.read_text(encoding="utf-8")
    out = []
    for m in re.finditer(r"^fn (\w+)\(", text, re.M):
        line_end = text.find("\n", m.start())
        line_end = len(text) if line_end < 0 else line_end
        if text[m.start():line_end].rstrip().endswith("}"):
            end = line_end  # a one-line function: `fn f(k): i64 { return …; }`
        else:
            close = re.compile(r"^}", re.M).search(text, m.start())
            end = close.end() if close else len(text)
        if "test_group(" not in text[m.start():end]:
            continue
        rest = text[:m.start()] + text[end:]
        if not re.search(rf"(?<![\w.]){m.group(1)}\(", rest):
            out.append(m.group(1))
    return out


def group_name(rel, name):
    """The Cyrius test_group a Rust test is ported as."""
    m = re.match(r"doc test at line (\d+)$", name)
    return f"doc_{Path(rel).stem}_{m.group(1)}" if m else name


def build(root, done):
    problems = []
    rust_files = sorted(str(p.relative_to(root / "rust-old")) for d in ("src", "tests")
                        for p in (root / "rust-old" / d).rglob("*.rs"))
    tests = {rel: rust_tests(root, rel) for rel in rust_files}
    for rel in rust_files:
        if tests[rel] and rel not in FILES:
            problems.append(f"{rel} has {len(tests[rel])} tests and no FILES entry")
    for rel, (ms, _) in FILES.items():
        if rel not in tests:
            problems.append(f"FILES names {rel}, which is no file in rust-old/")
        elif not tests[rel]:
            problems.append(f"FILES names {rel}, which has no tests")
        if ms not in MILESTONES and not (ms == "—" and rel in WHOLE_FILE):
            problems.append(f"FILES gives {rel} the milestone {ms}: not M1–M10 (a file with no milestone is WHOLE_FILE)")
    for (rel, name), ms in TEST_MILESTONE.items():
        if name not in {t for t, _, _ in tests.get(rel, [])}:
            problems.append(f"TEST_MILESTONE names {rel}::{name}, which is no Rust test (stale)")
        if ms not in MILESTONES:
            problems.append(f"TEST_MILESTONE gives {rel}::{name} the milestone {ms}: not M1–M10")
    for (rel, name), why in REASONS.items():
        if name not in {t for t, _, _ in tests.get(rel, [])}:
            problems.append(f"REASONS names {rel}::{name}, which is no Rust test (stale)")
        if not why.startswith(("n/a:", "deferred:")):
            problems.append(f"REASONS {rel}::{name}: a reason starts `n/a:` or `deferred:`")
        if rel in WHOLE_FILE:
            problems.append(f"REASONS names {rel}::{name}, whose whole file is in WHOLE_FILE")
    for rel in WHOLE_FILE:
        if rel not in FILES:
            problems.append(f"WHOLE_FILE names {rel}, which has no FILES entry")

    counts = {k: 0 for k in list(EXPECTED) + ["doc"]}
    for rel, ts in tests.items():
        for _, kind, _ in ts:
            counts[kind] += 1
    for kind, n in EXPECTED.items():
        if counts[kind] != n:
            problems.append(f"{counts[kind]} {kind} tests counted in rust-old/, {n} expected (EXPECTED)")

    rows, notes, ignored, pending_by_ms = [], [], [], {}
    sums = {"ported": 0, "n/a": 0, "deferred": 0, "pending": 0, "unaccounted": 0}
    checked = set()
    for rel, (file_ms, cys) in FILES.items():
        ts = tests.get(rel, [])
        have = {}
        for cy in cys:
            for g, c in group_counts(root, cy).items():
                have[g] = have.get(g, 0) + c
                if c > 1 and (cy, g) not in checked:
                    problems.append(f"{cy}: test_group(\"{g}\") is defined {c} times")
                checked.add((cy, g))
            if cy not in checked:
                checked.add(cy)
                for name in uncalled(root, cy):
                    problems.append(f"{cy}: {name} holds a test_group and nothing calls it, so it never runs")
        c = {k: 0 for k in sums}
        mss = set()
        for name, kind, ign in ts:
            ms = TEST_MILESTONE.get((rel, name), file_ms)
            group = group_name(rel, name)
            reason = REASONS.get((rel, name)) or WHOLE_FILE.get(rel)
            if group in have:
                c["ported"] += 1
                if have[group] > 1 and len(cys) > 1:
                    problems.append(f"{rel}: {group} is defined {have[group]} times across {cys}")
                if reason:
                    problems.append(f"{rel}::{name} is ported, but {'REASONS' if (rel, name) in REASONS else 'WHOLE_FILE'}"
                                    " still gives a reason (stale)")
            elif reason:
                c[reason.split(":")[0]] += 1
                if rel not in WHOLE_FILE:
                    notes.append((rel, name, reason))
            elif ms in done:
                c["unaccounted"] += 1
                problems.append(f"{rel}::{name}: {ms} is done, but the test is neither ported (test_group(\"{group}\") "
                                f"in {', '.join(cys) or 'no Cyrius file'}) nor in REASONS")
            else:
                c["pending"] += 1
                pending_by_ms[ms] = pending_by_ms.get(ms, 0) + 1
            if ms in MILESTONES:
                mss.add(ms)
            if ign:
                ignored.append((rel, name, group in have))
        for k in sums:
            sums[k] += c[k]
        shown = "+".join(sorted(mss, key=lambda m: int(m[1:]))) or "—"
        kinds = sorted({k for _, k, _ in ts})
        rows.append((rel, ", ".join(kinds), shown, cys, len(ts), c))
    return tests, counts, rows, notes, ignored, sums, pending_by_ms, problems


def cell(cys):
    return "<br>".join(f"`{c}`" for c in cys) if cys else "—"


def render(counts, rows, notes, ignored, sums, pending_by_ms, done):
    total = sum(counts.values())
    md = [
        "# Rust-test ledger",
        "",
        "> **Generated** by `python3 -I -B scripts/rust_test_ledger.py . --write`; `--check` gates it (port plan §4",
        "> step 7, §7). Every test of the frozen Rust oracle (`rust-old/`, soorat 1.2.0) is ported to a Cyrius",
        "> `test_group` of the same name (tests/README.md), recorded here with its reason, or pending until its",
        "> milestone (plan §3.1; done = ✅ in the plan's §14). The golden corpus checks every ported function on its",
        "> grid besides; this ledger is about the oracle's own assertions.",
        "",
        f"**{total} Rust tests** ({counts['unit']} unit, {counts['integration']} integration, {counts['gpu smoke']} "
        f"GPU smoke, {counts['oracle hooks']} oracle-hook, {counts['shader validation']} shader-validation, "
        f"{counts['doc']} doc): {sums['ported']} ported, {sums['n/a']} not applicable, {sums['deferred']} deferred, "
        f"{sums['pending']} pending, {sums['unaccounted']} unaccounted. Milestones done: "
        f"{', '.join(sorted(done, key=lambda m: int(m[1:]))) or 'none'}.",
        "",
        "| Rust file | Kind | Milestone | Cyrius tests | Rust tests | Ported | N/A | Deferred | Pending |",
        "|---|---|---|---|---:|---:|---:|---:|---:|",
    ]
    for rel, kinds, ms, cys, n, c in rows:
        pend = c["pending"] + c["unaccounted"]
        md.append(f"| `{rel}` | {kinds} | {ms} | {cell(cys)} | {n} | {c['ported']} | {c['n/a']} | {c['deferred']} | "
                  f"{pend}{' (' + str(c['unaccounted']) + ' unaccounted)' if c['unaccounted'] else ''} |")
    md.append(f"| **Total** | | | | **{total}** | **{sums['ported']}** | **{sums['n/a']}** | **{sums['deferred']}** | "
              f"**{sums['pending'] + sums['unaccounted']}** |")
    md += ["", "## Pending by milestone", ""]
    md.append(", ".join(f"{ms}: {n}" for ms, n in sorted(pending_by_ms.items(), key=lambda kv: int(kv[0][1:])))
              or "None.")
    md += ["", "## Rust's ignored tests", ""]
    if ignored:
        for rel, name, ported in ignored:
            md.append(f"- `{rel}` {name}: " + ("ported, live." if ported else "not ported."))
    else:
        md.append("None.")
    md += ["", "## Tests without a port", "", "| Rust file | Test | Status |", "|---|---|---|"]
    for rel, t, reason in notes:
        md.append(f"| `{rel}` | `{t}` | {reason} |")
    for rel, reason in WHOLE_FILE.items():
        n = next((r[4] for r in rows if r[0] == rel), 0)
        md.append(f"| `{rel}` | all {n} | {reason} |")
    md.append("")
    return "\n".join(md)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("root", nargs="?", default=".", help="the repository root (default: .)")
    ap.add_argument("--check", action="store_true", help="exit 1 on any problem (the CI gate)")
    ap.add_argument("--write", action="store_true", help=f"write {OUT}")
    ap.add_argument("--done", default="", metavar="M1,M2",
                    help="also treat these milestones as done (a preview of their gate)")
    args = ap.parse_args()
    root = Path(args.root).resolve()
    extra = {m.strip() for m in args.done.split(",") if m.strip()}
    done, problems = milestones_done(root, extra & set(MILESTONES))
    problems += [f"--done names {m}, which is no milestone (M1–M10)" for m in sorted(extra - set(MILESTONES))]
    tests, counts, rows, notes, ignored, sums, pending_by_ms, more = build(root, done)
    problems += more
    text = render(counts, rows, notes, ignored, sums, pending_by_ms, done)
    out_path = root / OUT
    if args.write:
        out_path.write_text(text, encoding="utf-8")
        print(f"wrote {OUT}")
    elif out_path.exists() and out_path.read_text(encoding="utf-8") != text:
        problems.append(f"{OUT} is stale (regenerate: python3 -I -B scripts/rust_test_ledger.py . --write)")

    total = sum(counts.values())
    print(f"rust-test ledger: rust-old/ (Rust 1.2.0) against tests/*.tcyr; milestones done (plan §14): "
          f"{', '.join(sorted(done, key=lambda m: int(m[1:]))) or 'none'}")
    print("counted: " + ", ".join(f"{n} {k}" for k, n in counts.items()) + f" = {total}")
    print()
    print(f"{'Rust file':<26} {'milestone':<10} {'tests':>5} {'ported':>7} {'n/a':>5} {'deferred':>9} {'pending':>8}")
    for rel, kinds, ms, cys, n, c in rows:
        print(f"{rel:<26} {ms:<10} {n:>5} {c['ported']:>7} {c['n/a']:>5} {c['deferred']:>9} "
              f"{c['pending'] + c['unaccounted']:>8}")
    print()
    print("pending by milestone: " + (", ".join(f"{ms} {n}" for ms, n in sorted(pending_by_ms.items(),
                                                                              key=lambda kv: int(kv[0][1:])))
                                      or "none"))
    if not out_path.exists() and not args.write:
        print(f"note: {OUT} is not generated yet (--write); --check checks it once it exists")
    for p in problems:
        print("  LEDGER", p)
    print(f"rust-test ledger: {total} tests — {sums['ported']} ported, {sums['n/a']} n/a, {sums['deferred']} deferred, "
          f"{sums['pending']} pending, {sums['unaccounted']} unaccounted; {len(problems)} problem(s)")
    return 1 if (args.check and problems) else 0


if __name__ == "__main__":
    sys.exit(main())
