"""Compiles and runs every ```cyrius code block in the docs, and every ported example, against the shipped bundle.

The Cyrius form of Rust's doc tests and `cargo run --example` (port plan §4 step 7; M10: "doc examples compiled and
run"). pravash's scripts/doc_examples.py (2.0.1; impetus's, with docs/examples/) with soorat's names. Usage, from the
repo root, after `cyrius distlib` has written dist/soorat.cyr:

    python3 -I -B scripts/doc_examples.py .            # build and run them all; exit 1 on a failure
    python3 -I -B scripts/doc_examples.py . --check    # the same (the tool is always a check; CI passes --check)

**What runs.** Every fence whose info string is exactly `cyrius` in README.md and in docs/**/*.md (the plan, the
ADRs and the guides included), and every docs/examples/<name>.cyr. A fence that should not run takes another info
string (`cyr`, `cyrius ignore`, `text`, ...). A block that defines `fn main` is a whole consumer program: its
`include "lib/soorat.cyr"` is pointed at `dist/soorat.cyr`, where a consumer's `cyrius deps` would have vendored the
bundle. Any other block is a fragment: it goes into `fn main(): i64 { alloc_init(); <block> return 0; }` after the
bundle's include. Each docs/examples/<name>.cyr is a whole program too, and must print exactly
docs/examples/<name>.out (recorded from Rust 1.2.0's example, rust-old/examples/, when it is ported), be formatted
(`cyrius fmt --check`), and lint clean (`cyrius lint` on the built copy, whose include points at dist/).

Each program is built with `cyrius build` into build/doc_examples/ and run. A compile warning, a failed build, a
nonzero exit, an example's other output, formatting or lint fails it: exit 1, naming the block. With nothing to run
(today: the docs hold no ```cyrius block and docs/examples/ no program) it reports 0 examples and exits 0, needing
neither `cyrius` nor the bundle. Stdlib only.
"""
import argparse
import re
import subprocess
import sys
from pathlib import Path

BUNDLE = "dist/soorat.cyr"
CONSUMER_INCLUDE = 'include "lib/soorat.cyr"'
TRAILER = "\nvar r = main();\nsys_exit_group(r);\n"


def blocks(text):
    """(line number, code) of each ```cyrius block."""
    out = []
    lines = text.split("\n")
    i = 0
    while i < len(lines):
        if lines[i].strip() == "```cyrius":
            start = i + 1
            j = start
            while j < len(lines) and lines[j].strip() != "```":
                j += 1
            out.append((start + 1, "\n".join(lines[start:j])))
            i = j
        i += 1
    return out


def program(code):
    """The block as a whole program, or None with a reason."""
    if re.search(r"^fn main\(", code, re.M):
        src = code.replace(CONSUMER_INCLUDE, f'include "{BUNDLE}"')
        if BUNDLE not in src:
            return None, f"a whole program must {CONSUMER_INCLUDE}"
        if "sys_exit_group(" not in src:
            src += TRAILER
        return src, None
    body = "\n".join("    " + ln if ln else "" for ln in code.split("\n"))
    return f'include "{BUNDLE}"\n\nfn main(): i64 {{\n    alloc_init();\n{body}\n    return 0;\n}}\n' + TRAILER, None


def build_and_run(root, label, src, out_dir):
    """Builds and runs one program: (stdout, None), or (None, why it failed)."""
    stem = re.sub(r"[^A-Za-z0-9]+", "_", label)
    cyr, exe = out_dir / f"{stem}.cyr", out_dir / stem
    cyr.write_text(src, encoding="utf-8")
    b = subprocess.run(["cyrius", "build", str(cyr.relative_to(root)), str(exe.relative_to(root))],
                       cwd=root, capture_output=True, text=True)
    log = (b.stdout + b.stderr).split("\n")
    warn = [ln.strip() for ln in log if "warning" in ln.lower()]
    if b.returncode != 0 or warn:
        errs = [ln.strip() for ln in log if "error" in ln.lower()]
        detail = warn if b.returncode == 0 else (errs or [ln for ln in log if ln.strip()][-3:])
        what = "warned" if b.returncode == 0 else "failed"
        return None, f"build {what}: " + " | ".join(detail[:3])
    r = subprocess.run([str(exe)], cwd=root, capture_output=True, text=True, timeout=300)
    if r.returncode != 0:
        return None, f"exit {r.returncode}"
    return r.stdout, None


