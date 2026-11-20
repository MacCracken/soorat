"""Check the upstream ledger (port plan §10) against the code and the sibling repositories.

Port plan §0.1 (every gap is filed upstream with soorat's numbers; a local stand-in is marked `UPSTREAM(Ln)` and
tracked in the ledger), §4 step 7 and §7 (`upstream_ledger.py --check` at the end of every milestone). After
pravash's scripts/upstream_ledger.py. Usage, from the repo root:

    python3 -I -B scripts/upstream_ledger.py .             # print the ledger's state
    python3 -I -B scripts/upstream_ledger.py . --check     # the gate: exit 1 on a failure (below)
    python3 -I -B scripts/upstream_ledger.py . --done M1   # preview: also treat M1 as done

**The ledger** is the table in docs/development/soorat-port-plan.md §10: `| L | Project | Request | Stand-in |`, one
row per request (`L2–L6` is one row for five ids). A filed request is a backticked `<repo>/<path>.md` in the row,
relative to the sibling repository's parent (`mabda/docs/development/issues/…` is /home/…/Repos/mabda/docs/…: the
siblings sit beside this repository). **A marker** is the text `UPSTREAM(Ln)` in a Cyrius source under src/ or tests/
(`*.cyr`, `*.tcyr`, `*.bcyr`, `*.fcyr`), at the stand-in's use.

STANDINS lists the rows whose stand-in is code in soorat, with the milestone that writes it (plan §3.1); NO_STANDIN
lists the others, with why no marker is due. Every row is in exactly one. **A milestone is done when its row in the
plan's §14 Tracking table has ✅ in its Status cell.** This checks:
1. the rows' ids run L1, L2, … with no gap and none twice;
2. every marker names a ledger row, and none names a row whose stand-in is "none allowed" (D3: no wgpu FFI in soorat);
3. every STANDINS row carries at least one marker once its milestone is done;
4. every filed path exists, and a row that says it was filed names one. A sibling that isn't checked out (as on CI)
   leaves its paths unverified, reported, not failed;
5. STANDINS and NO_STANDIN name only rows of the ledger, and a STANDINS row's stand-in isn't "none …".
It prints the ledger's state either way; with `--check` it exits 1 on any failure. Stdlib only.
"""
import argparse
import re
import sys
from pathlib import Path

PLAN = "docs/development/soorat-port-plan.md"
MILESTONES = [f"M{n}" for n in range(1, 11)]
NOTE = re.compile(r"UPSTREAM\((L\d+)\)")
SOURCES = (".cyr", ".tcyr", ".bcyr", ".fcyr")

