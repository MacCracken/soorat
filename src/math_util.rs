//! Shared math utilities for matrix and vector operations.

/// Identity 4x4 matrix (column-major).
pub const IDENTITY_MAT4: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

/// Multiply two 4x4 column-major matrices: result = a * b.
#[must_use]
#[inline]
pub fn mul_mat4(a: [f32; 16], b: [f32; 16]) -> [f32; 16] {
    let mut r = [0.0f32; 16];
    for col in 0..4 {
        for row in 0..4 {
            let mut sum = 0.0;
            for k in 0..4 {
                sum += a[k * 4 + row] * b[col * 4 + k];
            }
            r[col * 4 + row] = sum;
        }
    }
    r
}

/// Normalize a 3D vector.
///
/// If the vector length is near zero (< 1e-10), returns `[0.0, 0.0, 1.0]` as a Z-up fallback
/// to avoid division by zero. This convention aligns with the engine's Z-up coordinate system
/// and ensures downstream cross-product and matrix operations remain numerically stable.
#[must_use]
#[inline]
pub fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len < 1e-10 {
        return [0.0, 0.0, 1.0];
    }
    [v[0] / len, v[1] / len, v[2] / len]
}

/// Cross product of two 3D vectors.
#[must_use]
#[inline]
pub fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// 90° perspective projection matrix (aspect=1, fov=90°). For cube shadow map faces.
///
/// If `near` and `far` are effectively equal (difference < 1e-10), returns an identity matrix
/// to avoid division by zero.
#[must_use]
#[inline]
pub fn perspective_90(near: f32, far: f32) -> [f32; 16] {
    if (near - far).abs() < 1e-10 {
        tracing::warn!(
            near,
            far,
            "perspective_90: near ≈ far — returning identity matrix"
        );
        return IDENTITY_MAT4;
    }
    let nf = 1.0 / (near - far);
    [
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        far * nf,
        -1.0,
        0.0,
        0.0,
        near * far * nf,
        0.0,
    ]
}

/// Look-at view matrix from a position along a direction.
#[must_use]
#[inline]
pub fn look_at(pos: [f32; 3], dir: [f32; 3], up: [f32; 3]) -> [f32; 16] {
    let f = normalize3(dir);
    let s = normalize3(cross(f, up));
    let u = cross(s, f);

    [
        s[0],
        u[0],
        -f[0],
        0.0,
        s[1],
        u[1],
        -f[1],
        0.0,
        s[2],
        u[2],
        -f[2],
        0.0,
        -(s[0] * pos[0] + s[1] * pos[1] + s[2] * pos[2]),
        -(u[0] * pos[0] + u[1] * pos[1] + u[2] * pos[2]),
        f[0] * pos[0] + f[1] * pos[1] + f[2] * pos[2],
        1.0,
    ]
}

/// Flatten a `[[f32; 4]; 4]` to `[f32; 16]` (preserving memory layout, no transpose).
///
/// glTF stores matrices column-major; the `gltf` crate returns `[[f32; 4]; 4]`
/// where each inner array is a column. This flattens to a contiguous `[f32; 16]`
/// suitable for GPU uniform upload.
#[must_use]
#[inline]
pub fn flatten_mat4(m: [[f32; 4]; 4]) -> [f32; 16] {
    [
        m[0][0], m[0][1], m[0][2], m[0][3], m[1][0], m[1][1], m[1][2], m[1][3], m[2][0], m[2][1],
        m[2][2], m[2][3], m[3][0], m[3][1], m[3][2], m[3][3],
    ]
}

/// Compose Translation + Rotation (quaternion xyzw) + Scale into a 4x4 column-major matrix.
#[must_use]
#[inline]
pub fn compose_trs(t: [f32; 3], r: [f32; 4], s: [f32; 3]) -> [f32; 16] {
    let (x, y, z, w) = (r[0], r[1], r[2], r[3]);

    let r00 = 1.0 - 2.0 * (y * y + z * z);
    let r01 = 2.0 * (x * y + w * z);
    let r02 = 2.0 * (x * z - w * y);
    let r10 = 2.0 * (x * y - w * z);
    let r11 = 1.0 - 2.0 * (x * x + z * z);
    let r12 = 2.0 * (y * z + w * x);
    let r20 = 2.0 * (x * z + w * y);
    let r21 = 2.0 * (y * z - w * x);
    let r22 = 1.0 - 2.0 * (x * x + y * y);

    [
        s[0] * r00,
        s[0] * r01,
        s[0] * r02,
        0.0,
        s[1] * r10,
        s[1] * r11,
        s[1] * r12,
        0.0,
        s[2] * r20,
        s[2] * r21,
        s[2] * r22,
        0.0,
        t[0],
        t[1],
        t[2],
        1.0,
    ]
}