def style(root, ex, out_dir):
    """None when the example is formatted (`cyrius fmt --check` on the file as written) and lint-clean (`cyrius lint`
    on the built copy, since lint compiles the file and `lib/soorat.cyr` exists only in a consumer); else why not."""
    f = subprocess.run(["cyrius", "fmt", "--check", ex], cwd=root, capture_output=True, text=True)
    if f.returncode != 0:
        return "not formatted (cyrius fmt --check)"
    copy = out_dir / (re.sub(r"[^A-Za-z0-9]+", "_", ex) + ".cyr")
    lint = subprocess.run(["cyrius", "lint", str(copy.relative_to(root))], cwd=root, capture_output=True, text=True)
    log = (lint.stdout + lint.stderr).split("\n")
    if lint.returncode != 0 or not any(ln.strip() == "0 warnings" for ln in log):
        return "lint: " + " | ".join(ln.strip() for ln in log if ln.strip())[-200:]
    return None


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("root", nargs="?", default=".", help="the repository root (default: .)")
    ap.add_argument("--check", action="store_true", help="accepted for CI symmetry: the tool always checks")
    args = ap.parse_args()
    root = Path(args.root).resolve()
    docs = (["README.md"] if (root / "README.md").exists() else []) + \
        sorted(str(p.relative_to(root)) for p in (root / "docs").rglob("*.md"))
    examples = sorted(str(p.relative_to(root)) for p in (root / "docs/examples").glob("*.cyr"))
    found = [(doc, line, code) for doc in docs for line, code in blocks((root / doc).read_text(encoding="utf-8"))]
    total = len(found) + len(examples)
    failed = []
    if total and not (root / BUNDLE).exists():
        failed.append(f"no {BUNDLE}: run `cyrius distlib` first ({total} block(s) and program(s) wait on it)")
    elif total:
        out_dir = root / "build/doc_examples"
        out_dir.mkdir(parents=True, exist_ok=True)
        for doc, line, code in found:
            label = f"{doc}:{line}"
            src, why = program(code)
            if src is None:
                failed.append(f"{label}: {why}")
                continue
            _, why = build_and_run(root, label, src, out_dir)
            if why is not None:
                failed.append(f"{label}: {why}")
        for ex in examples:
            src, why = program((root / ex).read_text(encoding="utf-8"))
            if src is None:
                failed.append(f"{ex}: {why}")
                continue
            out, why = build_and_run(root, ex, src, out_dir)
            want = root / ex.replace(".cyr", ".out")
            if why is None:
                why = style(root, ex, out_dir)
            if why is not None:
                failed.append(f"{ex}: {why}")
            elif not want.exists():
                failed.append(f"{ex}: no {want.relative_to(root)} to compare with")
            elif out != want.read_text(encoding="utf-8"):
                got, exp = out.split("\n"), want.read_text(encoding="utf-8").split("\n")
                k = next((i for i, (a, b) in enumerate(zip(got, exp)) if a != b), min(len(got), len(exp)))
                failed.append(f"{ex}: prints otherwise than {want.relative_to(root)} at line {k + 1}")
    for f in failed:
        print("  DOC", f)
    ran = 0 if (total and not (root / BUNDLE).exists()) else total - len(failed)
    print(f"doc examples: {total} examples — {len(found)} ```cyrius blocks in {len(docs)} docs, {len(examples)} "
          f"programs in docs/examples/; {ran} ran, {len(failed)} failed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