# Row -> (the milestone that writes its stand-in, the stand-in). Its marker is due once that milestone is done.
STANDINS = {
    # crmath arrived at M3 part b with slerp's sin and acos (kimiya 2.0.1, scripts/vendor_crmath.py); M5 part a added
    # exp and log10 for the acoustic decay curve, in src/crmath_ext.cyr (the `acoustics` profile only); part c moved
    # cos (the balloon, the thermal network) into src/crmath.cyr, as it reads only sin's tables.
    "L1": ("M3", "the vendored correctly rounded crmath (kimiya): src/crmath.cyr (slerp's sin and acos, §3.1) and "
                 "src/crmath_ext.cyr (the decay curve's exp and log10, M5); src/crmath.cyr gained cos at M5"),
    "L22": ("M3", "Rust's f32 `%` for AnimationClip::sample's time wrap: _sr_f32_fmod in src/mathx.cyr"),
    "L8": ("M1", "f32 results returned as i64 bits, `var` constants and local f16 code: src/mathx.cyr (§3.1)"),
    "L9": ("M8", "selah's private PNG/JPEG/BMP encoders, vendored for screenshot (D8)"),
    "L21": ("M1", "Color's from_temperature / from_wavelength call prakash's allocating forms: src/color_optics.cyr"),
    "L10": ("M9", "the reduced drm subset (D8, ADR 0002 #11): where connector, mode and render-node queries are missing"),
}
# Row -> why no marker is due.
NO_STANDIN = {
    "L2–L6": "no stand-in is allowed (D3, §0.1): a mabda stand-in would be wgpu FFI in soorat; shipped in mabda 4.3.x",
    "L7": "none: the native shader path follows 2.0 (§13)",
    "L11": "a pin tidy-up in the science crates (P4: hisab 3.5.0 serves all five); no soorat code",
    "L12": "mabda's own pin and dependency scope (P5); no soorat code",
    "L13": "soorat calls memcpy as it is; the frame arena (§9.4) is the design, not a stand-in to remove",
    "L14": "the repairs are docs and the manifest (state.md), not code",
    "L15": "none needed by 2.0 (D5: the application owns the window)",
    "L16": "D7, applied at M5's close: bijli 2.0 had not shipped, so em_render follows in 2.1.0; no stand-in code",
    "L17": "none: a launcher change in mabda, landed with L2–L6 in 4.3.0",
    "L23": "bayan's JSON verdicts where serde_json's differ: the glTF rows are named (ADR 0002 #14), no stand-in code",
    "L24": "none: resolved in mabda 4.3.0, before soorat's profiler arrives with mabda at M6",
    "L25": "none: pravash's static data only shapes soorat's test layout (the fluids replay is its own harness)",
    "L26": "none: the rows prakash's green exponent changes are named Color::from_temperature_prakash (pending)",
    "L27": "none: the rows prakash's f64 rounding changes are named Color::from_*_prakash (pending)",
    "L18": "none possible (a dependency's names): the `ranga` feature excludes `acoustics` and `thermo` until it lands",
    "L19": "a manifest arrangement (agnodrm only behind the `drm` feature, cyrius.cyml), not code",
    "L20": "none: a duplicate-definition warning, last definition wins",
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


def ledger_rows(root):
    """[(label, [ids], project, request, stand-in, [filed paths])] from the plan's §10 table, and problems."""
    text = (root / PLAN).read_text(encoding="utf-8")
    at = text.find("\n## 10.")
    if at < 0:
        return [], [f"{PLAN} has no '## 10.' section"]
    rows = []
    for line in text[at + 1:].splitlines()[1:]:
        if line.startswith("## "):
            break
        m = re.match(r"^\| (L\d+(?:\s*[–-]\s*L\d+)?) \|", line)
        if not m:
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) != 4:
            return rows, [f"§10 row {m.group(1)} has {len(cells)} cells, not 4 (L, Project, Request, Stand-in)"]
        label = re.sub(r"\s+", "", cells[0])
        r = re.fullmatch(r"L(\d+)(?:[–-]L(\d+))?", label)
        lo, hi = int(r.group(1)), int(r.group(2) or r.group(1))
        ids = [f"L{n}" for n in range(lo, hi + 1)]
        paths = sorted(set(re.findall(r"`([a-z0-9_-]+/[^`\s]+\.md)`", line)))
        rows.append((label, ids, cells[1], cells[2], cells[3], paths))
    return rows, []


