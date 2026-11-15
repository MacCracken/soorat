//! Offline WGSL validation with naga — runs without a GPU, so CI catches
//! shader errors (including in shaders no Rust pipeline builds yet, such as
//! `pbr_skinned.wgsl`).

use naga::valid::{Capabilities, ValidationFlags, Validator};

const SHADERS: &[(&str, &str, &[&str])] = &[
    (
        "sprite.wgsl",
        include_str!("../src/sprite.wgsl"),
        &["vs_main", "fs_main"],
    ),
    (
        "line.wgsl",
        include_str!("../src/line.wgsl"),
        &["vs_main", "fs_main"],
    ),
    (
        "pbr.wgsl",
        include_str!("../src/pbr.wgsl"),
        &["vs_main", "fs_main"],
    ),
    (
        "pbr_skinned.wgsl",
        include_str!("../src/pbr_skinned.wgsl"),
        &["vs_main", "fs_main"],
    ),
    (
        "shadow.wgsl",
        include_str!("../src/shadow.wgsl"),
        &["vs_main"],
    ),
    (
        "ssao.wgsl",
        include_str!("../src/ssao.wgsl"),
        &["vs_main", "fs_main"],
    ),
    (
        "bloom.wgsl",
        include_str!("../src/bloom.wgsl"),
        &["vs_main", "fs_threshold", "fs_blur_h", "fs_blur_v"],
    ),
    (
        "postprocess.wgsl",
        include_str!("../src/postprocess.wgsl"),
        &["vs_main", "fs_main", "fs_no_bloom"],
    ),
    (
        "gpu_particles.wgsl",
        include_str!("../src/gpu_particles.wgsl"),
        &["cs_update"],
    ),
];

#[test]
fn all_shaders_parse_and_validate() {
    for (name, source, entry_points) in SHADERS {
        let module = naga::front::wgsl::parse_str(source)
            .unwrap_or_else(|e| panic!("{name} failed to parse:\n{}", e.emit_to_string(source)));
        Validator::new(ValidationFlags::all(), Capabilities::empty())
            .validate(&module)
            .unwrap_or_else(|e| panic!("{name} failed validation:\n{}", e.emit_to_string(source)));

        for entry in *entry_points {
            assert!(
                module.entry_points.iter().any(|ep| ep.name == *entry),
                "{name} is missing entry point `{entry}`"
            );
        }
    }
}

#[test]
fn shadow_sampling_avoids_implicit_derivatives() {
    // Shadow lookups run after non-uniform early returns; only the explicit-level
    // comparison sample is valid there under WGSL's uniformity rules (WebGPU).
    for (name, source, _) in SHADERS {
        assert!(
            !source.contains("textureSampleCompare("),
            "{name} uses textureSampleCompare (implicit derivatives) — use textureSampleCompareLevel"
        );
    }
}
