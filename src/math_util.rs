//! Shared math utilities for matrix and vector operations.

/// Identity 4x4 matrix (column-major).
pub const IDENTITY_MAT4: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

/// Multiply two 4x4 column-major matrices: result = a * b.
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
pub fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len < 1e-10 {
        return [0.0, 0.0, 1.0];
    }
    [v[0] / len, v[1] / len, v[2] / len]
}

/// Cross product of two 3D vectors.
pub fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
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
}
