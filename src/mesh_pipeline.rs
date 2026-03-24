//! 3D mesh rendering pipeline.

use crate::error::Result;
use crate::vertex::Vertex3D;
use wgpu::util::DeviceExt;

/// Camera uniforms for the mesh shader.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniforms {
    pub view_proj: [f32; 16],
    pub model: [f32; 16],
    pub camera_pos: [f32; 4],
    /// Inverse-transpose of model matrix rows for correct normals.
    /// Stored as 3 × vec4 (padded mat3) for WGSL uniform alignment.
    pub normal_matrix_0: [f32; 4],
    pub normal_matrix_1: [f32; 4],
    pub normal_matrix_2: [f32; 4],
}

impl Default for CameraUniforms {
    fn default() -> Self {
        Self {
            view_proj: IDENTITY_MAT4,
            model: IDENTITY_MAT4,
            camera_pos: [0.0, 0.0, 5.0, 0.0],
            normal_matrix_0: [1.0, 0.0, 0.0, 0.0],
            normal_matrix_1: [0.0, 1.0, 0.0, 0.0],
            normal_matrix_2: [0.0, 0.0, 1.0, 0.0],
        }
    }
}

impl CameraUniforms {
    /// Set the model matrix and auto-compute the normal matrix (inverse-transpose of upper 3x3).
    pub fn set_model(&mut self, model: [f32; 16]) {
        self.model = model;
        // Extract upper 3x3 rows, compute inverse-transpose
        // For uniform scale, transpose of upper 3x3 = inverse-transpose
        // For non-uniform scale, need proper inverse
        let (nm0, nm1, nm2) = inverse_transpose_3x3(&model);
        self.normal_matrix_0 = [nm0[0], nm0[1], nm0[2], 0.0];
        self.normal_matrix_1 = [nm1[0], nm1[1], nm1[2], 0.0];
        self.normal_matrix_2 = [nm2[0], nm2[1], nm2[2], 0.0];
    }
}

/// Compute inverse-transpose of upper-left 3x3 from a 4x4 column-major matrix.
/// Returns 3 rows of the resulting 3x3 matrix.
fn inverse_transpose_3x3(m: &[f32; 16]) -> ([f32; 3], [f32; 3], [f32; 3]) {
    // Extract upper-left 3x3 (column-major)
    let a = [m[0], m[1], m[2]];
    let b = [m[4], m[5], m[6]];
    let c = [m[8], m[9], m[10]];

    // For 3x3 columns [a, b, c]:
    // inv = (1/det) * [b×c, c×a, a×b]^T
    // inv^T = (1/det) * [b×c, c×a, a×b] (as rows)

    let bc = [
        b[1] * c[2] - b[2] * c[1],
        b[2] * c[0] - b[0] * c[2],
        b[0] * c[1] - b[1] * c[0],
    ];
    let ca = [
        c[1] * a[2] - c[2] * a[1],
        c[2] * a[0] - c[0] * a[2],
        c[0] * a[1] - c[1] * a[0],
    ];
    let ab = [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ];

    let det = a[0] * bc[0] + a[1] * bc[1] + a[2] * bc[2];
    let inv_det = if det.abs() > 1e-10 { 1.0 / det } else { 1.0 };

    (
        [bc[0] * inv_det, bc[1] * inv_det, bc[2] * inv_det],
        [ca[0] * inv_det, ca[1] * inv_det, ca[2] * inv_det],
        [ab[0] * inv_det, ab[1] * inv_det, ab[2] * inv_det],
    )
}

/// Light uniforms for the mesh shader.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniforms {
    /// RGB + intensity in alpha.
    pub ambient_color: [f32; 4],
    /// Normalized direction the light points (w unused).
    pub light_direction: [f32; 4],
    /// RGB + intensity in alpha.
    pub light_color: [f32; 4],
    /// Light view-projection matrix for shadow mapping.
    pub light_view_proj: [f32; 16],
}

impl Default for LightUniforms {
    fn default() -> Self {
        Self {
            ambient_color: [1.0, 1.0, 1.0, 0.1],
            // Normalized: (0, -1, -1) / sqrt(2)
            light_direction: [
                0.0,
                -std::f32::consts::FRAC_1_SQRT_2,
                -std::f32::consts::FRAC_1_SQRT_2,
                0.0,
            ],
            light_color: [1.0, 1.0, 1.0, 1.0],
            light_view_proj: IDENTITY_MAT4,
        }
    }
}

/// Identity 4x4 matrix (column-major).
const IDENTITY_MAT4: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

/// A loaded 3D mesh with GPU buffers ready for drawing.
pub struct Mesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
}

impl Mesh {
    /// Create a mesh from vertex and index data.
    pub fn new(device: &wgpu::Device, vertices: &[Vertex3D], indices: &[u32]) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh_vertex_buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh_index_buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
        }
    }
}

