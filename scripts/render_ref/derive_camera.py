#!/usr/bin/env python3
"""Derive a camera block for tests/golden/render/scenes.json.

Usage: derive_camera.py NAME EYE_X EYE_Y EYE_Z TGT_X TGT_Y TGT_Z FOVY_DEG NEAR FAR

The matrices are column-major (index = col * 4 + row), right-handed, with
wgpu's [0, 1] clip depth:

  view       = look_at(eye, target, up = +Y)
               f = normalize(target - eye), s = normalize(f x up), u = s x f
               rows: (s, -s.eye), (u, -u.eye), (-f, f.eye), (0, 0, 0, 1)
  projection = perspective(fovy, aspect 1, near, far), g = 1 / tan(fovy / 2):
               columns (g 0 0 0), (0 g 0 0), (0 0 far/(near-far) -1), (0 0 near*far/(near-far) 0)
  inv_projection = the closed-form inverse of `projection`
  view_proj  = projection * view

Everything is computed in f64 and each entry is rounded once to f32. The
printed numbers are the *exact* decimal values of those f32s, so any correctly
rounding parser recovers the same bits (render_ref checks this on load). Once
pasted into scenes.json the matrices are the definition; this script only
records how they were first produced.
"""

import math
import struct
import sys
from decimal import Decimal


def exact(x: float) -> str:
    """The exact decimal of x rounded to f32 (integers without a fraction)."""
    v = struct.unpack("<f", struct.pack("<f", x))[0]
    if v == 0.0:
        return "0"
    if v == int(v):
        return str(int(v))
    return format(Decimal(v), "f")


def dot(a, b):
    return sum(a[i] * b[i] for i in range(3))


def cross(a, b):
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]


def normalize(a):
    n = math.sqrt(dot(a, a))
    return [c / n for c in a]


def mul(p, v):
    """Column-major 4x4 product p * v."""
    return [
        sum(p[k * 4 + r] * v[c * 4 + k] for k in range(4)) for c in range(4) for r in range(4)
    ]


def main(argv):
    if len(argv) != 11:
        print(__doc__, file=sys.stderr)
        return 2
    name = argv[1]
    eye = [float(x) for x in argv[2:5]]
    target = [float(x) for x in argv[5:8]]
    fovy, near, far = (float(x) for x in argv[8:11])

    f = normalize([target[i] - eye[i] for i in range(3)])
    s = normalize(cross(f, [0.0, 1.0, 0.0]))
    u = cross(s, f)
    view = [
        s[0], u[0], -f[0], 0.0,
        s[1], u[1], -f[1], 0.0,
        s[2], u[2], -f[2], 0.0,
        -dot(s, eye), -dot(u, eye), dot(f, eye), 1.0,
    ]
    g = 1.0 / math.tan(math.radians(fovy) / 2.0)
    a = far / (near - far)
    b = near * far / (near - far)
    proj = [g, 0.0, 0.0, 0.0, 0.0, g, 0.0, 0.0, 0.0, 0.0, a, -1.0, 0.0, 0.0, b, 0.0]
    # x = x'/g, y = y'/g, z = -w', w = (z' - a w') / b
    inv = [1 / g, 0.0, 0.0, 0.0, 0.0, 1 / g, 0.0, 0.0, 0.0, 0.0, 0.0, 1 / b, 0.0, 0.0, -1.0, a / b]

    def vec(xs):
        return "[" + ", ".join(exact(x) for x in xs) + "]"

    fields = [
        ("eye", vec(eye)),
        ("target", vec(target)),
        ("fovy_deg", exact(fovy)),
        ("near", exact(near)),
        ("far", exact(far)),
        ("view", vec(view)),
        ("projection", vec(proj)),
        ("inv_projection", vec(inv)),
        ("view_proj", vec(mul(proj, view))),
    ]
    print(f'"{name}": {{')
    for i, (k, v) in enumerate(fields):
        print(f'  "{k}": {v}' + ("," if i + 1 < len(fields) else ""))
    print("}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
