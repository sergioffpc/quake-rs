use itertools::Itertools;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::sync::RwLock;

pub mod bsp;
pub mod commands;
pub mod mdl;

mod pak;
mod wad;

pub struct ResourcesManager {
    base_path: std::path::PathBuf,
    pak: pak::Pak,
    cache: RwLock<HashMap<String, Arc<dyn Any + Send + Sync>>>,
}

impl ResourcesManager {
    pub async fn new<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<std::path::Path>,
    {
        let path_ref = path.as_ref();

        if !path_ref.exists() || !path_ref.is_dir() {
            return Err(anyhow::anyhow!("Directory not found or is not a directory"));
        }

        let base_path = path.as_ref().to_owned().canonicalize()?;
        let pak = pak::Pak::new(path.as_ref()).await?;

        Ok(Self {
            base_path,
            pak,
            cache: RwLock::new(HashMap::new()),
        })
    }

    pub async fn by_name<T: quake_traits::FromBytes>(&self, name: &str) -> anyhow::Result<T> {
        // Try loading from filesystem first, then fall back to PAK archives
        match self.load_from_filesystem(name).await {
            Ok(result) => Ok(result),
            Err(_) => self.pak.by_name(name).await,
        }
    }

    pub async fn by_cached_name<T: quake_traits::FromBytes + 'static>(
        &self,
        name: &str,
    ) -> anyhow::Result<Arc<T>> {
        if let Some(cached_data) = self.cache.read().await.get(name) {
            let typed_data = cached_data
                .clone()
                .downcast::<T>()
                .map_err(|_| anyhow::anyhow!("Cached data has wrong type for: {}", name))?;
            return Ok(typed_data);
        }

        let data = Arc::new(self.by_name::<T>(name).await?);

        self.cache
            .write()
            .await
            .insert(name.to_owned(), data.clone());
        Ok(data)
    }

    pub async fn cached_names(&self) -> impl Iterator<Item = String> {
        self.cache
            .read()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
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
                    .map(|ext| ext.eq_ignore_ascii_case("pak"))
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

    pub async fn flush(&self) -> anyhow::Result<()> {
        self.cache.write().await.clear();
        Ok(())
    }

    async fn load_from_filesystem<T: quake_traits::FromBytes>(
        &self,
        name: &str,
    ) -> anyhow::Result<T> {
        let path = self
            .base_path
            .join(name)
            .canonicalize()
            .map_err(|_| anyhow::anyhow!("File not found in filesystem: {}", name))?;

        let bytes = std::fs::read(path)?;
        T::from_bytes(&bytes).await
    }
}

#[derive(Copy, Clone, Debug)]
pub enum BoundingVolume {
    Sphere { center: glam::Vec3, radius: f32 },
    Box { min: glam::Vec3, max: glam::Vec3 },
}

