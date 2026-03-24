//! Multi-light system — directional, point, and spot lights.

/// Maximum number of lights supported in a single draw call.
pub const MAX_LIGHTS: usize = 8;

/// Light type discriminator.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightType {
    Directional = 0,
    Point = 1,
    Spot = 2,
}

/// A single light packed for GPU upload.
/// 64 bytes per light (4 × vec4), aligned to 16 bytes.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuLight {
    /// xyz = position (point/spot) or direction (directional), w = light type (0/1/2)
    pub position_type: [f32; 4],
    /// xyz = direction (spot), w = range (point/spot, 0 = infinite)
    pub direction_range: [f32; 4],
    /// RGB + intensity in alpha
    pub color_intensity: [f32; 4],
    /// x = inner cone cos (spot), y = outer cone cos (spot), z/w unused
    pub spot_params: [f32; 4],
}

impl GpuLight {
    /// Create a directional light.
    pub fn directional(direction: [f32; 3], color: [f32; 3], intensity: f32) -> Self {
        Self {
            position_type: [
                direction[0],
                direction[1],
                direction[2],
                LightType::Directional as u32 as f32,
            ],
            direction_range: [0.0; 4],
            color_intensity: [color[0], color[1], color[2], intensity],
            spot_params: [0.0; 4],
        }
    }

    /// Create a point light.
    pub fn point(position: [f32; 3], color: [f32; 3], intensity: f32, range: f32) -> Self {
        Self {
            position_type: [
                position[0],
                position[1],
                position[2],
                LightType::Point as u32 as f32,
            ],
            direction_range: [0.0, 0.0, 0.0, range],
            color_intensity: [color[0], color[1], color[2], intensity],
            spot_params: [0.0; 4],
        }
    }

    /// Create a spot light.
    /// `inner_cone`/`outer_cone`: half-angles in radians.
    pub fn spot(
        position: [f32; 3],
        direction: [f32; 3],
        color: [f32; 3],
        intensity: f32,
        range: f32,
        inner_cone: f32,
        outer_cone: f32,
    ) -> Self {
        Self {
            position_type: [
                position[0],
                position[1],
                position[2],
                LightType::Spot as u32 as f32,
            ],
            direction_range: [direction[0], direction[1], direction[2], range],
            color_intensity: [color[0], color[1], color[2], intensity],
            spot_params: [inner_cone.cos(), outer_cone.cos(), 0.0, 0.0],
        }
    }
}

/// Light array uniform — fixed-size array of lights + count.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightArrayUniforms {
    /// xyz = ambient color, w = ambient intensity
    pub ambient: [f32; 4],
    /// Number of active lights (as u32 bits in f32)
    pub light_count: [f32; 4], // x = count, yzw unused (padding)
    /// Light array
    pub lights: [GpuLight; MAX_LIGHTS],
}

impl Default for LightArrayUniforms {
    fn default() -> Self {
        Self {
            ambient: [1.0, 1.0, 1.0, 0.1],
            light_count: [0.0; 4],
            lights: [GpuLight {
                position_type: [0.0; 4],
                direction_range: [0.0; 4],
                color_intensity: [0.0; 4],
                spot_params: [0.0; 4],
            }; MAX_LIGHTS],
        }
    }
}

impl LightArrayUniforms {
    /// Set the lights from a slice (up to MAX_LIGHTS).
    pub fn set_lights(&mut self, lights: &[GpuLight]) {
        let count = lights.len().min(MAX_LIGHTS);
        self.light_count[0] = count as f32;
        for (i, light) in lights.iter().take(count).enumerate() {
            self.lights[i] = *light;
        }
    }

    /// Set ambient color and intensity.
    pub fn set_ambient(&mut self, color: [f32; 3], intensity: f32) {
        self.ambient = [color[0], color[1], color[2], intensity];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_light_size() {
        assert_eq!(std::mem::size_of::<GpuLight>(), 64); // 4 * vec4 = 4 * 16
    }

    #[test]
    fn light_array_uniforms_size() {
        let expected = 4 * 4 + 4 * 4 + 64 * MAX_LIGHTS; // ambient + count + lights
        assert_eq!(std::mem::size_of::<LightArrayUniforms>(), expected);
    }

    #[test]
    fn directional_light() {
        let l = GpuLight::directional([0.0, -1.0, 0.0], [1.0, 1.0, 1.0], 1.0);
        assert_eq!(l.position_type[3], 0.0); // Directional = 0
        assert_eq!(l.color_intensity[3], 1.0);
    }

    #[test]
    fn point_light() {
        let l = GpuLight::point([5.0, 3.0, 0.0], [1.0, 0.0, 0.0], 2.0, 10.0);
        assert_eq!(l.position_type[3], 1.0); // Point = 1
        assert_eq!(l.direction_range[3], 10.0); // range
    }

    #[test]
    fn spot_light() {
        let l = GpuLight::spot(
            [0.0, 5.0, 0.0],
            [0.0, -1.0, 0.0],
            [1.0, 1.0, 1.0],
            3.0,
            15.0,
            0.3,
            0.5,
        );
        assert_eq!(l.position_type[3], 2.0); // Spot = 2
        assert_eq!(l.direction_range[3], 15.0); // range
        assert!((l.spot_params[0] - 0.3_f32.cos()).abs() < 0.001);
    }

    #[test]
    fn light_array_set_lights() {
        let mut arr = LightArrayUniforms::default();
        let lights = vec![
            GpuLight::directional([0.0, -1.0, 0.0], [1.0; 3], 1.0),
            GpuLight::point([0.0, 3.0, 0.0], [1.0, 0.0, 0.0], 2.0, 10.0),
        ];
        arr.set_lights(&lights);
        assert_eq!(arr.light_count[0], 2.0);
    }

    #[test]
    fn light_array_clamps_to_max() {
        let mut arr = LightArrayUniforms::default();
        let lights: Vec<GpuLight> = (0..20)
            .map(|_| GpuLight::directional([0.0, -1.0, 0.0], [1.0; 3], 1.0))
            .collect();
        arr.set_lights(&lights);
        assert_eq!(arr.light_count[0], MAX_LIGHTS as f32);
    }

    #[test]
    fn light_array_bytemuck() {
        let arr = LightArrayUniforms::default();
        let bytes = bytemuck::bytes_of(&arr);
        assert_eq!(bytes.len(), std::mem::size_of::<LightArrayUniforms>());
    }
}
