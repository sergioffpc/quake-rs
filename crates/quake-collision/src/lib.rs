#[derive(Copy, Clone, Debug)]
pub enum BoundingVolume {
    Sphere { center: glam::Vec3, radius: f32 },
    Box { min: glam::Vec3, max: glam::Vec3 },
}

impl BoundingVolume {
    pub fn read_bounding_sphere_at_origin_with<R, F>(
        reader: &mut R,
        read_vector_fn: F,
    ) -> anyhow::Result<Self>
    where
        R: std::io::Read,
        F: Fn(&mut R) -> anyhow::Result<f32>,
    {
        let radius = read_vector_fn(reader)?;

        Ok(Self::Sphere {
            center: glam::Vec3::ZERO,
            radius,
        })
    }

    pub fn read_bounding_box_with<R, F>(reader: &mut R, read_vector_fn: F) -> anyhow::Result<Self>
    where
        R: std::io::Read,
        F: Fn(&mut R) -> anyhow::Result<[f32; 3]>,
    {
        let min = read_vector_fn(reader)?.into();
        let max = read_vector_fn(reader)?.into();

        Ok(Self::Box { min, max })
    }

    pub fn transform(&self, transform: &glam::Mat4) -> Self {
        let mut transformed = *self;
        transformed.transform_mut(transform);
        transformed
    }

    pub fn transform_mut(&mut self, transform: &glam::Mat4) {
        match self {
            BoundingVolume::Sphere { center, radius } => {
                *center = transform.transform_point3(*center);
            }
            BoundingVolume::Box { min, max } => {
                *min = transform.transform_point3(*min);
                *max = transform.transform_point3(*max);
            }
        }
    }

    pub fn intersects(&self, other: &BoundingVolume) -> bool {
        match (self, other) {
            (
                BoundingVolume::Sphere {
                    center: c1,
                    radius: r1,
                },
                BoundingVolume::Sphere {
                    center: c2,
                    radius: r2,
                },
            ) => Self::sphere_intersects_sphere(*c1, *r1, *c2, *r2),
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
            (BoundingVolume::Sphere { center, radius }, BoundingVolume::Box { min, max })
            | (BoundingVolume::Box { min, max }, BoundingVolume::Sphere { center, radius }) => {
                Self::sphere_intersects_box(*center, *radius, *min, *max)
            }
        }
    }

    fn sphere_intersects_sphere(
        center1: glam::Vec3,
        radius1: f32,
        center2: glam::Vec3,
        radius2: f32,
    ) -> bool {
        let distance = (center1 - center2).length();
        distance <= radius1 + radius2
    }

    fn sphere_intersects_box(
        center: glam::Vec3,
        radius: f32,
        min: glam::Vec3,
        max: glam::Vec3,
    ) -> bool {
        // Find the closest point on the box to the sphere center
        let closest_point = glam::Vec3::new(
            center[0].clamp(min[0], max[0]),
            center[1].clamp(min[1], max[1]),
            center[2].clamp(min[2], max[2]),
        );

        // Calculate squared distance from sphere center to the closest point
        let distance = (center - closest_point).length();
        distance <= radius * radius
    }

    fn box_intersects_box(
        min1: glam::Vec3,
        max1: glam::Vec3,
        min2: glam::Vec3,
        max2: glam::Vec3,
    ) -> bool {
        // Two boxes intersect if they overlap on all three axes
        (min1[0] <= max2[0] && max1[0] >= min2[0])
            && (min1[1] <= max2[1] && max1[1] >= min2[1])
            && (min1[2] <= max2[2] && max1[2] >= min2[2])
    }
}
