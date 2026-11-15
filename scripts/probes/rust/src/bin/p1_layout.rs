//! P1 — GPU layouts (soorat-port-plan §4 step 1).
//!
//! 1. Prints the size and field offsets of every `#[repr(C)]` GPU type of soorat
//!    1.2.0 and of mabda's vertex/instance types (`offset_of!`).
//! 2. Parses the nine WGSL shaders with naga 29 and compares each Rust uniform /
//!    storage type with the WGSL struct it is bound as (size vs span, member
//!    offsets in order), and each vertex type with its `layout()` attributes.
//! 3. Generates `../math/tests/p1_layout.tcyr`: the same types as Cyrius structs
//!    (nested `f32` structs, arrays expanded into indexed fields) with the Rust
//!    sizes and offsets as expectations. The table is generated, never retyped.
//!
//! Run: `cargo run --release --bin p1_layout` (from scripts/probes/rust).

use std::fmt::Write as _;
use std::mem::{offset_of, size_of};

use soorat::animation::JointUniforms;
use soorat::debug_draw::LineVertex;
use soorat::gpu_particles::{GpuParticle, SimParams};
use soorat::hdr::BloomUniforms;
use soorat::instancing::InstanceData;
use soorat::lights::{GpuLight, LightArrayUniforms};
use soorat::mesh_pipeline::{CameraUniforms, LightUniforms, ShadowPassUniforms};
use soorat::pbr_material::MaterialUniforms;
use soorat::postprocess::PostProcessUniforms;
use soorat::shadow::{CascadeUniforms, ShadowUniforms};
use soorat::ssao::SsaoUniforms;
use soorat::vertex::{SkinnedVertex3D, Vertex2D, Vertex3D};

/// Field kinds, as the Cyrius mirror spells them.
#[derive(Clone, Copy)]
enum K {
    F32,
    U32,
    V2,
    V3,
    V4,
    U4,
    M4,
    Arr(&'static K, usize),
    Light,
}

const M4: K = K::M4;
const LIGHT: K = K::Light;

impl K {
    fn size(self) -> usize {
        match self {
            K::F32 | K::U32 => 4,
            K::V2 => 8,
            K::V3 => 12,
            K::V4 | K::U4 => 16,
            K::M4 => 64,
            K::Light => 64,
            K::Arr(k, n) => k.size() * n,
        }
    }
    fn cyr(self) -> &'static str {
        match self {
            K::F32 => "f32",
            K::U32 => "u32",
            K::V2 => "SrV2",
            K::V3 => "SrV3",
            K::V4 => "SrV4",
            K::U4 => "SrU4",
            K::M4 => "SrM4",
            K::Light => "SrGpuLight",
            K::Arr(..) => unreachable!(),
        }
    }
    /// Path from the field to its first scalar leaf, and whether it is u32.
    fn leaf(self) -> (&'static str, bool) {
        match self {
            K::F32 => ("", false),
            K::U32 => ("", true),
            K::V2 | K::V3 | K::V4 => (".x", false),
            K::U4 => (".x", true),
            K::M4 => (".c0.x", false),
            K::Light => (".position_type.x", false),
            K::Arr(..) => unreachable!(),
        }
    }
}

