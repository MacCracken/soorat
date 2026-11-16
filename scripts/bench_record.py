"""The benchmark record: every Rust criterion bench beside its Cyrius twin, in docs/benchmarks-rust-v-cyrius.md.

Port plan §0.1 ("every Rust benchmark gets a Cyrius twin", measured, not gated), §4 steps 7–8 and §7 (`cyrius bench`
at every milestone, recorded against Rust); M10 publishes all 35 rows. After pravash's and impetus's
scripts/bench_record.py and bijli's M0 form of it (a twin per bench, pending until its milestone). Usage, from the
repo root:

    python3 -I -B scripts/bench_record.py .                 # write docs/benchmarks-rust-v-cyrius.md
    python3 -I -B scripts/bench_record.py . --check         # the gate: exit 1 on a problem (below)
    python3 -I -B scripts/bench_record.py . --append LOG [--commit SHA] [--branch NAME]
                                                            # add a `cyrius bench` run to bench-history.csv
    python3 -I -B scripts/bench_record.py . --done M1       # preview: also treat M1 as done

**The Rust column** is rust-old/bench-history.csv's run RUST_RUN: 2026-11-15T21:19:13Z at `7f3213a`, the 1.2.0
release run (plan §4 step 8) and, the oracle being frozen, its latest. Criterion's median, in ns. The 35 Rust benches
are read from rust-old/benches/benchmarks.rs (`benchmark_group("g")` then `bench_function("b")`: criterion's id
`g/b`), in file order.

**The Cyrius column** is the latest run (the greatest timestamp) in **`bench-history.csv` at the repository root**:
the Cyrius bench history. Its format is the Rust oracle's and Cyrius's own (`cyrius/scripts/bench-history.sh`):

    timestamp,commit,branch,benchmark,estimate_ns

one row per bench and run, every row of a run sharing its timestamp (`YYYY-MM-DDTHH:MM:SSZ`, UTC), `estimate_ns` the
mean `cyrius bench` reports for the bench, in ns (its `[per op in ps: mean …]` line when there is one, which keeps
sub-ns precision). `--append LOG` adds a run from the saved stdout of `cyrius bench` (taskset to one core, as the
siblings', for a stable number), creating the file with its header the first time; then rerun the script to rewrite
the record.

**Twins.** A twin is a `bench_new("<name>")` in tests/**/*.bcyr; by default it has the Rust bench's own name, and
TWINS names the exceptions. BENCH_MILESTONE gives each criterion group its milestone (plan §3.1). **A milestone is done
when its row in the plan's §14 Tracking table has ✅ in its Status cell.** A Rust bench is
- **twinned** when its twin exists (its Cyrius time shows once a run in bench-history.csv has it);
- **n/a** when REASONS explains why it has none;
- **pending** while its milestone is open, **missing** once it is done (a problem).

**Problems** (`--check` exits 1 on any): a missing twin; a Cyrius bench that twins no Rust bench and isn't in EXTRA;
a TWINS, REASONS, BENCH_MILESTONE or EXTRA entry that names nothing (stale); a Rust bench without a row in the Rust
run; a malformed Cyrius history; a record that isn't the one this script writes now (regenerate it). Stdlib only.
"""
import argparse
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

PLAN = "docs/development/soorat-port-plan.md"
OUT = "docs/benchmarks-rust-v-cyrius.md"
RUST_HISTORY = "rust-old/bench-history.csv"
CYRIUS_HISTORY = "bench-history.csv"
HEADER = "timestamp,commit,branch,benchmark,estimate_ns"
MILESTONES = [f"M{n}" for n in range(0, 11)]  # M0 (prepare and initialize) ports no items

# The 1.2.0 release run (plan §4 step 8), pinned so that a later re-measurement of the oracle can't move the column.
RUST_RUN = "2026-11-15T21:19:13Z"
RUST_NOTE = "the 1.2.0 release run (plan §4 step 8), criterion medians"

