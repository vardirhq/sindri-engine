//! Timing curves shared by gameplay tween sampling and Weave transitions.

/// The named CSS timing curves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Easing {
    Linear,
    Ease,
    EaseIn,
    EaseOut,
    EaseInOut,
}

impl Easing {
    /// Reads a named curve; unknown names are refused.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Some(match name {
            "linear" => Self::Linear,
            "ease" => Self::Ease,
            "ease-in" => Self::EaseIn,
            "ease-out" => Self::EaseOut,
            "ease-in-out" => Self::EaseInOut,
            _ => return None,
        })
    }

    /// Samples normalized time, clamped to the endpoints.
    ///
    /// Callers validate that time is finite. Endpoints are exact so a
    /// finished tween reaches its destination without residual error.
    #[must_use]
    pub fn apply(self, t: f32) -> f32 {
        if t <= 0.0 { return 0.0; }
        if t >= 1.0 { return 1.0; }
        let (x1, y1, x2, y2) = match self {
            Self::Linear => return t,
            Self::Ease => (0.25, 0.1, 0.25, 1.0),
            Self::EaseIn => (0.42, 0.0, 1.0, 1.0),
            Self::EaseOut => (0.0, 0.0, 0.58, 1.0),
            Self::EaseInOut => (0.42, 0.0, 0.58, 1.0),
        };
        let bezier = |a: f32, b: f32, s: f32| {
            let inverse = 1.0 - s;
            3.0 * inverse * inverse * s * a + 3.0 * inverse * s * s * b + s * s * s
        };
        let (mut low, mut high) = (0.0_f32, 1.0_f32);
        for _ in 0..20 {
            let middle = f32::midpoint(low, high);
            if bezier(x1, x2, middle) < t { low = middle; }
            else { high = middle; }
        }
        bezier(y1, y2, f32::midpoint(low, high))
    }
}

#[cfg(test)]
mod tests {
    use super::Easing;

    #[test]
    fn curves_have_exact_endpoints_and_monotonic_progress() {
        for name in ["linear", "ease", "ease-in", "ease-out", "ease-in-out"] {
            let curve = Easing::named(name).expect("known curve");
            assert!(curve.apply(-1.0).abs() < f32::EPSILON);
            assert!((curve.apply(2.0) - 1.0).abs() < f32::EPSILON);
            let mut previous = 0.0;
            for step in 0..=100_u16 {
                let value = curve.apply(f32::from(step) / 100.0);
                assert!(value >= previous && value <= 1.0);
                previous = value;
            }
        }
        assert!(Easing::named("typo").is_none());
        assert!(Easing::EaseIn.apply(0.5) < 0.5);
        assert!(Easing::EaseOut.apply(0.5) > 0.5);
        assert!((Easing::EaseInOut.apply(0.5) - 0.5).abs() < 0.00001);
    }
}
