use crate::{Archive, FromBytes};
use byteorder::{LittleEndian, ReadBytesExt};
use glam::{Mat4, Vec3};
use itertools::Itertools;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

pub mod bsp;
pub mod dem;
pub mod mdl;
pub mod wad;

#[derive(Debug)]
pub struct Pak {
    archives: Box<[PakArchive]>,
}

impl Pak {
    pub fn new<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let files = Self::find_pak_files(path)?;
        let mut archives = Vec::with_capacity(files.len());
        for f in files {
            archives.push(PakArchive::new(f.as_path())?);
        }

        Ok(Self {
            archives: archives.into_boxed_slice(),
        })
    }

    pub fn by_name<T: FromBytes>(&self, name: &str) -> anyhow::Result<T> {
        for archive in &self.archives {
            if let Ok(data) = archive.by_name(name) {
                return T::from_bytes(&data);
            }
        }
        Err(anyhow::anyhow!(
            "File not found: {}",
            name.replace("\\", " \\ ")
        ))
    }

    pub fn file_names(&self) -> impl Iterator<Item = String> {
        self.archives.iter().flat_map(|a| a.file_names()).unique()
    }

    fn find_pak_files<P>(path: P) -> anyhow::Result<Vec<PathBuf>>
    where
        P: AsRef<Path>,
    {
        let pattern = format!("{}/**/*.pak", path.as_ref().display());
        let mut pak_files = glob::glob(&pattern)?
            .filter_map(|entry| entry.ok())
            .collect::<Vec<_>>();

        pak_files.sort_by(|a, b| b.file_name().unwrap().cmp(a.file_name().unwrap()));

        Ok(pak_files)
    }
}

impl Archive for Pak {
    fn by_name_bytes(&self, name: &str) -> anyhow::Result<Vec<u8>> {
        self.by_name::<Vec<u8>>(name)
    }

    fn file_names(&self) -> Box<dyn Iterator<Item = String> + '_> {
        Box::new(self.file_names())
    }
}

#[derive(Debug)]
struct PakArchive {
    path: PathBuf,
    entries: HashMap<String, (u64, u64)>,
}

impl PakArchive {
    fn new<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut reader = BufReader::new(File::open(path.as_ref())?);

        let mut ident = [0u8; 4];
        reader.read_exact(&mut ident)?;
        if ident != *b"PACK" {
            return Err(anyhow::anyhow!("Invalid PAK file"));
        }

        const DIRECTORY_ENTRY_SIZE: u64 = 0x40;

        let directory_offset = reader.read_u32::<LittleEndian>()? as u64;
        let directory_count = reader.read_u32::<LittleEndian>()? as u64 / DIRECTORY_ENTRY_SIZE;

        let entries = Self::read_directory_entries(&mut reader, directory_offset, directory_count)?;

        Ok(Self {
            path: path.as_ref().to_owned(),
            entries,
        })
    }

    fn read_directory_entries<R>(
        reader: &mut R,
        directory_offset: u64,
        directory_count: u64,
    ) -> anyhow::Result<HashMap<String, (u64, u64)>>
    where
        R: Read + Seek,
    {
        reader.seek(SeekFrom::Start(directory_offset))?;
        let mut entries = HashMap::with_capacity(directory_count as usize);

        for _ in 0..directory_count {
            const ENTRY_NAME_SIZE: usize = 0x38;

            let entry_name = read_null_terminated_string(reader, ENTRY_NAME_SIZE)?;
            let entry_offset = reader.read_u32::<LittleEndian>()? as u64;
            let entry_size = reader.read_u32::<LittleEndian>()? as u64;
            entries.insert(entry_name, (entry_offset, entry_size));
        }

        Ok(entries)
    }

    fn by_name(&self, name: &str) -> anyhow::Result<Box<[u8]>> {
        let (offset, size) = self.entries.get(name).ok_or(anyhow::anyhow!(
            "File not found: {}",
            name.replace("\\", " \\ ")
        ))?;
        let mut buffer = vec![0u8; *size as usize];

        let mut reader = BufReader::new(File::open(self.path.as_path())?);
        reader.seek(std::io::SeekFrom::Start(*offset))?;
        reader.read_exact(&mut buffer)?;

        Ok(buffer.into_boxed_slice())
    }

    fn file_names(&self) -> impl Iterator<Item = String> {
        self.entries.keys().cloned()
    }
}

#[derive(Copy, Clone, Debug)]
pub enum BoundingVolume {
    Sphere { center: Vec3, radius: f32 },
    Box { min: Vec3, max: Vec3 },
}

impl BoundingVolume {
    pub fn read_bounding_sphere_at_origin_with<R, F>(
        reader: &mut R,
        mut read_vector_fn: F,
    ) -> anyhow::Result<Self>
    where
        R: ReadBytesExt,
        F: FnMut(&mut R) -> anyhow::Result<f32>,
    {
        let radius = read_vector_fn(reader)?;

        Ok(Self::Sphere {
            center: Vec3::ZERO,
            radius,
        })
    }

