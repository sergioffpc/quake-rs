#[derive(Copy, Clone, Debug)]
pub enum BoundingVolume {
    Sphere { radius: f32 },
    Box { min: [f32; 3], max: [f32; 3] },
}

impl BoundingVolume {
    pub fn read_bounding_sphere_with<R, F>(
        reader: &mut R,
        read_vector_fn: F,
    ) -> anyhow::Result<Self>
    where
        R: std::io::Read,
        F: Fn(&mut R) -> anyhow::Result<f32>,
    {
        let radius = read_vector_fn(reader)?;

        Ok(Self::Sphere { radius })
    }

    pub fn read_bounding_box_with<R, F>(reader: &mut R, read_vector_fn: F) -> anyhow::Result<Self>
    where
        R: std::io::Read,
        F: Fn(&mut R) -> anyhow::Result<[f32; 3]>,
    {
        let min = read_vector_fn(reader)?;
        let max = read_vector_fn(reader)?;

        Ok(Self::Box { min, max })
    }

    pub fn intersects(&self, other: &BoundingVolume) -> bool {
        match (self, other) {
            (BoundingVolume::Sphere { radius: r1 }, BoundingVolume::Sphere { radius: r2 }) => {
                // For spheres without centers, assume they're at origin
                // This case might need center positions in practice
                r1 + r2 > 0.0
            }
            (
                BoundingVolume::Box {
                    min: min1,
                    max: max1,
                },
                BoundingVolume::Box {
                    min: min2,
                    max: max2,
                },
            ) => Self::box_intersects_box(*min1, *max1, *min2, *max2),
            (BoundingVolume::Sphere { radius }, BoundingVolume::Box { min, max })
            | (BoundingVolume::Box { min, max }, BoundingVolume::Sphere { radius }) => {
                // For sphere-box intersection, assume the sphere is centered at origin
                Self::sphere_intersects_box([0.0, 0.0, 0.0], *radius, *min, *max)
            }
        }
    }

    fn sphere_intersects_box(center: [f32; 3], radius: f32, min: [f32; 3], max: [f32; 3]) -> bool {
        // Find the closest point on the box to the sphere center
        let closest_point = [
            center[0].clamp(min[0], max[0]),
            center[1].clamp(min[1], max[1]),
            center[2].clamp(min[2], max[2]),
        ];

        // Calculate squared distance from sphere center to the closest point
        let distance_squared = (0..3)
            .map(|i| (center[i] - closest_point[i]).powi(2))
            .sum::<f32>();

        distance_squared <= radius * radius
    }

    fn box_intersects_box(min1: [f32; 3], max1: [f32; 3], min2: [f32; 3], max2: [f32; 3]) -> bool {
        // Two boxes intersect if they overlap on all three axes
        (min1[0] <= max2[0] && max1[0] >= min2[0])
            && (min1[1] <= max2[1] && max1[1] >= min2[1])
            && (min1[2] <= max2[2] && max1[2] >= min2[2])
    }
}
