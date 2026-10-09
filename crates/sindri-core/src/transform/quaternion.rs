//! Shared quaternion composition for transforms and their parent space.

pub(crate) fn multiply(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    let [ax, ay, az, aw] = a;
    let [bx, by, bz, bw] = b;
    [
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
        aw * bw - ax * bx - ay * by - az * bz,
    ]
}

pub(crate) const fn conjugate([x, y, z, w]: [f32; 4]) -> [f32; 4] {
    [-x, -y, -z, w]
}

pub(crate) fn normalized(quaternion: [f32; 4]) -> [f32; 4] {
    let length = quaternion
        .iter()
        .map(|part| f64::from(*part).powi(2))
        .sum::<f64>()
        .sqrt();
    if length > 0.0 && length.is_finite() {
        quaternion.map(|part| narrow(f64::from(part) / length))
    } else {
        [0.0, 0.0, 0.0, 1.0]
    }
}

pub(crate) fn rotate(rotation: [f32; 4], vector: [f32; 3]) -> [f32; 3] {
    let rotation = normalized(rotation);
    let [x, y, z, _] = multiply(
        multiply(rotation, [vector[0], vector[1], vector[2], 0.0]),
        conjugate(rotation),
    );
    [x, y, z]
}

// A normalized component is within [-1, 1].
#[allow(clippy::cast_possible_truncation)]
fn narrow(value: f64) -> f32 {
    value as f32
}
