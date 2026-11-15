//! glTF model loading.

use crate::error::{RenderError, Result};
use crate::mesh_pipeline::Mesh;
use crate::texture::Texture;
use crate::vertex::Vertex3D;

/// A loaded glTF model: meshes + textures.
pub struct Model {
    pub meshes: Vec<Mesh>,
    pub textures: Vec<Texture>,
    pub material_indices: Vec<Option<usize>>,
}

/// Loaded mesh data before GPU upload.
pub struct MeshData {
    pub vertices: Vec<Vertex3D>,
    pub indices: Vec<u32>,
    pub material_index: Option<usize>,
}

/// Load mesh data from glTF bytes (without GPU upload).
///
/// Only embedded data is read (GLB binary chunk / buffer-view images);
/// external URIs yield empty buffers and empty image slots. Input is treated
/// as untrusted:
/// - primitives that are not triangle lists, have no positions, or reference a
///   vertex index outside their position array are skipped (with a warning),
/// - image buffer-view ranges are bounds-checked (an invalid range yields an
///   empty image slot, rendered as a white fallback by [`load_model`]).
pub fn load_gltf_meshes(bytes: &[u8]) -> Result<(Vec<MeshData>, Vec<Vec<u8>>)> {
    tracing::debug!(byte_len = bytes.len(), "loading gltf meshes");
    let gltf = gltf::Gltf::from_slice(bytes).map_err(|e| RenderError::Model(e.to_string()))?;

    let blob = gltf.blob.as_deref().unwrap_or(&[]);
    let buffer_sources = collect_buffer_sources(&gltf, blob);

    // Load images (embedded buffer views only).
    let images: Vec<Vec<u8>> = gltf
        .images()
        .map(|image| match image.source() {
            gltf::image::Source::View { view, .. } => buffer_sources
                .get(view.buffer().index())
                .and_then(|buf| {
                    let start = view.offset();
                    buf.get(start..start.checked_add(view.length())?)
                })
                .map(<[u8]>::to_vec)
                .unwrap_or_default(),
            gltf::image::Source::Uri { .. } => Vec::new(),
        })
        .collect();

    // Load meshes
    let mut meshes = Vec::new();
    for mesh in gltf.meshes() {
        for primitive in mesh.primitives() {
            if primitive.mode() != gltf::mesh::Mode::Triangles {
                tracing::warn!(
                    mesh = mesh.index(),
                    mode = ?primitive.mode(),
                    "skipping non-triangle-list glTF primitive"
                );
                continue;
            }
            let reader = primitive.reader(|buf| buffer_sources.get(buf.index()).copied());

            let positions: Vec<[f32; 3]> = reader
                .read_positions()
                .map(|iter| iter.collect())
                .unwrap_or_default();
            if positions.is_empty() {
                continue;
            }
            let Ok(vertex_count) = u32::try_from(positions.len()) else {
                tracing::warn!(
                    vertices = positions.len(),
                    "skipping glTF primitive: too many vertices"
                );
                continue;
            };

            let indices: Vec<u32> = reader
                .read_indices()
                .map(|iter| iter.into_u32().collect())
                .unwrap_or_else(|| (0..vertex_count).collect());
            if let Some(&bad) = indices.iter().find(|&&i| i >= vertex_count) {
                tracing::warn!(
                    mesh = mesh.index(),
                    index = bad,
                    vertex_count,
                    "skipping glTF primitive: index out of range"
                );
                continue;
            }

            let mut normals = reader.read_normals();
            let mut tex_coords = reader.read_tex_coords(0).map(|tc| tc.into_f32());
            let vertices: Vec<Vertex3D> = positions
                .iter()
                .map(|&position| Vertex3D {
                    position,
                    normal: normals
                        .as_mut()
                        .and_then(Iterator::next)
                        .unwrap_or([0.0, 1.0, 0.0]),
                    tex_coords: tex_coords
                        .as_mut()
                        .and_then(Iterator::next)
                        .unwrap_or([0.0, 0.0]),
                    color: [1.0, 1.0, 1.0, 1.0],
                })
                .collect();

            meshes.push(MeshData {
                vertices,
                indices,
                material_index: primitive.material().index(),
            });
        }
    }

    Ok((meshes, images))
}

/// Load a glTF model and upload to GPU.
pub fn load_model(device: &wgpu::Device, queue: &wgpu::Queue, bytes: &[u8]) -> Result<Model> {
    tracing::debug!(byte_len = bytes.len(), "loading gltf model");
    let (mesh_datas, image_datas) = load_gltf_meshes(bytes)?;

    let mut meshes = Vec::with_capacity(mesh_datas.len());
    let mut material_indices = Vec::with_capacity(mesh_datas.len());

    for data in &mesh_datas {
        meshes.push(Mesh::new(device, &data.vertices, &data.indices));
        material_indices.push(data.material_index);
    }

    let mut textures = Vec::with_capacity(image_datas.len());
    let mut label_buf = String::new();
    for (i, img_bytes) in image_datas.iter().enumerate() {
        if img_bytes.is_empty() {
            textures.push(Texture::white_pixel(device, queue)?);
        } else {
            use std::fmt::Write;
            label_buf.clear();
            let _ = write!(label_buf, "gltf_texture_{i}");
            textures.push(Texture::from_bytes(device, queue, img_bytes, &label_buf)?);
        }
    }

    Ok(Model {
        meshes,
        textures,
        material_indices,
    })
}

/// Collect borrowed buffer slices from the glTF. Zero-copy for GLB blobs.
pub(crate) fn collect_buffer_sources<'a>(gltf: &'a gltf::Gltf, blob: &'a [u8]) -> Vec<&'a [u8]> {
    gltf.buffers()
        .map(|buffer| match buffer.source() {
            gltf::buffer::Source::Bin => blob,
            gltf::buffer::Source::Uri(_) => &[] as &[u8],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_meshes_skips_malformed_primitives() {
        let bytes = crate::test_support::mesh_edge_cases_glb();
        let (meshes, images) = load_gltf_meshes(&bytes).expect("fixture should parse");
        // Only the valid triangle-list primitive survives.
        assert_eq!(meshes.len(), 1);
        assert_eq!(meshes[0].vertices.len(), 3);
        assert_eq!(meshes[0].indices, vec![0, 2, 1]);
        // Missing normals/UVs fall back to defaults.
        assert_eq!(meshes[0].vertices[1].normal, [0.0, 1.0, 0.0]);
        assert_eq!(meshes[0].vertices[1].position, [1.0, 0.0, 0.0]);
        // Out-of-bounds image view → empty slot, not a panic.
        assert_eq!(images.len(), 1);
        assert!(images[0].is_empty());
    }

    #[test]
    fn load_meshes_truncated_input_is_error() {
        let bytes = crate::test_support::mesh_edge_cases_glb();
        for cut in [0, 4, 12, 20, bytes.len() / 2] {
            assert!(load_gltf_meshes(&bytes[..cut]).is_err(), "cut at {cut}");
        }
    }

    #[test]
    fn mesh_data_types() {
        let data = MeshData {
            vertices: vec![Vertex3D {
                position: [0.0, 0.0, 0.0],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            }],
            indices: vec![0],
            material_index: None,
        };
        assert_eq!(data.vertices.len(), 1);
        assert_eq!(data.indices.len(), 1);
    }

    #[test]
    fn load_invalid_gltf_returns_error() {
        let result = load_gltf_meshes(b"not a gltf file");
        assert!(result.is_err());
    }

    #[test]
    fn load_empty_gltf_returns_model_error() {
        let result = load_gltf_meshes(b"");
        assert!(result.is_err());
    }
}
