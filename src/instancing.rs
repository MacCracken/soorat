//! Per-instance rendering (transforms + color).
//!
//! Re-exported from [`mabda`] — the shared GPU foundation.

pub use mabda::instancing::{InstanceBuffer, InstanceData};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_data_size() {
        // mat4 + vec4 = 64 + 16 = 80
        assert_eq!(std::mem::size_of::<InstanceData>(), 80);
    }

    #[test]
    fn instance_data_default() {
        let d = InstanceData::default();
        assert_eq!(d.model[0], 1.0); // identity diagonal
        assert_eq!(d.model[5], 1.0);
        assert_eq!(d.model[10], 1.0);
        assert_eq!(d.model[15], 1.0);
        assert_eq!(d.color, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn instance_data_translation() {
        let d = InstanceData::from_translation(5.0, 3.0, 1.0);
        assert_eq!(d.model[12], 5.0);
        assert_eq!(d.model[13], 3.0);
        assert_eq!(d.model[14], 1.0);
    }

    #[test]
    fn instance_data_layout() {
        let layout = InstanceData::layout();
        assert_eq!(layout.array_stride, 80);
        assert_eq!(layout.attributes.len(), 5); // 4 mat4 cols + color
        assert_eq!(layout.step_mode, wgpu::VertexStepMode::Instance);
    }

    #[test]
    fn instance_data_bytemuck() {
        let d = InstanceData::default();
        let bytes = bytemuck::bytes_of(&d);
        assert_eq!(bytes.len(), 80);
    }

    #[test]
    fn instance_data_batch_cast() {
        let instances = vec![
            InstanceData::from_translation(0.0, 0.0, 0.0),
            InstanceData::from_translation(1.0, 0.0, 0.0),
            InstanceData::from_translation(2.0, 0.0, 0.0),
        ];
        let bytes: &[u8] = bytemuck::cast_slice(&instances);
        assert_eq!(bytes.len(), 80 * 3);
    }
}
