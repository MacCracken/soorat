"""Vendors kimiya's correctly rounded f64 `sin`, `cos` and `acos` into src/crmath.cyr, and `exp` and `log10` into
src/crmath_ext.cyr (upstream ledger L1's stand-in).

Port plan §5 ("correctly rounded stand-ins on paths parity checks") and §10 L1: ganita's f64 tier is not correctly
rounded (its `acos` is up to 2 ulp from glibc's on 21.8 % of a dense [0, 1] grid, its `sin` 1 ulp on 2.6 %:
port-parity §4.5), and slerp's class-E rows need the correctly rounded value (ADR 0002 #24). kimiya 2.0.1 has both,
two-phase (Ziv) on MPFR-generated tables, from + − × ÷, `f64_sqrt` and integer bit operations only.

This script copies the dependency closure of `_km_cr_sin`, `_km_cr_acos` and `_km_cr_cos` (functions with their doc
comments, the
tables of src/mathx_data.cyr and the constants of src/mathx.cyr they read) in kimiya's source order, renaming `_km_` →
`_sr_km_` and `_KM_` → `_SR_KM_` so nothing collides with a consumer that includes kimiya too. src/crmath_ext.cyr is
the closure of `_km_cr_exp` and `_km_cr_log10` less what src/crmath.cyr already holds: the acoustic decay curve's
class-E rows need them (M5), and only the `acoustics` profile includes it, because its tables cost a binary ~33 KB.
`cos` (the balloon, the thermal network) is one function on `sin`'s tables, so it is in src/crmath.cyr. Usage, from
the repo root:

    python3 -I -B scripts/vendor_crmath.py .            # write src/crmath.cyr and src/crmath_ext.cyr from ../kimiya
    python3 -I -B scripts/vendor_crmath.py . --check    # exit 1 when either differs from what ../kimiya gives

Without ../kimiya (CI checks out soorat alone), `--check` reports "unverified" and exits 0. Stdlib only.
"""
import os
import re
import subprocess
import textwrap
import sys

ROOT = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].startswith("-") else ".")
CHECK = "--check" in sys.argv
KIMIYA = os.path.join(os.path.dirname(ROOT), "kimiya")
VERSION = "2.0.1"
ROOTS = ["_km_cr_sin", "_km_cr_acos", "_km_cr_cos"]
OUT = os.path.join(ROOT, "src", "crmath.cyr")
EXP_ROOTS = ["_km_cr_exp", "_km_cr_log10"]
EXP_OUT = os.path.join(ROOT, "src", "crmath_ext.cyr")


def items(text, kind):
    """{name: (start, end)} of the top-level `fn` or `var` items of text, each starting at its doc comment."""
    out = {}
    lines = text.split("\n")
    offs = [0]
    for ln in lines:
        offs.append(offs[-1] + len(ln) + 1)
    pat = re.compile({"fn": r"^fn (\w+)\(", "var": r"^var (\w+)", "const": r"^const (\w+)"}[kind])
    for i, ln in enumerate(lines):
        m = pat.match(ln)
        if not m:
            continue
        j = i
        while j > 0 and lines[j - 1].startswith("#"):
            j -= 1
        if kind == "fn":
            k = i
            while lines[k] != "}":
                k += 1
        else:
            k = i
            if "{" in ln and "}" not in ln:
                while not lines[k].startswith("}"):
                    k += 1
        out[m.group(1)] = (offs[j], offs[k + 1])
    return out