# Criterion group (or `group/bench`) -> the milestone that ports what it measures (plan §3.1).
BENCH_MILESTONE = {
    "color": "M1",
    "vertex/bytemuck_cast_100": "M1",  # a Vertex2D stream's bytes: src/layout.cyr
    "vertex/quad_generation_100": "M2",  # quads from sprites
    "sprite": "M2",
    "pipeline": "M2",  # `batch_to_vertices*` live in batch.rs, re-exported by pipeline.rs
    "debug_draw": "M2",  # LineBatch: debug_draw's geometry half
    "terrain": "M3",
    "animation": "M3",
    "primitives": "M3",
    "shadow": "M4",  # matrices, splits and point-light faces: shadow's CPU half
    "screenshot": "M8",
}
# Rust bench -> its Cyrius twin, where the name differs.
TWINS = {
    # bytemuck is not carried (§3.2): a Cyrius vertex stream is already bytes (§9.2), so the twin times taking the
    # byte view of a 100-vertex stream, which is what `cast_slice` gives Rust.
    "vertex/bytemuck_cast_100": "vertex/byte_view_100",
}
# Rust bench -> why it has no twin (`n/a: …`).
REASONS = {}
# Cyrius benches that twin no Rust bench -> what they measure.
EXTRA = {
    # environment's CPU half and the BRDF LUT (M4 part b) have no Rust bench; M7's image-based lighting runs them at load.
    "env/face_mip_chain_128_srgb": "M4: a 128 × 128 sRGB face's CPU mip chain (decode, 7 box filters, encodes); no Rust bench",
    "pbr/brdf_lut_32x32_256": "M4 (`optics`): a 32 × 32 BRDF LUT at 256 samples a texel (prakash 2.15.2's integration); no Rust bench",
    # The acoustic renderers (M5 part a, the `acoustics` profile) have no Rust bench.
    "acoustic/ray_paths_64x8": "M5 (`acoustics`): 64 goonj ray paths of 8 bounces each, drawn as 512 lines; no Rust bench",
    "acoustic/portal": "M5 (`acoustics`): one goonj portal drawn as 7 lines; no Rust bench",
    "acoustic/coupled_decay_100": "M5 (`acoustics`): the default 100-step coupled decay curve (correctly rounded exp and log10); no Rust bench",
    # The render graph (M4 part c) has no Rust bench; a renderer orders its passes each frame.
    "render_graph/build_forward_with_effects": "M4: `forward_with_effects` built (7 passes, 8 dependencies) and dropped; no Rust bench",
    "render_graph/order_forward_with_effects": "M4: `execution_order` of `forward_with_effects` (7 passes); no Rust bench",
    # The glTF loader (M3 part c, the `gltf` profile) has no Rust bench.
    "gltf/load_rich": "M3: both glTF loaders on rich.glb (4.8 KB of JSON, every feature read); no Rust bench",
    "gltf/load_float_stress": "M3: both glTF loaders on float_stress.glb (125 KB of JSON, 400 joints); no Rust bench",
    "noop": "the scaffold's harness check: an empty call, i.e. loop and call overhead (tests/soorat.bcyr)",
    # math_util has no Rust bench; M4's shadow benches call it, so these give them a baseline (tests/math.bcyr).
    "math/mul_mat4": "M1: `mul_mat4` in f64, rounded once (D4 rule b); no Rust bench (`shadow/*` call it, M4)",
    "math/look_at": "M1: `look_at` in f64, rounded once; no Rust bench (`shadow/directional_matrix` calls it, M4)",
    "math/compose_trs": "M1: `compose_trs` in f64, rounded once; no Rust bench (animation's joint chains call it, M3)",
    "math/normalize3": "M1: `normalize3` in f64, rounded once; no Rust bench",
    "math/normal_to_basis": "M1: `normal_to_basis` in f64, rounded once; no Rust bench (acoustic and em call it, M5)",
    "math/perspective_90": "M1: `perspective_90` in f64, rounded once; no Rust bench (`shadow/point_shadow_6_faces`, M4)",
    # environment's f16 and sRGB conversions have no Rust bench; M4's face encoding calls them per component.
    "tables/f16_to_f32": "M1: binary16 → binary32 (bit manipulation); no Rust bench (environment's HDR faces, M4)",
    "tables/f32_to_f16": "M1: binary32 → binary16, ties to even; no Rust bench (environment's HDR faces, M4)",
    "tables/srgb_to_linear": "M1: sRGB decode, f32 with a correctly rounded pow; no Rust bench (environment, M4)",
    "tables/linear_to_srgb": "M1: sRGB encode, f32 with a correctly rounded pow; no Rust bench (environment, M4)",
    "tables/srgb8_decode": "M1: a decode-table lookup (the generated table); no Rust bench (environment, M4)",
    # Color's serde and optics halves (M1 part b2) have no Rust bench.
    "json/color_to_json": "M1: Color's serde document, written (bayan's round-trip digits); no Rust bench",
    "json/color_from_json": "M1: a serde_json Color document, read (bayan's value tree); no Rust bench",
    "color_optics/from_prakash": "M1 (`optics`): a prakash Rgb clamped and narrowed to a Color; no Rust bench",
    "color_optics/from_temperature": "M1 (`optics`): a blackbody colour through prakash's fit; no Rust bench",
    "color_optics/from_wavelength": "M1 (`optics`): a spectral colour through prakash; no Rust bench",
}


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


