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
(§3.3); 13–16 were added by M0's decisions, probes and harness (2026-11-15).

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
| 16 | f32 `sin`, `cos`, `powf` and `expf` come from host glibc, which misrounds 0.73 % of soorat's inputs, each by 1 ulp (probe P3, port-parity §1.3) | correctly rounded: ganita's f32 tier (widen, compute, narrow) gave the reference value on all 226,050 of soorat's inputs; `acos` through `ganita_f64_acos`; `log10` through kimiya's correctly rounded `crlog` (stand-in, L1) | class C, K = 0, against `<module>.cr.tsv` where glibc misrounds; a pass is *accepted (cr)* |
| 17 | `tracing` events under `soorat::<module>`, with fields (`near=1.0 far=1.0`), for the application's subscriber | sakshi events at the same level, `soorat::<module>: <message>` (src/log.cyr); the field values are left out until soorat formats f32 as Rust's shortest round trip does, which the first module that needs it brings | not compared: log text is not API |
| 18 | `optics` Cargo feature: `Color::from_prakash(Rgb, alpha)`, `to_prakash() -> Rgb`, `from_temperature`, `from_wavelength -> Option` | an optional prakash 2.15.2 (`[features] optics`), its code in `src/color_optics.cyr`, outside `[lib]` and bundled by `[lib.optics]` as `dist/soorat-optics.cyr` (ranga's `spectral` precedent). `to_prakash` writes into an `Rgb` the caller owns; `from_wavelength` returns a value-form `Option` (Some(out) / None). `from_temperature` and `from_wavelength` leave prakash's 24-byte `Rgb` on the bump heap per call (ledger L21) | class A on all 1,704 rows of `tests/golden/color_optics.tsv`, replayed by its own harness (`tests/parity_features.tcyr`) so the core replay compiles without prakash |
| 19 | `#[derive(Serialize, Deserialize)]` on `Color`, read and written by serde_json (the tests' `to_string` / `from_str`), with `serde_json::Error` | `SrColor_to_json(c, sb)` and `SrColor_from_json(buf, len, out)` (`src/json.cyr`, on bayan; pravash's ADR 0002 #6 and #10 are the precedent). **Opt-in**, unlike Rust's unconditional derive: the `[lib.serde]` profile (`dist/soorat-serde.cyr`), because including bayan costs a binary ~372 KB of unicode tables (soorat's smoke program: 49 KB → 421 KB; bayan's open issue `2026-10-31-bundle-carries-unicode-tables-into-every-binary.md`). the Err the Cyrius-only `SR_ERR_DESERIALIZE` (appended as 12). The writer prints an f32 as bayan's round-trip digits of its exact f64 (serde_json prints its shortest), so the text differs and every document still reads back to the same bits in both. Three documents serde_json reads, bayan's tree can't, by its open issues: the integer `-0` (read as +0), an integer above `i64::MAX` and a duplicated unknown field (both refused) | class A on the decoded values of `tests/golden/json.tsv` (177 rows); the three bayan cases are rows of their own (`Color::from_json_bayan_*`), pending until bayan changes; every decode round-trips through the writer (`tests/color.tcyr`) |
| 20 | `SpriteBatch { sprites: Vec<Sprite> }`, grown by `Vec` and sorted by `sort_by_key` (stable, adaptive); `batch_to_vertices` returning `(Vec<Vertex2D>, Vec<u16>)`, or refilling the caller's `Vec`s (`_into`) | `SrSpriteBatch`: a freelist-backed array (doubling from 4; `clear` keeps the capacity) freed by `SrSpriteBatch_drop`. `sort_by_z` sorts the keys `(z << 32) \| index`, distinct and so stable by construction, returns after one pass on sorted input, and reuses its scratch arrays. `sr_batch_to_vertices(_u32)(b, a, out)` allocate from the caller's allocator (a frame arena) into an `SrQuadData`; the `_into` forms write caller buffers sized by `sr_batch_sprite_count`. `SrSprite`'s fields are reordered for alignment (68 bytes; no GPU layout) | class A on the sort permutations and the quads (`tests/golden/sprite.tsv`; the limit batches as FNV-1a digests); class C at 0 ulp for rotated quads against `sprite.cr.tsv` |
| 21 | `TextBatch::draw_text(&str, …)` walking `chars()`; `UiLabel { text: String, … }` | text is a pointer and a byte length, decoded as UTF-8 (`_sr_utf8_next`): a byte that begins no well-formed sequence (overlong, surrogate, past U+10FFFF, truncated) counts as one character U+FFFD, which a Rust `&str` can't hold. A `SrUiLabel` **borrows** its text (pointer and length), which must outlive the label's use; Rust's owns a `String` | class A on the glyph rectangles and the sprites of every text and UI batch (`tests/golden/text.tsv`, all valid UTF-8); the ill-formed cases are Cyrius-only tests |
| 22 | `physics-debug` Cargo feature: `LineBatch::collider(&impetus::collider::ColliderShape, …)` | an optional impetus 2.0.0 (3D bundle, `[features] physics-debug`), the code in `src/debug_collider.cyr`, bundled by `[lib.physics-debug]` as `dist/soorat-physics-debug.cyr`; the shape is impetus's `ImColliderShape` | class C at 0 ulp on all 60 rows of `tests/golden/debug_collider.tsv` (Rust impetus 1.4.0 against Cyrius impetus 2.0.0), replayed by `tests/parity_features.tcyr` with `color_optics` |

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
