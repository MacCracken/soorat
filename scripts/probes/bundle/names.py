#!/usr/bin/env python3
"""Static name scans for soorat M0 probes P4 and P11 (soorat port plan §4 step 1).

The compiles are the measurement; this is the cross-check that reads the bundles as text.

  python3 -I names.py defs FILE...            top-level definitions per file (fn, var, const, enum member, struct)
  python3 -I names.py p4 HISAB_OLD HISAB_NEW CONSUMER...
        names HISAB_OLD defines and HISAB_NEW does not (removed), and the reverse (added); then, per consumer,
        every identifier it uses from either set, and every assignment to one of 3.5.0's enum-ified constants
  python3 -I names.py prefixes FILE...        every top-level name soorat's plan reserves or must avoid: sr_*, Sr<Upper>*,
                                              SR_*, soorat* / Soorat* / SOORAT* (case-sensitive)
  python3 -I names.py shared FILE...          names defined at top level in more than one file (fn and globals),
                                              which the compiler reports only when they conflict
"""
import re
import sys

ID = r"[A-Za-z_][A-Za-z0-9_]*"


def strip(text):
    """Drops comments and string/char literals, keeping line structure. `#` starts a comment unless a
    directive or attribute letter follows it (#define, #ifdef, #must_use, #derive, ...)."""
    out = []
    for line in text.split("\n"):
        buf = []
        i = 0
        n = len(line)
        while i < n:
            c = line[i]
            if c == '"':
                j = i + 1
                while j < n and line[j] != '"':
                    j += 2 if line[j] == "\\" else 1
                buf.append('""')
                i = j + 1
                continue
            if c == "'" and i + 2 < n and (line[i + 2] == "'" or line[i + 1] == "\\"):
                j = line.find("'", i + 2 if line[i + 1] != "\\" else i + 3)
                if j < 0:
                    j = n
                buf.append("0")
                i = j + 1
                continue
            if c == "#" and not (i + 1 < n and (line[i + 1].isalpha() or line[i + 1] == "@")):
                break
            buf.append(c)
            i += 1
        out.append("".join(buf))
    return out


def defs(path):
    """Top-level definitions: {name: kind}. Only column-0 (or `public`/`private`/attribute-prefixed)
    declarations count, plus members of top-level enums."""
    lines = strip(open(path, encoding="utf-8", errors="replace").read())
    found = {}
    depth = 0
    in_enum = False
    for ln in lines:
        s = ln.strip()
        if depth == 0:
            m = re.match(r"^(?:#\w+\s+)*(?:public\s+|private\s+)?(?:#\w+\s+)*(?:const\s+)?fn\s+(" + ID + r")", s)
            if m:
                found.setdefault(m.group(1), "fn")
            m = re.match(r"^(?:public\s+|private\s+)?(var|const)\s+(" + ID + r")", s)
            if m:
                found.setdefault(m.group(2), m.group(1))
            m = re.match(r"^(?:public\s+|private\s+)?(?:packed\s+)?struct\s+(" + ID + r")", s)
            if m:
                found.setdefault(m.group(1), "struct")
            m = re.match(r"^(?:public\s+|private\s+)?enum\s+(" + ID + r")", s)
            if m:
                found.setdefault(m.group(1), "enum")
                in_enum = True
            m = re.match(r"^#define\s+(" + ID + r")", s)
            if m:
                found.setdefault(m.group(1), "define")
        elif depth == 1 and in_enum:
            for m in re.finditer(r"(?:^|[;,{]\s*)(" + ID + r")\s*(?:=|;|,|$)", s):
                found.setdefault(m.group(1), "enum member")
        depth += ln.count("{") - ln.count("}")
        if depth <= 0:
            depth = 0
            in_enum = False
    return found


def idents(path):
    used = {}
    for no, ln in enumerate(strip(open(path, encoding="utf-8", errors="replace").read()), 1):
        for m in re.finditer(ID, ln):
            used.setdefault(m.group(0), []).append(no)
    return used


