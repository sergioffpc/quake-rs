use byteorder::{LittleEndian, ReadBytesExt};

pub fn read_f32_bounding_sphere<R>(
    reader: &mut R,
) -> anyhow::Result<quake_collision::BoundingVolume>
where
    R: std::io::Read,
{
    quake_collision::BoundingVolume::read_bounding_sphere_at_origin_with(reader, |r| {
        Ok(r.read_f32::<LittleEndian>()?)
    })
}

pub fn read_scaled_position_bounding_box<R>(
    reader: &mut R,
    scale: glam::Vec3,
    translate: glam::Vec3,
) -> anyhow::Result<quake_collision::BoundingVolume>
where
    R: std::io::Read,
{
    quake_collision::BoundingVolume::read_bounding_box_with(reader, |r| {
        let point = read_scaled_position(r, scale, translate)?;
        r.read_u8()? as f32;

        Ok(point)
    })
}

impl quake_resource::FromBytes for Mdl {
    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        Mdl::from_slice(bytes)
    }
}

#[derive(Clone, Debug)]
pub struct Mdl {
    pub bounding_volume: quake_collision::BoundingVolume,
    pub eye_position: glam::Vec3,
    pub vertices_count: u32,
    pub texture_width: u32,
    pub texture_height: u32,
    pub textures: Box<[Texture]>,
    pub texture_coords: Box<[TextureCoord]>,
    pub triangles: Box<[Triangle]>,
    pub frames: Box<[Frame]>,
    pub sync_type: SyncType,
    pub flags: Flags,
}

impl Mdl {
    pub fn from_slice(data: &[u8]) -> anyhow::Result<Self> {
        use std::io::Read;
        let mut reader = std::io::Cursor::new(data);

        let mut ident = [0u8; 4];
        reader.read_exact(&mut ident)?;
        if ident != *b"IDPO" {
            return Err(anyhow::anyhow!("Invalid MDL file"));
        }

        let version = reader.read_u32::<LittleEndian>()?;
        if version != 6 {
            return Err(anyhow::anyhow!("Invalid MDL version"));
        }

        let scale = read_f32_vector3(&mut reader)?;
        let translate = read_f32_vector3(&mut reader)?;
        let bounding_volume = read_f32_bounding_sphere(&mut reader)?;

        let eye_position = read_f32_vector3(&mut reader)?;

        let textures_count = reader.read_u32::<LittleEndian>()?;
        let texture_width = reader.read_u32::<LittleEndian>()?;
        let texture_height = reader.read_u32::<LittleEndian>()?;

        let vertices_count = reader.read_u32::<LittleEndian>()?;
        let triangles_count = reader.read_u32::<LittleEndian>()?;
        let frames_count = reader.read_u32::<LittleEndian>()?;

        let sync_type = SyncType::from_i32(reader.read_i32::<LittleEndian>()?);
        let flags = Flags::from_i32(reader.read_i32::<LittleEndian>()?);
        let _size = reader.read_u32::<LittleEndian>()?;

        let textures =
            Self::read_textures(&mut reader, textures_count, texture_width, texture_height)?;
        let texture_coords = Self::read_texture_coords(&mut reader, vertices_count)?;
        let triangles = Self::read_triangles(&mut reader, triangles_count)?;
        let frames = Self::read_frames(
            &mut reader,
            frames_count,
            vertices_count,
            &triangles,
            scale,
            translate,
        )?;

        Ok(Self {
            bounding_volume,
            eye_position,
            vertices_count,
            texture_width,
            texture_height,
            textures,
            texture_coords,
            triangles,
            frames,
            sync_type,
            flags,
        })
    }

    fn read_textures<R>(
        reader: &mut R,
        textures_count: u32,
        texture_width: u32,
        texture_height: u32,
    ) -> anyhow::Result<Box<[Texture]>>
    where
        R: std::io::Read + std::io::Seek,
    {
        let mut textures = Vec::with_capacity(textures_count as usize);
        for _ in 0..textures_count {
            let texture = Texture::from_reader(reader, texture_width, texture_height)?;
            textures.push(texture);
        }

        Ok(textures.into_boxed_slice())
    }

