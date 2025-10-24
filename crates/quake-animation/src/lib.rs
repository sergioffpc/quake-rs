use glam::FloatExt;

#[derive(Clone, Debug)]
pub struct KeyFrame {
    positions: Box<[glam::Vec3]>,
    bounding_volume: quake_collision::BoundingVolume,
}

impl KeyFrame {
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        Self::interpolate_with(self, other, t, f32::lerp)
    }

    pub fn interpolate_with<F>(kf1: &Self, kf2: &Self, t: f32, interpolate_fn: F) -> Self
    where
        F: Fn(f32, f32, f32) -> f32,
    {
        let positions_count = kf1.positions.len();
        let mut interpolated_positions = vec![glam::Vec3::ZERO; positions_count];
        for i in 0..positions_count {
            let p1 = kf1.positions[i];
            let p2 = kf2.positions[i];
            interpolated_positions[i] = Self::interpolate_vec3(&p1, &p2, t, &interpolate_fn);
        }

        let interpolated_bounding_volume =
            kf1.bounding_volume
                .interpolate_with(&kf2.bounding_volume, t, &interpolate_fn);

        Self {
            positions: interpolated_positions.into_boxed_slice(),
            bounding_volume: interpolated_bounding_volume,
        }
    }

    fn interpolate_vec3<F>(
        v1: &glam::Vec3,
        v2: &glam::Vec3,
        t: f32,
        interpolate_fn: F,
    ) -> glam::Vec3
    where
        F: Fn(f32, f32, f32) -> f32,
    {
        [
            interpolate_fn(v1[0], v2[0], t),
            interpolate_fn(v1[1], v2[1], t),
            interpolate_fn(v1[2], v2[2], t),
        ]
        .into()
    }
}