ENUMIFIED = """EPSILON_F64 F64_TINY F64_POS_INF GEO_FACE_NONE GEO_FACE_OUT_BYTES GEO_CAP_CYLINDER GEO_CAP_START_CAP
GEO_CAP_END_CAP GEO_CAP_SPHERE GJK_MAX_ITER EPA_MAX_ITER CGA_NUM_BLADES CGA_MV_SIZE HALFEDGE_NONE AD_NONE
AD_FULL""".split()


def p4(old, new, consumers):
    d_old, d_new = defs(old), defs(new)
    fn_old = {k for k, v in d_old.items() if v == "fn"}
    fn_new = {k for k, v in d_new.items() if v == "fn"}
    removed = sorted(fn_old - fn_new)
    added = sorted(set(d_new) - set(d_old))
    print(f"{old}: {len(d_old)} top-level names, {len(fn_old)} fns")
    print(f"{new}: {len(d_new)} top-level names, {len(fn_new)} fns")
    print(f"fns in old, not in new ({len(removed)}): {' '.join(removed)}")
    print(f"names in new, not in old: {len(added)}")
    for c in consumers:
        used = idents(c)
        own = defs(c)
        hit_rm = {k: v for k, v in used.items() if k in removed and k not in own}
        hit_add = {k: v for k, v in used.items() if k in added and k not in own and k not in d_old}
        print(f"\n== {c}")
        print(f"  uses a fn removed in new: {len(hit_rm)}" + "".join(f"\n    {k} lines {v[:6]}" for k, v in sorted(hit_rm.items())))
        print(f"  uses a name only new defines: {len(hit_add)}" + "".join(f"\n    {k} lines {v[:6]}" for k, v in sorted(hit_add.items())))
        lines = strip(open(c, encoding="utf-8", errors="replace").read())
        assigns = [(no, ln.strip()) for no, ln in enumerate(lines, 1)
                   for k in ENUMIFIED if re.search(r"(?<![\w.])" + k + r"\s*=(?!=)", ln) and not re.match(r"\s*(var|const)\b", ln)]
        print(f"  assigns an enum-ified constant: {len(assigns)}" + "".join(f"\n    {no}: {t}" for no, t in assigns))
        w2s = [(no, ln.strip()) for no, ln in enumerate(lines, 1) if "world_to_screen_checked(" in ln]
        print(f"  world_to_screen_checked call sites: {len(w2s)}" + "".join(f"\n    {no}: {t}" for no, t in w2s))
        mu = [(no, ln.strip()) for no, ln in enumerate(lines, 1) if re.search(r"\bspatial_hash_(insert|clear)\(", ln)]
        print(f"  spatial_hash_insert/clear call sites: {len(mu)}" + "".join(f"\n    {no}: {t}" for no, t in mu))


def prefixes(files):
    pat = re.compile(r"^(sr_|Sr[A-Z_0-9]|SR_|soorat|Soorat|SOORAT)")
    for f in files:
        names = sorted(k for k in defs(f) if pat.match(k))
        print(f"{f}: {len(names)}" + ("\n  " + " ".join(names) if names else ""))


def shared(files):
    where = {}
    for f in files:
        for k, v in defs(f).items():
            where.setdefault(k, []).append((f, v))
    multi = {k: v for k, v in where.items() if len({f for f, _ in v}) > 1}
    for k in sorted(multi):
        print(k + ": " + ", ".join(f"{f.rsplit('/', 1)[-1]} ({v})" for f, v in multi[k]))
    print(f"-- {len(multi)} names defined in more than one file")


if __name__ == "__main__":
    cmd, args = sys.argv[1], sys.argv[2:]
    if cmd == "defs":
        for f in args:
            d = defs(f)
            kinds = {}
            for v in d.values():
                kinds[v] = kinds.get(v, 0) + 1
            print(f, len(d), kinds)
    elif cmd == "p4":
        p4(args[0], args[1], args[2:])
    elif cmd == "prefixes":
        prefixes(args)
    elif cmd == "shared":
        shared(args)
    else:
        sys.exit(__doc__)