    pub fn read_bounding_box_with<R, F>(
        reader: &mut R,
        mut read_vector_fn: F,
    ) -> anyhow::Result<Self>
    where
        R: ReadBytesExt,
        F: FnMut(&mut R) -> anyhow::Result<Vec3>,
    {
        let min = read_vector_fn(reader)?;
        let max = read_vector_fn(reader)?;

        Ok(Self::Box { min, max })
    }

    pub fn transform(&self, transform: &Mat4) -> Self {
        let mut transformed = *self;
        transformed.transform_mut(transform);
        transformed
    }

    pub fn transform_mut(&mut self, transform: &Mat4) {
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

    fn sphere_intersects_sphere(center1: Vec3, radius1: f32, center2: Vec3, radius2: f32) -> bool {
        let distance = (center1 - center2).length();
        distance <= radius1 + radius2
    }

    fn sphere_intersects_box(center: Vec3, radius: f32, min: Vec3, max: Vec3) -> bool {
        // Find the closest point on the box to the sphere center
        let closest_point = Vec3::new(
            center[0].clamp(min[0], max[0]),
            center[1].clamp(min[1], max[1]),
            center[2].clamp(min[2], max[2]),
        );

        // Calculate squared distance from sphere center to the closest point
        let distance = (center - closest_point).length();
        distance <= radius * radius
    }

    fn box_intersects_box(min1: Vec3, max1: Vec3, min2: Vec3, max2: Vec3) -> bool {
        // Two boxes intersect if they overlap on all three axes
        (min1[0] <= max2[0] && max1[0] >= min2[0])
            && (min1[1] <= max2[1] && max1[1] >= min2[1])
            && (min1[2] <= max2[2] && max1[2] >= min2[2])
    }

    fn interpolate_vec3<F>(v1: &Vec3, v2: &Vec3, t: f32, interpolate_fn: F) -> Vec3
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
    R: ReadBytesExt,
{
    BoundingVolume::read_bounding_sphere_at_origin_with(reader, |r| {
        Ok(r.read_f32::<LittleEndian>()?)
    })
}

pub fn read_i16_bounding_box<R>(reader: &mut R) -> anyhow::Result<BoundingVolume>
where
    R: ReadBytesExt,
{
    BoundingVolume::read_bounding_box_with(reader, |r| Ok(read_i16_vec3(r)?))
}

pub fn read_f32_bounding_box<R>(reader: &mut R) -> anyhow::Result<BoundingVolume>
where
    R: ReadBytesExt,
{
    BoundingVolume::read_bounding_box_with(reader, |r| Ok(read_vec3(r)?))
}

pub fn read_scaled_position_bounding_box<R>(
    reader: &mut R,
    scale: Vec3,
    translate: Vec3,
) -> anyhow::Result<BoundingVolume>
where
    R: ReadBytesExt,
{
    BoundingVolume::read_bounding_box_with(reader, |r| {
        let point = read_scaled_position::<R>(r, scale, translate)?;
        r.read_u8()? as f32;
        Ok(point)
    })
}

pub fn read_vec3<R>(reader: &mut R) -> anyhow::Result<Vec3>
where
    R: ReadBytesExt,
{
    let vector = [
        reader.read_f32::<LittleEndian>()?,
        reader.read_f32::<LittleEndian>()?,
        reader.read_f32::<LittleEndian>()?,
    ];

    // Swaps Y↔Z axes to convert from Quake's coordinate system to standard 3D
    Ok([vector[0], vector[2], -vector[1]].into())
}

pub fn read_i16_vec3<R>(reader: &mut R) -> anyhow::Result<Vec3>
where
    R: ReadBytesExt,
{
    let vector = [
        reader.read_i16::<LittleEndian>()? as f32,
        reader.read_i16::<LittleEndian>()? as f32,
        reader.read_i16::<LittleEndian>()? as f32,
    ];

    // Swaps Y↔Z axes to convert from Quake's coordinate system to standard 3D
    Ok([vector[0], vector[2], -vector[1]].into())
}

pub fn read_i8_vec3<R>(reader: &mut R) -> anyhow::Result<Vec3>
where
    R: ReadBytesExt,
{
    let vector = [
        reader.read_i8()? as f32,
        reader.read_i8()? as f32,
        reader.read_i8()? as f32,
    ];

    // Swaps Y↔Z axes to convert from Quake's coordinate system to standard 3D
    Ok([vector[0], vector[2], -vector[1]].into())
}

pub fn read_scaled_position<R>(reader: &mut R, scale: Vec3, translate: Vec3) -> anyhow::Result<Vec3>
where
    R: ReadBytesExt,
{
    Ok(read_i8_vec3(reader)? * scale + translate)
}

pub fn read_null_terminated_string<R>(reader: &mut R, buffer_size: usize) -> anyhow::Result<String>
where
    R: ReadBytesExt,
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