def rust_benches(root):
    """The `group/bench` id of each bench_function in rust-old/benches/benchmarks.rs, in file order."""
    out, group = [], None
    for line in (root / "rust-old/benches/benchmarks.rs").read_text(encoding="utf-8").split("\n"):
        m = re.search(r'benchmark_group\("([^"]+)"\)', line)
        if m:
            group = m.group(1)
        m = re.search(r'bench_function\("([^"]+)"', line)
        if m:
            out.append(f"{group}/{m.group(1)}")
    return out


def read_history(path):
    """{timestamp: (commit, {bench: ns})} from a bench history CSV, and its problems."""
    runs, problems = {}, []
    if not path.exists():
        return runs, problems
    lines = path.read_text(encoding="utf-8").split("\n")
    if lines[0].strip() != HEADER:
        problems.append(f"{path.name}: the header is not `{HEADER}`")
    for n, line in enumerate(lines[1:], 2):
        if not line.strip():
            continue
        f = line.split(",")
        if len(f) != 5 or not re.fullmatch(r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ", f[0]):
            problems.append(f"{path.name}:{n}: not `{HEADER}`: {line[:80]}")
            continue
        try:
            ns = float(f[4])
        except ValueError:
            problems.append(f"{path.name}:{n}: estimate_ns {f[4]!r} is not a number")
            continue
        runs.setdefault(f[0], (f[1], {}))[1][f[3]] = ns
    return runs, problems


def cyrius_benches(root):
    """Every `bench_new("<name>")` in tests/**/*.bcyr."""
    names = set()
    for path in sorted((root / "tests").rglob("*.bcyr")):
        names.update(re.findall(r'bench_new\("([^"]+)"', path.read_text(encoding="utf-8")))
    return names


UNITS = {"ps": 1e-3, "ns": 1.0, "us": 1e3, "µs": 1e3, "ms": 1e6, "s": 1e9}


def parse_cyrius_log(text):
    """{bench: ns} from `cyrius bench` stdout: `  name: <avg><unit> avg (...)`, refined by a following
    `    [per op in ps: mean N ...]` line."""
    out, last = {}, None
    for line in text.split("\n"):
        m = re.match(r"^\s+(\S.*?): ([\d.]+)\s*(ps|ns|us|µs|ms|s) avg\b", line)
        if m:
            last = m.group(1)
            out[last] = float(m.group(2)) * UNITS[m.group(3)]
            continue
        m = re.match(r"^\s+\[per op in ps: mean (\d+)", line)
        if m and last is not None:
            out[last] = int(m.group(1)) / 1000.0
        last = None if not m else last
    return out


def fmt_ns(x):
    if x >= 1e6:
        return f"{x / 1e6:.2f} ms"
    if x >= 1e3:
        return f"{x / 1e3:.2f} µs"
    return f"{x:.2f} ns"


def milestone_of(bench):
    return BENCH_MILESTONE.get(bench) or BENCH_MILESTONE.get(bench.split("/")[0])


def append(root, log, commit, branch):
    """Add one run, from a `cyrius bench` log, to bench-history.csv."""
    rows = parse_cyrius_log(Path(log).read_text(encoding="utf-8"))
    if not rows:
        print(f"bench record: {log} holds no `cyrius bench` rows (`  name: <time> avg ...`)")
        return 1

    def git(*a):
        try:
            r = subprocess.run(["git", "-C", str(root), *a], capture_output=True, text=True, timeout=10)
            return r.stdout.strip() or "unknown"
        except OSError:
            return "unknown"

    commit = commit or git("rev-parse", "--short", "HEAD")
    branch = branch or git("branch", "--show-current")
    stamp = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    path = root / CYRIUS_HISTORY
    new = not path.exists()
    with path.open("a", encoding="utf-8") as f:
        if new:
            f.write(HEADER + "\n")
        for name, ns in rows.items():
            if "," in name:
                print(f"bench record: skipping {name!r}: a comma can't go in the CSV")
                continue
            f.write(f"{stamp},{commit},{branch},{name},{ns:.4f}\n")
    print(f"bench record: appended {len(rows)} rows at {stamp} ({commit}, {branch}) to {CYRIUS_HISTORY}; rerun "
          "`python3 -I -B scripts/bench_record.py .` to rewrite the record")
    return 0


def build(root, done):
    problems = []
    benches = rust_benches(root)
    if len(benches) != len(set(benches)):
        problems.append("rust-old/benches/benchmarks.rs has duplicate bench ids")
    rust_runs, why = read_history(root / RUST_HISTORY)
    problems += why
    rust_commit, rns = rust_runs.get(RUST_RUN, ("?", {}))
    if RUST_RUN not in rust_runs:
        problems.append(f"{RUST_HISTORY} has no run {RUST_RUN}")
    later = sorted(t for t in rust_runs if t > RUST_RUN)
    missing_rust = [b for b in benches if b not in rns]
    if missing_rust:
        problems.append(f"{len(missing_rust)} Rust benches have no row in run {RUST_RUN}: {missing_rust[:5]}")
    cy_runs, why = read_history(root / CYRIUS_HISTORY)
    problems += why
    cy_stamp = max(cy_runs) if cy_runs else None
    cy_commit, cns = cy_runs[cy_stamp] if cy_stamp else ("", {})
    have = cyrius_benches(root)

    rows, used = [], set()
    counts = {"twinned": 0, "n/a": 0, "pending": 0, "missing": 0}
    for b in benches:
        twin = TWINS.get(b, b)
        ms = milestone_of(b)
        if ms is None:
            problems.append(f"{b} has no BENCH_MILESTONE entry (by group or by name)")
            ms = "?"
        elif ms not in MILESTONES:
            problems.append(f"BENCH_MILESTONE gives {b} the milestone {ms}: not M1–M10")
        r = fmt_ns(rns[b]) if b in rns else "—"
        c, ratio = "—", "—"
        if twin in have:
            used.add(twin)
            counts["twinned"] += 1
            status = f"`{twin}`"
            if twin in cns:
                c = fmt_ns(cns[twin])
                if b in rns and rns[b] > 0:
                    x = cns[twin] / rns[b]
                    ratio = f"{x:.2f}×" if x < 10 else f"{x:.1f}×" if x < 100 else f"{x:.0f}×"
        elif b in REASONS:
            counts["n/a"] += 1
            status = REASONS[b]
        elif ms in done:
            counts["missing"] += 1
            problems.append(f"{b}: {ms} is done, but its twin `{twin}` is no bench_new in tests/*.bcyr, and REASONS "
                            "gives no reason")
            status = f"`{twin}` — **missing**"
        else:
            counts["pending"] += 1
            status = f"`{twin}` — pending"
        rows.append(f"| `{b}` | {ms} | {r} | {status} | {c} | {ratio} |")
    for name in sorted(have - used):
        if name not in EXTRA:
            problems.append(f"Cyrius bench `{name}` twins no Rust bench and is not in EXTRA")
    for name in EXTRA:
        if name not in have:
            problems.append(f"EXTRA names `{name}`, which is no bench_new in tests/*.bcyr (stale)")
    for table, tname in ((TWINS, "TWINS"), (REASONS, "REASONS")):
        for b in table:
            if b not in benches:
                problems.append(f"{tname} names {b}, which is no Rust bench (stale)")
    groups = {b.split("/")[0] for b in benches}
    for key in BENCH_MILESTONE:
        if key not in groups and key not in benches:
            problems.append(f"BENCH_MILESTONE names {key}, which is no Rust bench or group (stale)")
    for b, t in TWINS.items():
        if t in EXTRA:
            problems.append(f"TWINS gives {b} the twin `{t}`, which EXTRA also lists")

    done_txt = ", ".join(sorted(done, key=lambda m: int(m[1:]))) or "none"
    cy_txt = (f"`{cy_stamp}` at `{cy_commit}`, `cyrius bench` means" if cy_stamp else
              "no run recorded yet, so every Cyrius cell is —")
    extra_rows = [f"| `{n}` | {why} | {fmt_ns(cns[n]) if n in cns else '—'} |" for n, why in sorted(EXTRA.items())]
    lines = [
        "# Benchmarks: Rust 1.2.0 vs Cyrius",
        "",
        "> **Generated** by `python3 -I -B scripts/bench_record.py .`; `--check` gates it (port plan §4 step 7, §7).",
        "> Speed is measured, not gated (port plan §0.1, §11): a slow row is a finding to file with its numbers.",
        "",
        f"**Rust column:** `{RUST_HISTORY}`, run `{RUST_RUN}` at `{rust_commit}`: {RUST_NOTE}.",
        "",
        f"**Cyrius column:** `{CYRIUS_HISTORY}` at the repository root (`{HEADER}`, as the Rust history), its latest "
        f"run: {cy_txt}. `scripts/bench_record.py . --append <cyrius bench log>` adds a run.",
        "",
        f"**{len(benches)} Rust benches:** {counts['twinned']} twinned, {counts['n/a']} not applicable, "
        f"{counts['pending']} pending, {counts['missing']} missing. Milestones done (plan §14): {done_txt}.",
        "",
        "| Rust bench | Milestone | Rust 1.2.0 | Cyrius twin | Cyrius | Cyrius / Rust |",
        "|---|---|---:|---|---:|---:|",
        *rows,
        "",
        "## Cyrius-only benches",
        "",
        "| Cyrius bench | What it measures | Cyrius |",
        "|---|---|---:|",
        *extra_rows,
        "",
    ]
    return "\n".join(lines), benches, counts, later, problems


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("root", nargs="?", default=".", help="the repository root (default: .)")
    ap.add_argument("--check", action="store_true",
                    help=f"exit 1 on any problem, including a stale {OUT}; writes nothing")
    ap.add_argument("--append", metavar="LOG", help=f"add a run from a `cyrius bench` log to {CYRIUS_HISTORY}")
    ap.add_argument("--commit", help="the commit to record with --append (default: git rev-parse --short HEAD)")
    ap.add_argument("--branch", help="the branch to record with --append (default: git branch --show-current)")
    ap.add_argument("--done", default="", metavar="M1,M2",
                    help="also treat these milestones as done (a preview of their gate)")
    args = ap.parse_args()
    root = Path(args.root).resolve()
    if args.append:
        return append(root, args.append, args.commit, args.branch)
    extra = {m.strip() for m in args.done.split(",") if m.strip()}
    done, problems = milestones_done(root, extra & set(MILESTONES))
    problems += [f"--done names {m}, which is no milestone (M1–M10)" for m in sorted(extra - set(MILESTONES))]
    text, benches, counts, later, more = build(root, done)
    problems += more
    out = root / OUT
    if args.check:
        if not out.exists() or out.read_text(encoding="utf-8") != text:
            problems.append(f"{OUT} is not current (regenerate: python3 -I -B scripts/bench_record.py .)")
    else:
        out.write_text(text, encoding="utf-8")
        print(f"wrote {OUT}")
    if later:
        print(f"note: {RUST_HISTORY} has runs after the pinned 1.2.0 run {RUST_RUN} ({', '.join(later)}); the record "
              "keeps the pinned one")
    for p in problems:
        print("  BENCH", p)
    print(f"bench record: {len(benches)} Rust benches — {counts['twinned']} twinned, {counts['n/a']} n/a, "
          f"{counts['pending']} pending, {counts['missing']} missing; {len(problems)} problem(s)")
    return 1 if (args.check and problems) else 0


if __name__ == "__main__":
    sys.exit(main())