def markers(root):
    """{id: [file:line, ...]} for every UPSTREAM(Ln) in a Cyrius source under src/ and tests/."""
    found = {}
    for top in ("src", "tests"):
        for path in sorted((root / top).rglob("*")):
            if path.suffix not in SOURCES or not path.is_file():
                continue
            for n, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
                for rid in NOTE.findall(line):
                    found.setdefault(rid, []).append(f"{path.relative_to(root)}:{n}")
    return found


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("root", nargs="?", default=".", help="the repository root (default: .)")
    ap.add_argument("--check", action="store_true", help="exit 1 on any failure (the CI gate)")
    ap.add_argument("--done", default="", metavar="M1,M2",
                    help="also treat these milestones as done (a preview of their gate)")
    args = ap.parse_args()
    root = Path(args.root).resolve()
    repos = root.parent
    extra = {m.strip() for m in args.done.split(",") if m.strip()}
    done, fails = milestones_done(root, extra & set(MILESTONES))
    fails += [f"--done names {m}, which is no milestone (M1–M10)" for m in sorted(extra - set(MILESTONES))]
    rows, why = ledger_rows(root)
    fails += why
    notes = markers(root)
    if not rows:
        fails.append("no ledger rows found in §10")

    # 1. ids: L1, L2, … once each.
    by_id = {}
    for label, ids, *_ in rows:
        for rid in ids:
            if rid in by_id:
                fails.append(f"{rid} is in two rows ({by_id[rid]} and {label})")
            by_id[rid] = label
    nums = sorted(int(r[1:]) for r in by_id)
    gaps = [f"L{n}" for n in range(1, (nums[-1] if nums else 0) + 1) if n not in nums]
    if gaps:
        fails.append(f"the ledger has no row for {', '.join(gaps)}")

    # 5. the classification covers the rows exactly.
    labels = {r[0] for r in rows}
    for label in labels:
        if (label in STANDINS) + (label in NO_STANDIN) != 1:
            fails.append(f"row {label} must be in exactly one of STANDINS and NO_STANDIN")
    for table, tname in ((STANDINS, "STANDINS"), (NO_STANDIN, "NO_STANDIN")):
        for label in table:
            if label not in labels:
                fails.append(f"{tname} names {label}, which is no row of the §10 ledger (stale)")
    for label, ms in ((k, v[0]) for k, v in STANDINS.items()):
        if ms not in MILESTONES:
            fails.append(f"STANDINS gives {label} the milestone {ms}: not M1–M10")

    # 2. every marker names a row, and not a no-stand-in-allowed one.
    forbidden = {rid for label, ids, _, _, standin, _ in rows if "none allowed" in standin.lower() for rid in ids}
    for rid, where in sorted(notes.items(), key=lambda kv: int(kv[0][1:])):
        if rid not in by_id:
            fails.append(f"{where[0]}: UPSTREAM({rid}) names no ledger row")
        elif rid in forbidden:
            fails.append(f"{where[0]}: UPSTREAM({rid}): row {by_id[rid]} allows no stand-in (D3, §0.1)")

    # 3. a stand-in is marked once its milestone is done; 4. filed paths exist.
    unverified = set()
    report = []
    for label, ids, project, request, standin, paths in rows:
        marked = sum(len(notes.get(rid, [])) for rid in ids)
        if label in STANDINS:
            ms, what = STANDINS[label]
            if re.match(r"^\W*none\b", standin, re.I):
                fails.append(f"STANDINS lists {label}, but the plan's stand-in for it is '{standin[:40]}…'")
            if ms in done and marked == 0:
                fails.append(f"{label}: {ms} is done, but no UPSTREAM({ids[0]}) marks its stand-in ({what})")
            state = f"stand-in at {ms}" + (" (due)" if ms in done else "")
        else:
            state = "no stand-in"
        found = missing = 0
        for p in paths:
            repo, _, rel = p.partition("/")
            if not (repos / repo).is_dir():
                unverified.add(repo)
            elif (repos / repo / rel).exists():
                found += 1
            else:
                missing += 1
                fails.append(f"{label}: the filed request {p} does not exist at {repos / repo / rel}")
        if re.search(r"\bfiled\b", request, re.I) and not paths:
            fails.append(f"{label} says it was filed but names no `<repo>/…/<file>.md`")
        report.append(f"{label:>6}  {project.replace('*', '')[:22]:<22} {state:<20} markers {marked:<3} "
                      f"filed {len(paths)} ({found} found" + (f", {missing} missing" if missing else "") +
                      (f", {len(paths) - found - missing} unverified" if len(paths) - found - missing else "") + ")")

    print(f"upstream ledger: {PLAN} §10 against src/ and tests/; milestones done (plan §14): "
          f"{', '.join(sorted(done, key=lambda m: int(m[1:]))) or 'none'}")
    for line in report:
        print(line)
    if unverified:
        print(f"note: no checkout of {', '.join(sorted(unverified))} beside this repository: their requests were "
              "not verified")
    for f in fails:
        print("  FAIL", f)
    total = sum(len(v) for v in notes.values())
    n_paths = sum(len(r[5]) for r in rows)
    print(f"upstream ledger: {len(rows)} rows ({len(by_id)} ids), {len(STANDINS)} with a stand-in, {total} UPSTREAM "
          f"markers in src/ and tests/, {n_paths} filed paths; {len(fails)} failure(s)")
    return 1 if (args.check and fails) else 0


if __name__ == "__main__":
    sys.exit(main())
