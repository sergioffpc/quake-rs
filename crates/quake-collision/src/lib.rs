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
        F: Fn(&mut R) -> anyhow::Result<glam::Vec3>,
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
            BoundingVolume::Sphere { center, .. } => {
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

    pub fn interpolate_with<F>(&self, other: &Self, t: f32, interpolate_fn: F) -> Self
    where
        F: Fn(f32, f32, f32) -> f32,
    {
        match (self, other) {
            (
                Self::Sphere {
                    center: c1,
                    radius: r1,
                },
                Self::Sphere {
                    center: c2,
                    radius: r2,
                },
            ) => Self::Sphere {
                center: Self::interpolate_vec3(&c1, &c2, t, &interpolate_fn),
                radius: interpolate_fn(*r1, *r2, t),
            },
            (
                Self::Box {
                    min: min1,
                    max: max1,
                },
                Self::Box {
                    min: min2,
                    max: max2,
                },
            ) => Self::Box {
                min: Self::interpolate_vec3(&min1, &min2, t, &interpolate_fn),
                max: Self::interpolate_vec3(&max1, &max2, t, &interpolate_fn),
            },
            _ => panic!("Unsupported bounding volumes"),
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
