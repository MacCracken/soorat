# 0002 — API divergences between Rust 1.2.0 and the Cyrius 2.0.0 port

**Status**: Accepted (rows are added as milestones find them; never renumbered)
**Date**: 2026-11-15

## Context

The Cyrius port reproduces Rust 1.2.0's behaviour ([ADR 0001](0001-cyrius-port-with-rust-oracle.md)), but
some of 1.2.0's API can't, or shouldn't, carry over:
- Cyrius has no wgpu types, no `Vec<T>` of structs, no data-carrying enums without allocation, and no module
  paths, in one flat namespace;
- soorat reaches the GPU only through mabda;
- some 1.2.0 behaviours are defects that need public-type changes to fix;
- the probes found inputs on which 1.2.0 panics, so there is no verdict to copy.

There is no Rust 2.0, so API fixes land in the Cyrius API only, as kimiya's and pravash's did.

## Decision

The port diverges from 1.2.0 exactly where this table says, and nowhere else. Every row says how the parity
harness still compares the function, so nothing leaves parity entirely. Rows 1–12 were seeded by the plan
(§3.3); 13–15 were added by M0's decisions and probes (2026-11-15).

| # | Rust 1.2.0 | Cyrius 2.0.0 | Comparison |
|---|---|---|---|
| 1 | `Result<T, RenderError>` with detail strings | value-form `Result` and an `SR_ERR_*` code | class A on the variant |
| 2 | soorat creates the device and window (`Window::new`) | the application passes a mabda `GpuContext` and a surface (D5) | — |
| 3 | wgpu types in the API | mabda handles | — |
| 4 | Matrices and geometry in f32 arithmetic | f64 arithmetic, rounded once to f32 (D4 rule b) | class E against Rust; ≤ 0.5 ulp against the f64 reference |
| 5 | Science inputs as Rust types (f32 in goonj) | the Cyrius producers' f64 types (§2.5) | the oracle feeds f32-representable inputs to both sides |
| 6 | `Vec` returns | caller buffers or a frame arena, plus explicit `_drop` | same contents (class A on counts) |
| 7 | One model matrix per `MeshPipeline` and `ShadowPipeline` (backlog: per-draw uniforms) | per-draw model uniforms through dynamic offsets (D2: an API fix in Cyrius only) | class G on multi-mesh scenes, against Rust renders drawn one mesh per pass |
| 8 | `MeshPipeline::draw` always clears depth | a load/clear choice on the draw parameters (D2) | class G |
| 9 | Cargo features | one bundle with dead-code elimination; each science crate an optional dependency | — |
| 10 | `egui_bridge` | removed (D8) | n/a |
| 11 | `drm` | agnodrm's subset (D8) | class A on what remains |
| 12 | `em_render` axis convention (if ported in 2.0, D7) | one convention for 2D data | rows mapped by name |
| 13 | CUBICSPLINE samplers keep each key's value and drop its tangents, so playback is linear (slerp for rotations) between keys | channels carry their interpolation mode and the in/out tangents, and CUBICSPLINE plays back as the glTF cubic Hermite spline (rotations normalized) | **class R** against the glTF spec formula, evaluated independently in f64. The 1.2.0 value-only rows stay for LINEAR and STEP, and for CUBICSPLINE at key times, where both agree |
| 14 | glTF verdicts on malformed input: 4 inputs that 1.2.0 accepts and bayan refuses (duplicate keys and an integer above i64::MAX in `extras`, both open bayan issues; nesting deeper than 128); 4 inputs that **panic** Rust 1.2.0 (`byteStride: 4` on POSITION, an out-of-range POSITION accessor, a float index type, an unknown target path); and document refusals D6 didn't list (POSITION without min/max, non-object `extras`, an unknown semantic, `extensionsRequired`, an invalid interpolation, mode > 6) | each is a defined `SR_ERR_*`; joint TRS numbers parse correctly rounded (bayan: 4,000 of 4,000; serde_json: 3,959) | class A on every other fixture; these rows are named in the harness (P9)
| 15 | `image` decodes up to its 512 MiB limit | chitra caps an image at 16,777,216 pixels, and the decode goes through a freeing allocator (`chitra_image_decode_budget_via`) | larger images are refused with `SR_ERR_*`; PNG/BMP class A, JPEG B-img (P10) |

## Consequences

- **Positive.** Every divergence is named, justified and still checked; the harness lists the affected rows
  by number instead of widening a tolerance.
- **Negative.** Consumers migrating from 1.2.0 must read this table. The migration guide (M10) walks through
  it, led by row 2, the window.
- **Neutral.** New rows arrive as milestones port. A row that turns out avoidable is struck through, not
  removed.

## Alternatives considered

- **Bit-for-bit API compatibility.** Impossible for rows 2, 3, 6 and 9 (language and dependency facts), and
  undesirable for 7, 8 and 13, which are defects.
- **A Rust 2.0 carrying the same changes.** Rejected (no Rust 2.0): it would double the porting work for
  consumers who are moving to Cyrius anyway.