    fn read_texture_coords<R>(
        reader: &mut R,
        vertices_count: u32,
    ) -> anyhow::Result<Box<[TextureCoord]>>
    where
        R: std::io::Read + std::io::Seek,
    {
        let mut texture_coords = Vec::with_capacity(vertices_count as usize);
        for _ in 0..vertices_count {
            let texture_coord = TextureCoord::from_reader(reader)?;
            texture_coords.push(texture_coord)
        }

        Ok(texture_coords.into_boxed_slice())
    }

    fn read_triangles<R>(reader: &mut R, triangles_count: u32) -> anyhow::Result<Box<[Triangle]>>
    where
        R: std::io::Read + std::io::Seek,
    {
        let mut triangles = Vec::with_capacity(triangles_count as usize);
        for _ in 0..triangles_count {
            let triangle = Triangle::from_reader(reader)?;
            triangles.push(triangle);
        }

        Ok(triangles.into_boxed_slice())
    }

    fn read_frames<R>(
        reader: &mut R,
        frames_count: u32,
        vertices_count: u32,
        triangles: &[Triangle],
        scale: glam::Vec3,
        translate: glam::Vec3,
    ) -> anyhow::Result<Box<[Frame]>>
    where
        R: std::io::Read + std::io::Seek,
    {
        let mut frames = Vec::with_capacity(frames_count as usize);
        for _ in 0..frames_count {
            let frame = Frame::from_reader(reader, vertices_count, triangles, scale, translate)?;
            frames.push(frame);
        }

        Ok(frames.into_boxed_slice())
    }
}

fn read_f32_vector3<R>(reader: &mut R) -> anyhow::Result<glam::Vec3>
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

