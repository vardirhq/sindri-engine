//! Column-major affine math without a new asset-crate dependency.
pub(super) type Matrix = [[f32; 4]; 4];
pub(super) const IDENTITY: Matrix = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

fn valid(matrix: Matrix) -> bool {
    matrix.iter().flatten().all(|v| v.is_finite())
        && matrix[..3].iter().all(|column| column[3].abs() <= 0.0)
        && (matrix[3][3] - 1.0).abs() <= 0.0
        && determinant(matrix).abs() > 0.0
}

pub(super) fn determinant(m: Matrix) -> f64 {
    let [a, b, c] = [m[0], m[1], m[2]].map(|v| v.map(f64::from));
    a[0] * (b[1] * c[2] - b[2] * c[1]) - b[0] * (a[1] * c[2] - a[2] * c[1])
        + c[0] * (a[1] * b[2] - a[2] * b[1])
}

pub(super) fn compose(parent: Matrix, local: Matrix) -> Option<Matrix> {
    if !valid(local) {
        return None;
    }
    let mut output = [[0.0; 4]; 4];
    for (column, values) in output.iter_mut().enumerate() {
        for (row, value) in values.iter_mut().enumerate() {
            *value = narrow(
                (0..4)
                    .map(|i| f64::from(parent[i][row]) * f64::from(local[column][i]))
                    .sum(),
            )?;
        }
    }
    valid(output).then_some(output)
}

pub(super) fn point(m: Matrix, p: [f32; 3]) -> Option<[f32; 3]> {
    if !p.into_iter().all(f32::is_finite) {
        return None;
    }
    let mut output = [0.0; 3];
    for (row, value) in output.iter_mut().enumerate() {
        *value = narrow(
            f64::from(m[3][row])
                + (0..3)
                    .map(|i| f64::from(m[i][row]) * f64::from(p[i]))
                    .sum::<f64>(),
        )?;
    }
    Some(output)
}

pub(super) fn has_area(points: [[f32; 3]; 3]) -> bool {
    let [a, b, c] = points.map(|p| p.map(f64::from));
    let b: [f64; 3] = std::array::from_fn(|i| b[i] - a[i]);
    let c: [f64; 3] = std::array::from_fn(|i| c[i] - a[i]);
    [
        b[1] * c[2] - b[2] * c[1],
        b[2] * c[0] - b[0] * c[2],
        b[0] * c[1] - b[1] * c[0],
    ]
    .into_iter()
    .any(|axis| axis.abs() > 0.0)
}

fn narrow(value: f64) -> Option<f32> {
    if !value.is_finite() || value.abs() > f64::from(f32::MAX) {
        return None;
    }
    // Finiteness and f32 range are checked before this deliberate narrowing.
    #[allow(clippy::cast_possible_truncation)]
    let result = value as f32;
    Some(result)
}