impl BoundingVolume {
    pub async fn read_bounding_sphere_at_origin_with<R, F>(
        reader: &mut R,
        mut read_vector_fn: F,
    ) -> anyhow::Result<Self>
    where
        R: AsyncReadExt + Unpin + Send,
        F: for<'r> FnMut(
            &'r mut R,
        ) -> Pin<Box<dyn Future<Output = anyhow::Result<f32>> + Send + 'r>>,
    {
        let radius = read_vector_fn(reader).await?;

        Ok(Self::Sphere {
            center: glam::Vec3::ZERO,
            radius,
        })
    }

    pub async fn read_bounding_box_with<R, F>(
        reader: &mut R,
        mut read_vector_fn: F,
    ) -> anyhow::Result<Self>
    where
        R: AsyncReadExt + Unpin + Send,
        F: for<'r> FnMut(
            &'r mut R,
        )
            -> Pin<Box<dyn Future<Output = anyhow::Result<glam::Vec3>> + Send + 'r>>,
    {
        let min = read_vector_fn(reader).await?;
        let max = read_vector_fn(reader).await?;

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
                center: Self::interpolate_vec3(c1, c2, t, &interpolate_fn),
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
                min: Self::interpolate_vec3(min1, min2, t, &interpolate_fn),
                max: Self::interpolate_vec3(max1, max2, t, &interpolate_fn),
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

pub async fn read_f32_bounding_sphere<R>(reader: &mut R) -> anyhow::Result<BoundingVolume>
where
    R: AsyncReadExt + Unpin + Send,
{
    BoundingVolume::read_bounding_sphere_at_origin_with(reader, |r| {
        Box::pin(async move { Ok(r.read_f32_le().await?) })
    })
    .await
}

pub async fn read_i16_bounding_box<R>(reader: &mut R) -> anyhow::Result<BoundingVolume>
where
    R: AsyncReadExt + Unpin + Send,
{
    BoundingVolume::read_bounding_box_with(reader, |r| {
        Box::pin(async move { Ok(read_i16_vector3_as_f32(r).await?) })
    })
    .await
}

pub async fn read_f32_bounding_box<R>(reader: &mut R) -> anyhow::Result<BoundingVolume>
where
    R: AsyncReadExt + Unpin + Send,
{
    BoundingVolume::read_bounding_box_with(reader, |r| {
        Box::pin(async move { Ok(read_f32_vector3(r).await?) })
    })
    .await
}

pub async fn read_scaled_position_bounding_box<R>(
    reader: &mut R,
    scale: glam::Vec3,
    translate: glam::Vec3,
) -> anyhow::Result<BoundingVolume>
where
    R: AsyncReadExt + Unpin + Send,
{
    BoundingVolume::read_bounding_box_with(reader, |r| {
        Box::pin(async move {
            let point = read_scaled_position::<R>(r, scale, translate).await?;
            r.read_u8().await? as f32;
            Ok(point)
        })
    })
    .await
}

pub async fn read_f32_vector3<R>(reader: &mut R) -> anyhow::Result<glam::Vec3>
where
    R: AsyncReadExt + Unpin + Send,
{
    let vector = [
        reader.read_f32_le().await?,
        reader.read_f32_le().await?,
        reader.read_f32_le().await?,
    ];

    // Swaps Y↔Z axes to convert from Quake's coordinate system to standard 3D
    Ok([vector[0], vector[2], -vector[1]].into())
}

pub async fn read_i16_vector3_as_f32<R>(reader: &mut R) -> anyhow::Result<glam::Vec3>
where
    R: AsyncReadExt + Unpin + Send,
{
    let vector = [
        reader.read_i16_le().await? as f32,
        reader.read_i16_le().await? as f32,
        reader.read_i16_le().await? as f32,
    ];

    // Swaps Y↔Z axes to convert from Quake's coordinate system to standard 3D
    Ok([vector[0], vector[2], -vector[1]].into())
}

pub async fn read_scaled_position<R>(
    reader: &mut R,
    scale: glam::Vec3,
    translate: glam::Vec3,
) -> anyhow::Result<glam::Vec3>
where
    R: AsyncReadExt + Unpin + Send,
{
    let vector = [
        reader.read_u8().await? as f32 * scale[0] + translate[0],
        reader.read_u8().await? as f32 * scale[1] + translate[1],
        reader.read_u8().await? as f32 * scale[2] + translate[2],
    ];

    // Swaps Y↔Z axes to convert from Quake's coordinate system to standard 3D
    Ok([vector[0], vector[2], -vector[1]].into())
}

pub async fn read_null_terminated_string<R>(
    reader: &mut R,
    buffer_size: usize,
) -> anyhow::Result<String>
where
    R: AsyncReadExt + Unpin + Send,
{
    let mut name_buffer = vec![0u8; buffer_size];
    reader.read_exact(&mut name_buffer).await?;
    let null_terminated_bytes: Vec<u8> = name_buffer
        .iter()
        .take_while(|&byte| *byte != 0)
        .copied()
        .collect();
    Ok(String::from_utf8_lossy(&null_terminated_bytes).to_string())
}