fn read_scaled_position<R>(
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

fn read_null_terminated_string<R>(reader: &mut R, buffer_size: usize) -> anyhow::Result<String>
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

#[derive(Clone, Debug)]
pub struct TimedGroup<T> {
    pub times: Box<[f32]>,
    pub items: Box<[T]>,
}

#[derive(Clone, Debug)]
pub enum Texture {
    Single(TextureSingle),
    Group(TextureGroup),
}

impl Texture {
    pub fn from_reader<R>(
        reader: &mut R,
        texture_width: u32,
        texture_height: u32,
    ) -> anyhow::Result<Self>
    where
        R: std::io::Read + std::io::Seek,
    {
        let texture_type = reader.read_u32::<LittleEndian>()?;
        let texture = match texture_type {
            0 => Self::Single(TextureSingle::from_reader(
                reader,
                texture_width,
                texture_height,
            )?),
            _ => Self::Group(TextureGroup::from_reader(
                reader,
                texture_width,
                texture_height,
            )?),
        };
        Ok(texture)
    }
}

#[derive(Clone, Debug)]
pub struct TextureSingle {
    pub width: u32,
    pub height: u32,
    pub data: Box<[u8]>,
}

impl TextureSingle {
    pub fn from_reader<R>(
        reader: &mut R,
        texture_width: u32,
        texture_height: u32,
    ) -> anyhow::Result<Self>
    where
        R: std::io::Read + std::io::Seek,
    {
        let mut texture = vec![0u8; texture_width as usize * texture_height as usize];
        reader.read_exact(&mut texture)?;

        Ok(Self {
            width: texture_width,
            height: texture_height,
            data: texture.into_boxed_slice(),
        })
    }
}

pub type TextureGroup = TimedGroup<TextureSingle>;

impl TextureGroup {
    pub fn from_reader<R>(
        reader: &mut R,
        texture_width: u32,
        texture_height: u32,
    ) -> anyhow::Result<Self>
    where
        R: std::io::Read + std::io::Seek,
    {
        let texture_count = reader.read_u32::<LittleEndian>()?;

        let mut times = Vec::with_capacity(texture_count as usize);
        for _ in 0..texture_count {
            times.push(reader.read_f32::<LittleEndian>()?);
        }

        let mut textures = Vec::with_capacity(texture_count as usize);
        for _ in 0..texture_count {
            textures.push(TextureSingle::from_reader(
                reader,
                texture_width,
                texture_height,
            )?);
        }

        Ok(Self {
            times: times.into_boxed_slice(),
            items: textures.into_boxed_slice(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct TextureCoord {
    pub on_seam: bool,
    pub s: u32,
    pub t: u32,
}

impl TextureCoord {
    pub fn from_reader<R>(reader: &mut R) -> anyhow::Result<Self>
    where
        R: std::io::Read + std::io::Seek,
    {
        let on_seam = reader.read_u32::<LittleEndian>()? == 0x20;
        let s = reader.read_u32::<LittleEndian>()?;
        let t = reader.read_u32::<LittleEndian>()?;
        Ok(Self { on_seam, s, t })
    }
}

#[derive(Clone, Debug)]
pub struct Triangle {
    pub faces_front: bool,
    pub indices: glam::UVec3,
}

impl Triangle {
    pub fn from_reader<R>(reader: &mut R) -> anyhow::Result<Self>
    where
        R: std::io::Read + std::io::Seek,
    {
        let faces_front = reader.read_u32::<LittleEndian>()? == 1;

        let indices = [
            reader.read_u32::<LittleEndian>()?,
            reader.read_u32::<LittleEndian>()?,
            reader.read_u32::<LittleEndian>()?,
        ];

        // Reverses triangle winding to convert from right-handed to left-handed
        let indices = [indices[0], indices[2], indices[1]].into();

        Ok(Self {
            faces_front,
            indices,
        })
    }
}

#[derive(Clone, Debug)]
pub enum Frame {
    Single(FrameSingle),
    Group(FrameGroup),
}

impl Frame {
    pub fn from_reader<R>(
        reader: &mut R,
        vertices_count: u32,
        triangles: &[Triangle],
        scale: glam::Vec3,
        translate: glam::Vec3,
    ) -> anyhow::Result<Self>
    where
        R: std::io::Read + std::io::Seek,
    {
        let frame_type = reader.read_u32::<LittleEndian>()?;
        let frame = match frame_type {
            0 => Frame::Single(FrameSingle::from_reader(
                reader,
                vertices_count,
                triangles,
                scale,
                translate,
            )?),
            _ => Frame::Group(FrameGroup::from_reader(
                reader,
                vertices_count,
                triangles,
                scale,
                translate,
            )?),
        };
        Ok(frame)
    }
}

#[derive(Clone, Debug)]
pub struct FrameSingle {
    pub name: String,
    pub bounding_volume: quake_collision::BoundingVolume,
    pub positions: Box<[glam::Vec3]>,
    pub normals: Box<[glam::Vec3]>,
}

impl FrameSingle {
    pub fn from_reader<R>(
        reader: &mut R,
        vertices_count: u32,
        triangles: &[Triangle],
        scale: glam::Vec3,
        translate: glam::Vec3,
    ) -> anyhow::Result<Self>
    where
        R: std::io::Read + std::io::Seek,
    {
        const FRAME_NAME_SIZE: usize = 0x10;

        let bounding_volume = read_scaled_position_bounding_box(reader, scale, translate)?;
        let name = read_null_terminated_string(reader, FRAME_NAME_SIZE)?;

        let mut positions = Vec::with_capacity(vertices_count as usize);
        for _ in 0..vertices_count {
            let position = read_scaled_position(reader, scale, translate)?;
            let _normal_index = reader.read_u8()?;

            positions.push(position);
        }
        let positions = positions.into_boxed_slice();
        let normals = calculate_normals_for_buffers(
            &positions,
            triangles
                .iter()
                .flat_map(|t| [t.indices.x, t.indices.y, t.indices.z])
                .collect::<Vec<_>>()
                .as_slice(),
        )
        .into_boxed_slice();

        Ok(FrameSingle {
            name,
            bounding_volume,
            positions,
            normals,
        })
    }
}

#[derive(Clone, Debug)]
pub struct FrameGroup {
    pub bounding_volume: quake_collision::BoundingVolume,
    pub frames: TimedGroup<FrameSingle>,
}

impl FrameGroup {
    pub fn from_reader<R>(
        reader: &mut R,
        vertices_count: u32,
        triangles: &[Triangle],
        scale: glam::Vec3,
        translate: glam::Vec3,
    ) -> anyhow::Result<Self>
    where
        R: std::io::Read + std::io::Seek,
    {
        let subframes_count = reader.read_u32::<LittleEndian>()?;

        let bounding_volume = read_scaled_position_bounding_box(reader, scale, translate)?;

        let mut times = Vec::with_capacity(subframes_count as usize);
        for _ in 0..subframes_count {
            times.push(reader.read_f32::<LittleEndian>()?);
        }

        let mut subframes = Vec::with_capacity(subframes_count as usize);
        for _ in 0..subframes_count {
            subframes.push(FrameSingle::from_reader(
                reader,
                vertices_count,
                triangles,
                scale,
                translate,
            )?);
        }

        Ok(FrameGroup {
            bounding_volume,
            frames: TimedGroup {
                times: times.into_boxed_slice(),
                items: subframes.into_boxed_slice(),
            },
        })
    }

    pub fn bounding_volume(&self) -> &quake_collision::BoundingVolume {
        &self.bounding_volume
    }

    pub fn times(&self) -> &[f32] {
        &self.frames.times
    }

    pub fn frames(&self) -> &[FrameSingle] {
        &self.frames.items
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum SyncType {
    Sync = 0, // All entities animate in sync
    Rand = 1, // Entities start animation at random offset
}

impl SyncType {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => SyncType::Rand,
            _ => SyncType::Sync,
        }
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Flags: i32 {
        const EF_ROCKET   = 1 << 0;  // Rocket trail
        const EF_GRENADE  = 1 << 1;  // Grenade trail
        const EF_GIB      = 1 << 2;  // Blood trail (gibs)
        const EF_ROTATE   = 1 << 3;  // Rotate (for pickups)
        const EF_TRACER   = 1 << 4;  // Yellow tracer
        const EF_ZOMGIB   = 1 << 5;  // Glowing gib
        const EF_TRACER2  = 1 << 6;  // Orange tracer
        const EF_TRACER3  = 1 << 7;  // Green tracer
    }
}

impl Flags {
    pub fn from_i32(v: i32) -> Self {
        Flags::from_bits_truncate(v)
    }
}

fn calculate_normals_for_buffers(positions: &[glam::Vec3], indices: &[u32]) -> Vec<glam::Vec3> {
    let mut normals = vec![glam::Vec3::ZERO; positions.len()];

    for idx in indices.chunks_exact(3) {
        let (idx0, idx1, idx2) = match *idx {
            [idx0, idx1, idx2] => (idx0, idx1, idx2),
            _ => unsafe { std::hint::unreachable_unchecked() },
        };

        let pos1 = unsafe { *positions.get_unchecked(idx0 as usize) };
        let pos2 = unsafe { *positions.get_unchecked(idx1 as usize) };
        let pos3 = unsafe { *positions.get_unchecked(idx2 as usize) };

        let edge1 = pos2 - pos1;
        let edge2 = pos3 - pos1;

        let normal = edge1.cross(edge2);

        unsafe { *normals.get_unchecked_mut(idx0 as usize) += normal };
        unsafe { *normals.get_unchecked_mut(idx1 as usize) += normal };
        unsafe { *normals.get_unchecked_mut(idx2 as usize) += normal };
    }

    for normal in normals.iter_mut() {
        *normal = normal.normalize_or_zero();
    }

    normals
}