def main():
    src_path = os.path.join(KIMIYA, "src", "mathx.cyr")
    data_path = os.path.join(KIMIYA, "src", "mathx_data.cyr")
    if not os.path.exists(src_path):
        if CHECK:
            print("vendor_crmath: unverified (../kimiya is not checked out)")
            return 0
        print("vendor_crmath: ../kimiya not found", file=sys.stderr)
        return 1
    with open(os.path.join(KIMIYA, "VERSION"), encoding="utf-8") as f:
        version = f.read().strip()
    if version != VERSION:
        print(f"vendor_crmath: ../kimiya is {version}, this script vendors {VERSION}", file=sys.stderr)
        return 1
    with open(src_path, encoding="utf-8") as f:
        src = f.read()
    with open(data_path, encoding="utf-8") as f:
        data = f.read()
    fns = items(src, "fn")
    dvars = items(data, "var")
    consts = items(src, "const")

    def closure(roots, have):
        """The functions, tables and constants `roots` reach, less those in `have`, each in kimiya's order."""
        seen, need, cneed, stack = set(), set(), set(), list(roots)
        while stack:
            name = stack.pop()
            if name in seen:
                continue
            seen.add(name)
            a, b = fns[name]
            body = src[src.index("fn " + name + "(", a):b]
            for ref in re.findall(r"\b(_[kK][mM]_\w+)\b", body):
                if ref in fns and ref not in seen:
                    stack.append(ref)
                elif ref in dvars:
                    need.add(ref)
                elif ref in consts:
                    cneed.add(ref)
        return (sorted(seen - have, key=lambda n: fns[n][0]), sorted(need - have, key=lambda n: dvars[n][0]),
                sorted(cneed - have, key=lambda n: consts[n][0]))

    def render(head, order_fns, order_vars, order_consts):
        body = "".join(data[dvars[n][0]:dvars[n][1]] + "\n" for n in order_vars)
        if order_consts:
            body += "# ── constants (kimiya src/mathx.cyr) ─────────────────────────────────────────────────\n\n"
            body += "".join(src[consts[n][0]:consts[n][1]] + "\n" for n in order_consts)
        body += "# ── functions (kimiya src/mathx.cyr) ─────────────────────────────────────────────────\n\n"
        body += "".join(src[fns[n][0]:fns[n][1]] + "\n" for n in order_fns)
        body = re.sub(r"\b_km_", "_sr_km_", body)
        body = re.sub(r"\b_KM_", "_SR_KM_", body)
        return ("\n".join(head) + body).rstrip("\n") + "\n"

    try:
        commit = subprocess.run(["git", "-C", KIMIYA, "rev-parse", "--short", VERSION], capture_output=True,
                                text=True, check=True).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        commit = "unknown"
    order_fns, order_vars, order_consts = closure(ROOTS, set())
    head = [
        "# crmath.cyr — correctly rounded f64 sin, cos and acos, vendored from kimiya " + VERSION + " (tag " + VERSION + ", " + commit + ")",
        "# src/mathx.cyr and src/mathx_data.cyr. GENERATED by scripts/vendor_crmath.py: do not edit.",
        "# Regenerate: python3 -I -B scripts/vendor_crmath.py .   (--check verifies this file against ../kimiya)",
        "#",
    ] + textwrap.wrap(
        "Upstream ledger L1's stand-in (port plan §10): ganita's f64 tier is not correctly rounded, and class-E rows need "
        "the correctly rounded value: slerp's sin and acos (ADR 0002 #24), the directivity balloon's and the thermal "
        "network's sin and cos (M5). `_sr_km_cr_sin(x)`, `_sr_km_cr_cos(x)` and `_sr_km_cr_acos(x)` (f64 → f64) are "
        "kimiya's `_km_cr_sin`, `_km_cr_cos` and `_km_cr_acos` renamed (`_km_` → `_sr_km_`, `_KM_` → `_SR_KM_`), with "
        "the " + str(len(order_fns)) + " functions and " + str(len(order_vars)) + " tables they reach. Each is "
        "two-phase (Ziv): a fast path with a proven error bound and a rounding test, then a triple-double accurate "
        "path. The ADR and section numbers in the comments below are kimiya's. kimiya is GPL-3.0-only, as soorat is.",
        width=118, initial_indent="# ", subsequent_indent="# ", break_on_hyphens=False) + [
        "",
        "# ── tables (kimiya src/mathx_data.cyr, generated there from MPFR 4.2.2) ──────────────",
        "",
    ]
    outs = [(OUT, render(head, order_fns, order_vars, order_consts), len(order_fns), len(order_vars))]
    have = set(order_fns) | set(order_vars) | set(order_consts)
    e_fns, e_vars, e_consts = closure(EXP_ROOTS, have)
    e_head = [
        "# crmath_ext.cyr — correctly rounded f64 exp and log10, vendored from kimiya " + VERSION + " (tag " + VERSION + ", " + commit + ")",
        "# src/mathx.cyr and src/mathx_data.cyr. GENERATED by scripts/vendor_crmath.py: do not edit.",
        "# Regenerate: python3 -I -B scripts/vendor_crmath.py .   (--check verifies this file against ../kimiya)",
        "#",
    ] + textwrap.wrap(
        "The acoustic decay curve's class-E rows need the correctly rounded exp and log10 (M5); ganita's f64 tier is "
        "not correctly rounded (upstream ledger L1). `_sr_km_cr_exp(x)` and `_sr_km_cr_log10(x)` (f64 → f64) are "
        "kimiya's functions renamed as src/crmath.cyr's are, with the " + str(len(e_fns)) + " functions, " + str(len(e_vars)) + " tables and " + str(len(e_consts))
        + (" constant" if len(e_consts) == 1 else " constants") + " they reach that src/crmath.cyr doesn't hold. "
        "**Include src/crmath.cyr first** (it is in `[lib]`). Only the `acoustics` profile includes this file: its "
        "tables cost a binary about 33 KB.",
        width=118, initial_indent="# ", subsequent_indent="# ", break_on_hyphens=False) + [
        "",
        "# ── tables (kimiya src/mathx_data.cyr, generated there from MPFR 4.2.2) ──────────────",
        "",
    ]
    outs.append((EXP_OUT, render(e_head, e_fns, e_vars, e_consts), len(e_fns), len(e_vars)))
    if CHECK:
        bad = 0
        for path, text, nf, nv in outs:
            try:
                with open(path, encoding="utf-8") as f:
                    old = f.read()
            except FileNotFoundError:
                old = None
            if old != text:
                print(f"vendor_crmath: {os.path.relpath(path, ROOT)} is stale (regenerate: python3 -I -B scripts/vendor_crmath.py .)")
                bad += 1
        if bad:
            return 1
        print(f"vendor_crmath: ok (kimiya {VERSION}: {outs[0][2]} functions, {outs[0][3]} tables; ext: {outs[1][2]} functions, {outs[1][3]} tables)")
        return 0
    for path, text, nf, nv in outs:
        with open(path, "w", encoding="utf-8") as f:
            f.write(text)
        print(f"wrote {path}: {nf} functions, {nv} tables, {text.count(chr(10))} lines")
    return 0


if __name__ == "__main__":
    sys.exit(main())