/// Compute a right/up orthonormal basis from a normal (direction) vector.
///
/// Given a direction `n`, returns `(right, up)` such that `right`, `up`, and `n`
/// form an approximately orthogonal frame. Used for portal rendering, arrowheads,
/// and any geometry that needs a tangent frame from a single direction.
#[must_use]
#[inline]
pub fn normal_to_basis(n: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    // Pick a reference that isn't parallel to n
    let ref_vec = if n[1].abs() < 0.99 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };

    // right = normalize(cross(n, ref_vec))
    let rx = n[1] * ref_vec[2] - n[2] * ref_vec[1];
    let ry = n[2] * ref_vec[0] - n[0] * ref_vec[2];
    let rz = n[0] * ref_vec[1] - n[1] * ref_vec[0];
    let rlen = (rx * rx + ry * ry + rz * rz).sqrt().max(f32::EPSILON);
    let right = [rx / rlen, ry / rlen, rz / rlen];

    // up = normalize(cross(right, n))
    let ux = right[1] * n[2] - right[2] * n[1];
    let uy = right[2] * n[0] - right[0] * n[2];
    let uz = right[0] * n[1] - right[1] * n[0];
    let ulen = (ux * ux + uy * uy + uz * uz).sqrt().max(f32::EPSILON);
    let up = [ux / ulen, uy / ulen, uz / ulen];

    (right, up)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mul_mat4_identity() {
        assert_eq!(mul_mat4(IDENTITY_MAT4, IDENTITY_MAT4), IDENTITY_MAT4);
    }

    #[test]
    fn normalize3_unit() {
        let n = normalize3([3.0, 0.0, 4.0]);
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        assert!((len - 1.0).abs() < 0.001);
    }

    #[test]
    fn normalize3_zero() {
        let n = normalize3([0.0, 0.0, 0.0]);
        assert_eq!(n, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn cross_product_basis() {
        assert!((cross([1.0, 0.0, 0.0], [0.0, 1.0, 0.0])[2] - 1.0).abs() < 0.001);
        assert!((cross([0.0, 1.0, 0.0], [1.0, 0.0, 0.0])[2] + 1.0).abs() < 0.001);
    }

    #[test]
    fn look_at_parallel_vectors() {
        // dir parallel to up — should not produce NaN
        let m = look_at([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0]);
        for v in m {
            assert!(!v.is_nan(), "NaN in look_at with parallel dir/up");
        }
    }

    #[test]
    fn look_at_zero_direction() {
        // zero direction and zero pos — should not produce NaN or panic
        let m = look_at([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        for v in m {
            assert!(!v.is_nan(), "NaN in look_at with zero direction");
        }
    }

    #[test]
    fn flatten_mat4_identity() {
        let identity_nested = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        assert_eq!(flatten_mat4(identity_nested), IDENTITY_MAT4);
    }

    #[test]
    fn flatten_mat4_roundtrip() {
        let m = [
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        ];
        let flat = flatten_mat4(m);
        // Each inner array becomes 4 consecutive floats in column-major order
        assert_eq!(flat[0], 1.0); // m[0][0]
        assert_eq!(flat[4], 5.0); // m[1][0]
        assert_eq!(flat[8], 9.0); // m[2][0]
        assert_eq!(flat[12], 13.0); // m[3][0]
        assert_eq!(flat[15], 16.0); // m[3][3]
    }

    #[test]
    fn compose_trs_zero_scale() {
        let m = compose_trs([1.0, 2.0, 3.0], [0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 0.0]);
        for v in m {
            assert!(!v.is_nan(), "NaN in compose_trs with zero scale");
        }
        // Rotation columns should be zeroed out by zero scale
        assert_eq!(m[0], 0.0);
        assert_eq!(m[5], 0.0);
        assert_eq!(m[10], 0.0);
        // Translation should still be present
        assert_eq!(m[12], 1.0);
        assert_eq!(m[13], 2.0);
        assert_eq!(m[14], 3.0);
    }

    #[test]
    fn compose_trs_rotation_only() {
        // 90° rotation around Z axis: quat = (0, 0, sin(45°), cos(45°))
        let s = std::f32::consts::FRAC_PI_4.sin();
        let c = std::f32::consts::FRAC_PI_4.cos();
        let m = compose_trs([0.0; 3], [0.0, 0.0, s, c], [1.0; 3]);
        // After 90° Z rotation, X axis should map to Y axis
        // m[0..4] is first column (original X direction)
        assert!((m[0]).abs() < 0.01); // x-component near zero
        assert!((m[1] - 1.0).abs() < 0.01); // y-component near 1
        assert_eq!(m[15], 1.0); // homogeneous w
    }

    fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }

    #[test]
    fn normal_to_basis_z_forward() {
        let n = [0.0, 0.0, 1.0];
        let (right, up) = normal_to_basis(n);
        assert!(dot3(right, n).abs() < 0.001, "right not perpendicular to n");
        assert!(dot3(up, n).abs() < 0.001, "up not perpendicular to n");
    }

    #[test]
    fn normal_to_basis_y_up() {
        let n = [0.0, 1.0, 0.0];
        let (right, up) = normal_to_basis(n);
        assert!(dot3(right, n).abs() < 0.001, "right not perpendicular to n");
        assert!(dot3(up, n).abs() < 0.001, "up not perpendicular to n");
    }

    #[test]
    fn normal_to_basis_orthogonal() {
        // Test with an arbitrary direction
        let n = normalize3([1.0, 2.0, 3.0]);
        let (right, up) = normal_to_basis(n);
        assert!(dot3(right, up).abs() < 0.001, "right and up not orthogonal");
        assert!(dot3(right, n).abs() < 0.001, "right not perpendicular to n");
        assert!(dot3(up, n).abs() < 0.001, "up not perpendicular to n");
    }
}