/// Depth buffer texture for z-testing.
pub struct DepthBuffer {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

impl DepthBuffer {
    pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth_buffer"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self { texture, view }
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        *self = Self::new(device, width, height);
    }
}

/// Shadow pass uniforms for the PBR shader.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShadowPassUniforms {
    pub light_view_proj: [f32; 16],
    pub shadow_map_size: [f32; 4],
}

impl Default for ShadowPassUniforms {
    fn default() -> Self {
        Self {
            light_view_proj: IDENTITY_MAT4,
            shadow_map_size: [2048.0, 0.0, 0.0, 0.0],
        }
    }
}

/// 3D mesh rendering pipeline with PBR shading (Cook-Torrance/GGX/Fresnel-Schlick).
pub struct MeshPipeline {
    render_pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    light_buffer: wgpu::Buffer,
    material_buffer: wgpu::Buffer,
    shadow_pass_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    material_bind_group_layout: wgpu::BindGroupLayout,
    shadow_bind_group_layout: wgpu::BindGroupLayout,
    ibl_bind_group_layout: wgpu::BindGroupLayout,
}

impl MeshPipeline {
    /// Create a new PBR mesh pipeline for the given surface format.
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Result<Self> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pbr_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("pbr.wgsl").into()),
        });

        // Group 0: camera + light_array + material + shadow uniforms
        let uniform_entry = |binding: u32, vis: wgpu::ShaderStages| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: vis,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("pbr_uniform_layout"),
                entries: &[
                    uniform_entry(0, wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT), // camera
                    uniform_entry(1, wgpu::ShaderStages::FRAGMENT), // light_array
                    uniform_entry(2, wgpu::ShaderStages::FRAGMENT), // material
                    uniform_entry(3, wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT), // shadow
                ],
            });

        // Group 1: textures (base color + sampler)
        let material_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("pbr_material_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        // Group 2: shadow map (depth texture + comparison sampler)
        let shadow_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("pbr_shadow_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                ],
            });

        // Group 3: IBL (irradiance cubemap + prefiltered cubemap + BRDF LUT)
        let ibl_bind_group_layout = crate::environment::IblBindGroup::layout(device);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pbr_pipeline_layout"),
            bind_group_layouts: &[
                &uniform_bind_group_layout,
                &material_bind_group_layout,
                &shadow_bind_group_layout,
                &ibl_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mesh_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex3D::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DepthBuffer::FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera_uniform_buffer"),
            size: std::mem::size_of::<CameraUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let light_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("light_array_uniform_buffer"),
            size: std::mem::size_of::<crate::lights::LightArrayUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let material_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("material_uniform_buffer"),
            size: std::mem::size_of::<crate::pbr_material::MaterialUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let shadow_pass_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadow_pass_uniform_buffer"),
            size: std::mem::size_of::<ShadowPassUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pbr_uniform_bind_group"),
            layout: &uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: light_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: material_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: shadow_pass_buffer.as_entire_binding(),
                },
            ],
        });

        Ok(Self {
            render_pipeline,
            camera_buffer,
            light_buffer,
            material_buffer,
            shadow_pass_buffer,
            uniform_bind_group,
            material_bind_group_layout,
            shadow_bind_group_layout,
            ibl_bind_group_layout,
        })
    }

    /// Get the material bind group layout for creating material bind groups.
    pub fn material_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.material_bind_group_layout
    }

    /// Update camera uniforms (view-projection + model matrix).
    pub fn update_camera(&self, queue: &wgpu::Queue, camera: &CameraUniforms) {
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(camera));
    }

    /// Update light uniforms (multi-light array).
    pub fn update_lights(&self, queue: &wgpu::Queue, lights: &crate::lights::LightArrayUniforms) {
        queue.write_buffer(&self.light_buffer, 0, bytemuck::bytes_of(lights));
    }

    /// Update shadow pass uniforms (light view-proj + shadow map size).
    pub fn update_shadow_pass(&self, queue: &wgpu::Queue, shadow: &ShadowPassUniforms) {
        queue.write_buffer(&self.shadow_pass_buffer, 0, bytemuck::bytes_of(shadow));
    }

    /// Update PBR material uniforms (metallic, roughness, base_color_factor).
    pub fn update_material(
        &self,
        queue: &wgpu::Queue,
        material: &crate::pbr_material::MaterialUniforms,
    ) {
        queue.write_buffer(&self.material_buffer, 0, bytemuck::bytes_of(material));
    }

    /// Get the shadow bind group layout for creating shadow bind groups.
    pub fn shadow_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.shadow_bind_group_layout
    }

    /// Get the IBL bind group layout for creating IBL bind groups.
    pub fn ibl_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.ibl_bind_group_layout
    }

    /// Create a shadow bind group from a shadow map.
    pub fn create_shadow_bind_group(
        &self,
        device: &wgpu::Device,
        shadow_map: &crate::shadow::ShadowMap,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow_bind_group"),
            layout: &self.shadow_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&shadow_map.depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&shadow_map.sampler),
                },
            ],
        })
    }

    /// Draw a mesh with PBR shading, shadow mapping, and IBL.
    /// `ibl_bind_group`: IBL environment maps. Use `EnvironmentMap::solid_color` for a black
    /// cubemap when IBL is not desired.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        color_view: &wgpu::TextureView,
        depth: &DepthBuffer,
        mesh: &Mesh,
        material_bind_group: &wgpu::BindGroup,
        shadow_bind_group: &wgpu::BindGroup,
        ibl_bind_group: &wgpu::BindGroup,
        clear_color: Option<crate::color::Color>,
    ) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("mesh_encoder"),
        });

        {
            let color_load = match clear_color {
                Some(c) => wgpu::LoadOp::Clear(c.to_wgpu()),
                None => wgpu::LoadOp::Load,
            };

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("mesh_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: color_load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_bind_group(1, material_bind_group, &[]);
            render_pass.set_bind_group(2, shadow_bind_group, &[]);
            render_pass.set_bind_group(3, ibl_bind_group, &[]);
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
        }

        queue.submit(std::iter::once(encoder.finish()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_uniforms_size() {
        // 2 * mat4 + vec4 + 3 * vec4 (normal matrix) = 128 + 16 + 48 = 192
        assert_eq!(std::mem::size_of::<CameraUniforms>(), 192);
    }

    #[test]
    fn light_uniforms_size() {
        assert_eq!(std::mem::size_of::<LightUniforms>(), 112); // 3 * vec4 + mat4 = 48 + 64
    }

    #[test]
    fn shadow_pass_uniforms_size() {
        assert_eq!(std::mem::size_of::<ShadowPassUniforms>(), 80); // mat4 + vec4 = 64 + 16
    }

    #[test]
    fn camera_uniforms_default() {
        let cam = CameraUniforms::default();
        // Identity matrix: diagonal = 1
        assert_eq!(cam.view_proj[0], 1.0);
        assert_eq!(cam.view_proj[5], 1.0);
        assert_eq!(cam.view_proj[10], 1.0);
        assert_eq!(cam.view_proj[15], 1.0);
    }

    #[test]
    fn light_uniforms_default() {
        let light = LightUniforms::default();
        assert_eq!(light.ambient_color[3], 0.1); // low ambient
        assert!(light.light_direction[1] < 0.0); // pointing down
        // Direction should be normalized (length ~1.0)
        let d = light.light_direction;
        let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        assert!((len - 1.0).abs() < 0.001);
    }

    #[test]
    fn camera_uniforms_bytemuck() {
        let cam = CameraUniforms::default();
        let bytes = bytemuck::bytes_of(&cam);
        assert_eq!(bytes.len(), 192);
    }

    #[test]
    fn camera_set_model_computes_normals() {
        let mut cam = CameraUniforms::default();
        cam.set_model(IDENTITY_MAT4);
        // Identity model → identity normal matrix
        assert!((cam.normal_matrix_0[0] - 1.0).abs() < 0.001);
        assert!((cam.normal_matrix_1[1] - 1.0).abs() < 0.001);
        assert!((cam.normal_matrix_2[2] - 1.0).abs() < 0.001);
    }

    #[test]
    fn camera_set_model_non_uniform_scale() {
        let mut cam = CameraUniforms::default();
        // Non-uniform scale: 2x in X, 1x in Y, 0.5x in Z
        let mut model = IDENTITY_MAT4;
        model[0] = 2.0; // scale X
        model[10] = 0.5; // scale Z
        cam.set_model(model);
        // Normal matrix should compensate: X normals shrink, Z normals stretch
        assert!((cam.normal_matrix_0[0] - 0.5).abs() < 0.001); // 1/2
        assert!((cam.normal_matrix_1[1] - 1.0).abs() < 0.001); // 1/1
        assert!((cam.normal_matrix_2[2] - 2.0).abs() < 0.001); // 1/0.5
    }

    #[test]
    fn shadow_pass_uniforms_default() {
        let s = ShadowPassUniforms::default();
        assert_eq!(s.shadow_map_size[0], 2048.0);
        assert_eq!(s.light_view_proj[0], 1.0); // identity
    }

    #[test]
    fn shadow_pass_uniforms_bytemuck() {
        let s = ShadowPassUniforms::default();
        let bytes = bytemuck::bytes_of(&s);
        assert_eq!(bytes.len(), 80);
    }

    #[test]
    fn light_uniforms_bytemuck() {
        let light = LightUniforms::default();
        let bytes = bytemuck::bytes_of(&light);
        assert_eq!(bytes.len(), 112);
    }

    #[test]
    fn depth_buffer_format() {
        assert_eq!(DepthBuffer::FORMAT, wgpu::TextureFormat::Depth32Float);
    }
}
