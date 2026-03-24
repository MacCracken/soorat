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
pub fn load_gltf_meshes(bytes: &[u8]) -> Result<(Vec<MeshData>, Vec<Vec<u8>>)> {
    let gltf = gltf::Gltf::from_slice(bytes).map_err(|e| RenderError::Texture(e.to_string()))?;

    let buffers = load_buffers(&gltf, bytes)?;
    let mut meshes = Vec::new();
    let mut images = Vec::new();

    // Load images
    for image in gltf.images() {
        match image.source() {
            gltf::image::Source::View { view, .. } => {
                let buf = &buffers[view.buffer().index()];
                let start = view.offset();
                let end = start + view.length();
                images.push(buf[start..end].to_vec());
            }
            gltf::image::Source::Uri { .. } => {
                images.push(Vec::new());
            }
        }
    }

    // Load meshes
    for mesh in gltf.meshes() {
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buf| Some(&buffers[buf.index()]));

            let positions: Vec<[f32; 3]> = reader
                .read_positions()
                .map(|iter| iter.collect())
                .unwrap_or_default();

            if positions.is_empty() {
                continue;
            }

            let normals: Vec<[f32; 3]> = reader
                .read_normals()
                .map(|iter| iter.collect())
                .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);

            let tex_coords: Vec<[f32; 2]> = reader
                .read_tex_coords(0)
                .map(|iter| iter.into_f32().collect())
                .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);

            let indices: Vec<u32> = reader
                .read_indices()
                .map(|iter| iter.into_u32().collect())
                .unwrap_or_else(|| (0..positions.len() as u32).collect());

            let vertices: Vec<Vertex3D> = positions
                .iter()
                .enumerate()
                .map(|(i, pos)| Vertex3D {
                    position: *pos,
                    normal: normals[i],
                    tex_coords: tex_coords[i],
                    color: [1.0, 1.0, 1.0, 1.0],
                })
                .collect();

            let material_index = primitive.material().index();

            meshes.push(MeshData {
                vertices,
                indices,
                material_index,
            });
        }
    }

    Ok((meshes, images))
}

/// Load a glTF model and upload to GPU.
pub fn load_model(device: &wgpu::Device, queue: &wgpu::Queue, bytes: &[u8]) -> Result<Model> {
    let (mesh_datas, image_datas) = load_gltf_meshes(bytes)?;

    let mut meshes = Vec::with_capacity(mesh_datas.len());
    let mut material_indices = Vec::with_capacity(mesh_datas.len());

    for data in &mesh_datas {
        meshes.push(Mesh::new(device, &data.vertices, &data.indices));
        material_indices.push(data.material_index);
    }

    let mut textures = Vec::with_capacity(image_datas.len());
    for (i, img_bytes) in image_datas.iter().enumerate() {
        if img_bytes.is_empty() {
            textures.push(Texture::white_pixel(device, queue));
        } else {
            let label = format!("gltf_texture_{i}");
            textures.push(Texture::from_bytes(device, queue, img_bytes, &label)?);
        }
    }

    Ok(Model {
        meshes,
        textures,
        material_indices,
    })
}

fn load_buffers(gltf: &gltf::Gltf, glb_bytes: &[u8]) -> Result<Vec<Vec<u8>>> {
    let mut buffers = Vec::new();

    for buffer in gltf.buffers() {
        match buffer.source() {
            gltf::buffer::Source::Bin => {
                let blob = gltf
                    .blob
                    .as_ref()
                    .ok_or_else(|| RenderError::Texture("Missing GLB bin chunk".into()))?;
                buffers.push(blob.clone());
            }
            gltf::buffer::Source::Uri(_) => {
                // For embedded base64 or external files — not supported in from-bytes path
                buffers.push(Vec::new());
            }
        }
    }

    // If no buffers found but we have GLB data, use the entire blob
    if buffers.is_empty() && !glb_bytes.is_empty() {
        buffers.push(glb_bytes.to_vec());
    }

    Ok(buffers)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
