use byteorder::{LittleEndian, ReadBytesExt};
use itertools::Itertools;
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub mod bsp;
pub mod builtins;
pub mod mdl;

mod pak;
mod wad;

pub trait FromBytes: Sized + Sync + Send {
    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self>;
}

impl FromBytes for Vec<u8> {
    fn from_bytes(data: &[u8]) -> anyhow::Result<Self> {
        Ok(data.to_vec())
    }
}

impl FromBytes for String {
    fn from_bytes(data: &[u8]) -> anyhow::Result<Self> {
        Ok(String::from_utf8_lossy(data).to_string())
    }
}

pub struct Resources {
    base_path: std::path::PathBuf,
    pak: pak::Pak,
    cache: RwLock<HashMap<String, Arc<dyn Any + Send + Sync>>>,
}

impl Resources {
    pub fn new<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<std::path::Path>,
    {
        let path_ref = path.as_ref();

        if !path_ref.exists() || !path_ref.is_dir() {
            return Err(anyhow::anyhow!("Directory not found or is not a directory"));
        }

        let base_path = path.as_ref().to_owned().canonicalize()?;
        let pak = pak::Pak::new(path.as_ref())?;

        Ok(Self {
            base_path,
            pak,
            cache: RwLock::new(HashMap::new()),
        })
    }

    pub fn by_name<T: FromBytes>(&self, name: &str) -> anyhow::Result<T> {
        // Try loading from filesystem first, then fall back to PAK archives
        self.load_from_filesystem(name)
            .or_else(|_| self.pak.by_name(name))
    }

    pub fn by_cached_name<T: FromBytes + 'static>(&self, name: &str) -> anyhow::Result<Arc<T>> {
        let cache = self.cache.read().map_err(|e| anyhow::anyhow!("{}", e))?;
        if let Some(cached_data) = cache.get(name) {
            let typed_data = cached_data
                .clone()
                .downcast::<T>()
                .map_err(|_| anyhow::anyhow!("Cached data has wrong type for: {}", name))?;
            return Ok(typed_data);
        }
        drop(cache); // Release read lock before acquiring write lock

        let data = Arc::new(self.by_name::<T>(name)?);

        let mut cache = self.cache.write().map_err(|e| anyhow::anyhow!("{}", e))?;
        cache.insert(name.to_owned(), data.clone());
        Ok(data)
    }

    pub fn cached_names(&self) -> impl Iterator<Item = String> {
        let cache = self.cache.read().unwrap();
        cache.keys().cloned().collect::<Vec<_>>().into_iter()
    }

    pub fn file_names(&self) -> impl Iterator<Item = String> {
        let pattern = format!("{}/**/*", self.base_path.display());
        let base_files = glob::glob(&pattern)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|path| path.is_file())
            .filter(|path| {
                !path
                    .extension()
                    .map(|ext| ext.to_ascii_lowercase() == "pak")
                    .unwrap_or(false)
            })
            .map(|path| {
                path.strip_prefix(&self.base_path)
                    .ok()
                    .and_then(|p| p.to_str())
                    .unwrap()
                    .to_owned()
            })
            .collect::<Vec<_>>();

        base_files.into_iter().chain(self.pak.file_names()).sorted()
    }

    pub fn flush(&self) -> anyhow::Result<()> {
        let mut cache = self.cache.write().map_err(|e| anyhow::anyhow!("{}", e))?;
        cache.clear();
        Ok(())
    }

    fn load_from_filesystem<T: FromBytes>(&self, name: &str) -> anyhow::Result<T> {
        let path = self
            .base_path
            .join(name)
            .canonicalize()
            .map_err(|_| anyhow::anyhow!("File not found in filesystem: {}", name))?;

        let bytes = std::fs::read(path)?;
        T::from_bytes(&bytes)
    }
}

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

pub fn read_f32_bounding_sphere<R>(reader: &mut R) -> anyhow::Result<BoundingVolume>
where
    R: std::io::Read,
{
    BoundingVolume::read_bounding_sphere_at_origin_with(reader, |r| {
        Ok(r.read_f32::<LittleEndian>()?)
    })
}

pub fn read_i16_bounding_box<R>(reader: &mut R) -> anyhow::Result<BoundingVolume>
where
    R: std::io::Read,
{
    BoundingVolume::read_bounding_box_with(reader, |r| read_i16_vector3_as_f32(r))
}

pub fn read_f32_bounding_box<R>(reader: &mut R) -> anyhow::Result<BoundingVolume>
where
    R: std::io::Read,
{
    BoundingVolume::read_bounding_box_with(reader, |r| read_f32_vector3(r))
}

pub fn read_scaled_position_bounding_box<R>(
    reader: &mut R,
    scale: glam::Vec3,
    translate: glam::Vec3,
) -> anyhow::Result<BoundingVolume>
where
    R: std::io::Read,
{
    BoundingVolume::read_bounding_box_with(reader, |r| {
        let point = read_scaled_position(r, scale, translate)?;
        r.read_u8()? as f32;

        Ok(point)
    })
}

pub fn read_f32_vector3<R>(reader: &mut R) -> anyhow::Result<glam::Vec3>
where
    R: std::io::Read,
{
    let vector = [
        reader.read_f32::<LittleEndian>()?,
        reader.read_f32::<LittleEndian>()?,
        reader.read_f32::<LittleEndian>()?,
    ];

    // Swaps Y↔Z axes to convert from Quake's coordinate system to standard 3D
    Ok([vector[0], vector[2], -vector[1]].into())
}

pub fn read_i16_vector3_as_f32<R>(reader: &mut R) -> anyhow::Result<glam::Vec3>
where
    R: std::io::Read,
{
    let vector = [
        reader.read_i16::<LittleEndian>()? as f32,
        reader.read_i16::<LittleEndian>()? as f32,
        reader.read_i16::<LittleEndian>()? as f32,
    ];

    // Swaps Y↔Z axes to convert from Quake's coordinate system to standard 3D
    Ok([vector[0], vector[2], -vector[1]].into())
}

pub fn read_scaled_position<R>(
    reader: &mut R,
    scale: glam::Vec3,
    translate: glam::Vec3,
) -> anyhow::Result<glam::Vec3>
where
    R: std::io::Read,
{
    let vector = [
        reader.read_u8()? as f32 * scale[0] + translate[0],
        reader.read_u8()? as f32 * scale[1] + translate[1],
        reader.read_u8()? as f32 * scale[2] + translate[2],
    ];

    // Swaps Y↔Z axes to convert from Quake's coordinate system to standard 3D
    Ok([vector[0], vector[2], -vector[1]].into())
}

pub fn read_null_terminated_string<R>(reader: &mut R, buffer_size: usize) -> anyhow::Result<String>
where
    R: std::io::Read,
{
    let mut name_buffer = vec![0u8; buffer_size];
    reader.read_exact(&mut name_buffer)?;
    let null_terminated_bytes: Vec<u8> = name_buffer
        .iter()
        .take_while(|&byte| *byte != 0)
        .copied()
        .collect();
    Ok(String::from_utf8_lossy(&null_terminated_bytes).to_string())
}
