//! `layout`: the size and field offsets of every GPU-facing `#[repr(C)]` type (class A).
//! Probe P1 (port-parity §1.1) showed packed nested f32 structs reproduce them; M1's
//! `src/layout.cyr` is generated from these rows and checked against them.

use crate::{Module, Toks};
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

macro_rules! layout {
    ($m:expr, $t:ty, [$($f:ident),* $(,)?]) => {{
        let names = [$(stringify!($f)),*];
        let offs: Vec<i128> = vec![$(offset_of!($t, $f) as i128),*];
        let outs = Toks::new().i(size_of::<$t>() as i128).is(&offs);
        $m.row(
            concat!("layout::", stringify!($t)),
            'A',
            &Toks::new(),
            &outs,
            &format!("size {}; {}", size_of::<$t>(), names.join(" ")),
        );
    }};
}

pub fn module() -> Module {
    let mut m = Module::new("layout");
    layout!(m, GpuParticle, [position, velocity, color]);
    layout!(
        m,
        SimParams,
        [delta_time, gravity_y, damping, particle_count]
    );
    layout!(m, PostProcessUniforms, [params]);
    layout!(m, JointUniforms, [joint_count, joints]);
    layout!(m, BloomUniforms, [params, texel_size]);
    layout!(m, LineVertex, [position, color]);
    layout!(
        m,
        GpuLight,
        [position_type, direction_range, color_intensity, spot_params]
    );
    layout!(m, LightArrayUniforms, [ambient, light_count, lights]);
    layout!(
        m,
        MaterialUniforms,
        [base_color_factor, metallic, roughness, _pad0, _pad1]
    );
    layout!(m, SsaoUniforms, [params, projection, inv_projection]);
    layout!(
        m,
        CameraUniforms,
        [
            view_proj,
            model,
            camera_pos,
            normal_matrix_0,
            normal_matrix_1,
            normal_matrix_2
        ]
    );
    layout!(
        m,
        LightUniforms,
        [ambient_color, light_direction, light_color, light_view_proj]
    );
    layout!(m, ShadowPassUniforms, [light_view_proj, shadow_map_size]);
    layout!(m, ShadowUniforms, [light_view_proj, model]);
    layout!(m, CascadeUniforms, [splits, matrices]);
    layout!(m, Vertex2D, [position, tex_coords, color]);
    layout!(m, Vertex3D, [position, normal, tex_coords, color]);
    layout!(
        m,
        SkinnedVertex3D,
        [
            position, normal, tex_coords, color, tangent, joints, weights
        ]
    );
    layout!(m, InstanceData, [model, color]);
    m
}