struct Ty {
    rust: &'static str,
    size: usize,
    fields: Vec<(&'static str, usize, K)>,
    /// (shader file, WGSL struct) this type is bound as, if any.
    wgsl: Option<(&'static str, &'static str)>,
}

macro_rules! ty {
    ($t:ty, $wgsl:expr, [$(($f:ident, $k:expr)),* $(,)?]) => {
        Ty {
            rust: stringify!($t),
            size: size_of::<$t>(),
            fields: vec![$((stringify!($f), offset_of!($t, $f), $k)),*],
            wgsl: $wgsl,
        }
    };
}

fn types() -> Vec<Ty> {
    vec![
        ty!(GpuParticle, Some(("gpu_particles.wgsl", "Particle")), [(position, K::V4), (velocity, K::V4), (color, K::V4)]),
        ty!(SimParams, Some(("gpu_particles.wgsl", "SimParams")), [(delta_time, K::F32), (gravity_y, K::F32), (damping, K::F32), (particle_count, K::U32)]),
        ty!(PostProcessUniforms, Some(("postprocess.wgsl", "PostProcessUniforms")), [(params, K::V4)]),
        ty!(JointUniforms, Some(("pbr_skinned.wgsl", "JointUniforms")), [(joint_count, K::V4), (joints, K::Arr(&M4, 128))]),
        ty!(BloomUniforms, Some(("bloom.wgsl", "BloomUniforms")), [(params, K::V4), (texel_size, K::V4)]),
        ty!(LineVertex, None, [(position, K::V3), (color, K::V4)]),
        ty!(GpuLight, Some(("pbr.wgsl", "GpuLight")), [(position_type, K::V4), (direction_range, K::V4), (color_intensity, K::V4), (spot_params, K::V4)]),
        ty!(LightArrayUniforms, Some(("pbr.wgsl", "LightArrayUniforms")), [(ambient, K::V4), (light_count, K::V4), (lights, K::Arr(&LIGHT, 8))]),
        ty!(MaterialUniforms, Some(("pbr.wgsl", "MaterialUniforms")), [(base_color_factor, K::V4), (metallic, K::F32), (roughness, K::F32), (_pad0, K::F32), (_pad1, K::F32)]),
        ty!(SsaoUniforms, Some(("ssao.wgsl", "SsaoUniforms")), [(params, K::V4), (projection, K::M4), (inv_projection, K::M4)]),
        ty!(CameraUniforms, Some(("pbr.wgsl", "CameraUniforms")), [(view_proj, K::M4), (model, K::M4), (camera_pos, K::V4), (normal_matrix_0, K::V4), (normal_matrix_1, K::V4), (normal_matrix_2, K::V4)]),
        ty!(LightUniforms, None, [(ambient_color, K::V4), (light_direction, K::V4), (light_color, K::V4), (light_view_proj, K::M4)]),
        ty!(ShadowPassUniforms, Some(("pbr.wgsl", "ShadowUniforms")), [(light_view_proj, K::M4), (shadow_map_size, K::V4)]),
        ty!(ShadowUniforms, Some(("shadow.wgsl", "ShadowUniforms")), [(light_view_proj, K::M4), (model, K::M4)]),
        ty!(CascadeUniforms, None, [(splits, K::V4), (matrices, K::Arr(&M4, 4))]),
        ty!(Vertex2D, None, [(position, K::V2), (tex_coords, K::V2), (color, K::V4)]),
        ty!(Vertex3D, None, [(position, K::V3), (normal, K::V3), (tex_coords, K::V2), (color, K::V4)]),
        ty!(SkinnedVertex3D, None, [(position, K::V3), (normal, K::V3), (tex_coords, K::V2), (color, K::V4), (tangent, K::V4), (joints, K::U4), (weights, K::V4)]),
        ty!(InstanceData, None, [(model, K::M4), (color, K::V4)]),
    ]
}

/// WGSL struct layouts from naga: name → (span, [(member, offset, size)]).
fn wgsl_struct(shader: &str, name: &str) -> Option<(u32, Vec<(String, u32, u32)>)> {
    let path = format!("{}/../../../src/{shader}", env!("CARGO_MANIFEST_DIR"));
    let src = std::fs::read_to_string(&path).ok()?;
    let module = naga::front::wgsl::parse_str(&src).ok()?;
    let mut layouter = naga::proc::Layouter::default();
    layouter.update(module.to_ctx()).ok()?;
    for (_, ty) in module.types.iter() {
        if ty.name.as_deref() != Some(name) {
            continue;
        }
        if let naga::TypeInner::Struct { members, span } = &ty.inner {
            let m = members
                .iter()
                .map(|m| {
                    (
                        m.name.clone().unwrap_or_default(),
                        m.offset,
                        layouter[m.ty].size,
                    )
                })
                .collect();
            return Some((*span, m));
        }
    }
    None
}

/// A plain copy of a wgpu vertex layout (stride, attribute offsets/formats).
mod wgpu_layout {
    pub struct L {
        pub stride: u64,
        pub attrs: Vec<(u64, String)>,
    }
}

/// Copy a `VertexBufferLayout` without naming wgpu's type (soorat does not re-export wgpu).
macro_rules! vl {
    ($e:expr) => {{
        let l = $e;
        wgpu_layout::L {
            stride: l.array_stride,
            attrs: l.attributes.iter().map(|a| (a.offset, format!("{:?}", a.format))).collect(),
        }
    }};
}

fn main() {
    let tys = types();
    let mut mismatches = 0;

    println!("# P1 — Rust layouts vs WGSL (naga 29)\n");
    for t in &tys {
        let fields: Vec<String> = t.fields.iter().map(|(f, o, _)| format!("{f}@{o}")).collect();
        println!("{:<20} size {:>5}  {}", t.rust, t.size, fields.join(" "));
        // Packed sanity: the Cyrius mirror is packed, so Rust must have no padding.
        let packed: usize = t.fields.iter().map(|(_, _, k)| k.size()).sum();
        if packed != t.size {
            println!("  !! Rust size {} != sum of field sizes {packed} (implicit padding)", t.size);
            mismatches += 1;
        }
        if let Some((shader, name)) = t.wgsl {
            match wgsl_struct(shader, name) {
                None => {
                    println!("  !! WGSL struct {name} not found in {shader}");
                    mismatches += 1;
                }
                Some((span, members)) => {
                    let wo: Vec<u32> = members.iter().map(|m| m.1).collect();
                    let ro: Vec<u32> = t.fields.iter().map(|f| f.1 as u32).collect();
                    let ok = span as usize == t.size && wo == ro;
                    println!(
                        "  {} {shader}::{name} span {span} offsets {:?}{}",
                        if ok { "==" } else { "!!" },
                        wo,
                        if ok { "" } else { "  <-- MISMATCH" }
                    );
                    if !ok {
                        mismatches += 1;
                    }
                }
            }
        }
    }

    // Vertex types against the layout() wgpu actually uses.
    println!("\n# Vertex buffer layouts (mabda layout() vs offset_of!)\n");
    let check = |name: &str, layout: wgpu_layout::L, offsets: Vec<usize>, size: usize| -> bool {
        let lo: Vec<usize> = layout.attrs.iter().map(|a| a.0 as usize).collect();
        let ok = lo == offsets && layout.stride as usize == size;
        println!(
            "{} {name}: stride {} attrs {:?} vs fields {:?}",
            if ok { "==" } else { "!!" },
            layout.stride,
            layout.attrs,
            offsets
        );
        ok
    };
    for (name, l, t) in [
        ("Vertex2D", vl!(Vertex2D::layout()), "Vertex2D"),
        ("Vertex3D", vl!(Vertex3D::layout()), "Vertex3D"),
        ("SkinnedVertex3D", vl!(SkinnedVertex3D::layout()), "SkinnedVertex3D"),
        ("LineVertex", vl!(LineVertex::layout()), "LineVertex"),
    ] {
        let ty = tys.iter().find(|x| x.rust == t).expect("type");
        let offs = ty.fields.iter().map(|f| f.1).collect();
        if !check(name, l, offs, ty.size) {
            mismatches += 1;
        }
    }

    // Generate the Cyrius mirror and its expectations.
    let out = format!("{}/../math/tests/p1_layout.tcyr", env!("CARGO_MANIFEST_DIR"));
    std::fs::write(&out, cyrius_test(&tys)).expect("write p1_layout.tcyr");
    println!("\nwrote {out}");
    println!("mismatches: {mismatches}");
}

/// The Cyrius test: struct mirrors + per-type marker probes.
fn cyrius_test(tys: &[Ty]) -> String {
    let mut s = String::new();
    s.push_str(
        "# p1_layout.tcyr — GENERATED by scripts/probes/rust/src/bin/p1_layout.rs from soorat 1.2.0.\n\
         # Do not edit: re-run the generator. Each soorat GPU type mirrored as a packed Cyrius struct\n\
         # (nested f32 structs; arrays expanded into indexed fields, since array fields are refused),\n\
         # checked against Rust's size_of / offset_of by writing a marker into each field's first\n\
         # scalar and finding it in the bytes.\n\n",
    );
    s.push_str("struct SrV2 { x: f32; y: f32; }\n");
    s.push_str("struct SrV3 { x: f32; y: f32; z: f32; }\n");
    s.push_str("struct SrV4 { x: f32; y: f32; z: f32; w: f32; }\n");
    s.push_str("struct SrU4 { x: u32; y: u32; z: u32; w: u32; }\n");
    s.push_str("struct SrM4 { c0: SrV4; c1: SrV4; c2: SrV4; c3: SrV4; }\n");
    for t in tys {
        let _ = write!(s, "struct Sr{} {{", t.rust);
        for (f, _, k) in &t.fields {
            match k {
                K::Arr(inner, n) => {
                    for i in 0..*n {
                        let _ = write!(s, " {f}_{i}: {};", inner.cyr());
                    }
                }
                k => {
                    let _ = write!(s, " {f}: {};", k.cyr());
                }
            }
        }
        s.push_str(" }\n");
    }
    s.push_str(
        "\nfn find32(base, size, marker) {\n    var i = 0;\n    while (i + 4 <= size) {\n        \
         if (load32(base + i) == marker) { return i; }\n        i = i + 1;\n    }\n    return -1;\n}\n\
         fn mk(k) { return f32_from(f64_from(k)); }\n",
    );
    for t in tys {
        let _ = writeln!(s, "\nfn probe_{}() {{", t.rust);
        let _ = writeln!(s, "    var t: Sr{};\n    var base = &t;\n    memset(base, 0, sizeof(Sr{}));", t.rust, t.rust);
        let _ = writeln!(s, "    assert_eq(sizeof(Sr{}), {}, \"{} size\");", t.rust, t.size, t.rust);
        let mut checks = Vec::new();
        let mut k_id = 1usize;
        for (f, off, k) in &t.fields {
            let mut emit = |name: String, kind: K, offset: usize| {
                let (leaf, is_u32) = kind.leaf();
                let marker = if is_u32 { format!("{k_id}") } else { format!("mk({k_id})") };
                let _ = writeln!(s, "    t.{name}{leaf} = {marker};");
                checks.push(format!(
                    "    assert_eq(find32(base, sizeof(Sr{}), {marker}), {offset}, \"{}.{name}\");",
                    t.rust, t.rust
                ));
                k_id += 1;
            };
            match k {
                K::Arr(inner, n) => {
                    for i in 0..*n {
                        emit(format!("{f}_{i}"), **inner, off + i * inner.size());
                    }
                }
                k => emit((*f).to_string(), *k, *off),
            }
        }
        for c in checks {
            s.push_str(&c);
            s.push('\n');
        }
        s.push_str("    return 0;\n}\n");
    }
    s.push_str("\nalloc_init();\ntest_group(\"p1 layouts\");\n");
    for t in tys {
        let _ = writeln!(s, "probe_{}();", t.rust);
    }
    s.push_str("var r = assert_summary();\n");
    s
}

