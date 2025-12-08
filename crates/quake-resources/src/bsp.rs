use crate::bsp::MipTexture::BspTexture;
use crate::{
    BoundingVolume, read_f32_bounding_box, read_i16_bounding_box, read_null_terminated_string,
};
use nom::Parser;
use nom::bytes::complete::take_while;
use nom::character::complete::{char, multispace0};
use nom::combinator::map;
use nom::multi::many0;
use nom::sequence::delimited;
use std::collections::HashMap;
use std::io::Cursor;
use std::pin::Pin;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LumpType {
    Entities = 0,
    Planes = 1,
    Textures = 2,
    Vertexes = 3,
    Visibility = 4,
    Nodes = 5,
    TexInfo = 6,
    Faces = 7,
    Lightmap = 8,
    ClipNodes = 9,
    Leaves = 10,
    MarkSurfaces = 11,
    Edges = 12,
    SurfEdges = 13,
    BrushModels = 14,
}

#[derive(Copy, Clone, Debug)]
pub struct Lump {
    offset: u32,
    size: u32,
}

impl Lump {
    pub async fn from_reader<R>(mut reader: R) -> anyhow::Result<Self>
    where
        R: AsyncReadExt + AsyncSeekExt + Unpin + Send,
    {
        let offset = reader.read_u32_le().await?;
        let size = reader.read_u32_le().await?;
        Ok(Self { offset, size })
    }

    pub fn offset(&self) -> u32 {
        self.offset
    }

    pub fn size(&self) -> u32 {
        self.size
    }
}

#[derive(Clone, Debug)]
pub struct Bsp {
    pub entities: slotmap::SlotMap<EntityKey, Entity>,
    pub planes: Box<[Plane]>,
    pub textures: Box<[MipTexture]>,
    pub vertexes: Box<[glam::Vec3]>,
    pub visibility: Box<[u8]>,
    pub nodes: Box<[Node]>,
    pub texture_info: Box<[TextureInfo]>,
    pub faces: Box<[Face]>,
    pub lightmap: Box<[u8]>,
    pub clip_nodes: Box<[ClipNode]>,
    pub leaves: Box<[Leaf]>,
    pub mark_surfaces: Box<[usize]>,
    pub edges: Box<[Edge]>,
    pub surface_edges: Box<[SurfaceEdge]>,
    pub brush_models: Box<[BrushModel]>,
}

impl Bsp {
    pub async fn from_slice(data: &[u8]) -> anyhow::Result<Self> {
        let mut reader = Cursor::new(data);

        let version = reader.read_u32_le().await?;
        if version != 0x1D {
            return Err(anyhow::anyhow!("Invalid BSP version"));
        }

        let mut lumps = [Lump { offset: 0, size: 0 }; 15];
        for i in 0..lumps.len() {
            lumps[i] = Lump::from_reader(&mut reader).await?;
        }

        Self::read_bsp_data(&mut reader, &lumps).await
    }

    async fn read_bsp_data<R>(mut reader: R, lumps: &[Lump; 15]) -> anyhow::Result<Self>
    where
        R: AsyncReadExt + AsyncSeekExt + Unpin + Send,
    {
        let entities = Self::read_entities(&mut reader, lumps[LumpType::Entities as usize]).await?;
        let planes = Self::read_planes(&mut reader, lumps[LumpType::Planes as usize]).await?;
        let textures = Self::read_textures(&mut reader, lumps[LumpType::Textures as usize]).await?;
        let vertexes = Self::read_vertexes(&mut reader, lumps[LumpType::Vertexes as usize]).await?;
        let visibility =
            Self::read_visibility(&mut reader, lumps[LumpType::Visibility as usize]).await?;
        let nodes = Self::read_nodes(&mut reader, lumps[LumpType::Nodes as usize]).await?;
        let texture_info =
            Self::read_textures_info(&mut reader, lumps[LumpType::TexInfo as usize]).await?;
        let faces = Self::read_faces(&mut reader, lumps[LumpType::Faces as usize]).await?;
        let lightmap = Self::read_lightmap(&mut reader, lumps[LumpType::Lightmap as usize]).await?;
        let clip_nodes =
            Self::read_clip_nodes(&mut reader, lumps[LumpType::ClipNodes as usize]).await?;
        let leaves = Self::read_leaves(&mut reader, lumps[LumpType::Leaves as usize]).await?;
        let mark_surfaces =
            Self::read_mark_surfaces(&mut reader, lumps[LumpType::MarkSurfaces as usize]).await?;
        let edges = Self::read_edges(&mut reader, lumps[LumpType::Edges as usize]).await?;
        let surface_edges =
            Self::read_surface_edges(&mut reader, lumps[LumpType::SurfEdges as usize]).await?;
        let brush_models =
            Self::read_brush_models(&mut reader, lumps[LumpType::BrushModels as usize]).await?;

        Ok(Self {
            entities,
            planes,
            textures,
            vertexes,
            visibility,
            nodes,
            texture_info,
            faces,
            lightmap,
            clip_nodes,
            leaves,
            mark_surfaces,
            edges,
            surface_edges,
            brush_models,
        })
    }

    async fn read_entities<R>(
        mut reader: R,
        lump: Lump,
    ) -> anyhow::Result<slotmap::SlotMap<EntityKey, Entity>>
    where
        R: AsyncReadExt + AsyncSeekExt + Unpin + Send,
    {
        let content = Self::read_lump_as_string(&mut reader, lump).await?;
        let (_, entities) = Self::parse_entities(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse entities: {}", e))?;
        Ok(entities)
    }

    async fn read_lump_as_string<R>(reader: &mut R, lump: Lump) -> anyhow::Result<String>
    where
        R: AsyncReadExt + AsyncSeekExt + Unpin + Send,
    {
        reader
            .seek(std::io::SeekFrom::Start(lump.offset as u64))
            .await?;
        let mut buffer = vec![0u8; lump.size as usize];
        reader.read_exact(&mut buffer).await?;
        String::from_utf8(buffer).map_err(|e| anyhow::anyhow!("Invalid UTF-8 in entities: {}", e))
    }

    fn parse_entities(input: &str) -> nom::IResult<&str, slotmap::SlotMap<EntityKey, Entity>> {
        map(
            delimited(
                multispace0,
                many0(delimited(
                    (multispace0, char('{')),
                    Entity::from_parser,
                    (multispace0, char('}'), multispace0),
                )),
                multispace0,
            ),
            Self::build_entity_slot_map,
        )
        .parse(input)
    }

    fn build_entity_slot_map(entities: Vec<Entity>) -> slotmap::SlotMap<EntityKey, Entity> {
        entities
            .into_iter()
            .filter(|entity| !entity.components.is_empty() || !entity.classname.is_empty())
            .fold(slotmap::SlotMap::with_key(), |mut slot_map, entity| {
                slot_map.insert(entity);
                slot_map
            })
    }

    async fn read_planes<R>(mut reader: R, lump: Lump) -> anyhow::Result<Box<[Plane]>>
    where
        R: AsyncReadExt + AsyncSeekExt + Unpin + Send,
    {
        const PLANE_ENTRY_SIZE: usize = 0x14;
        reader
            .seek(std::io::SeekFrom::Start(lump.offset as u64))
            .await?;
        let planes_count = lump.size as usize / PLANE_ENTRY_SIZE;
        let mut planes = Vec::with_capacity(planes_count);
        for _ in 0..planes_count {
            let plane = Plane::from_reader(&mut reader).await?;
            planes.push(plane);
        }
        Ok(planes.into_boxed_slice())
    }

    async fn read_textures<R>(mut reader: R, lump: Lump) -> anyhow::Result<Box<[MipTexture]>>
    where
        R: AsyncReadExt + AsyncSeekExt + Unpin + Send,
    {
        reader
            .seek(std::io::SeekFrom::Start(lump.offset as u64))
            .await?;
        let textures_count = reader.read_u32_le().await?;
        let mut textures_offsets = Vec::with_capacity(textures_count as usize);
        for _ in 0..textures_count {
            textures_offsets.push(reader.read_u32_le().await?);
        }
        let mut textures = Vec::with_capacity(textures_count as usize);
        for texture_offset in textures_offsets {
            let texture = if texture_offset == u32::MAX {
                MipTexture::WadTexture
            } else {
                MipTexture::from_reader(&mut reader, lump.offset + texture_offset).await?
            };

            textures.push(texture);
        }
        Ok(textures.into_boxed_slice())
    }

    async fn read_vertexes<R>(mut reader: R, lump: Lump) -> anyhow::Result<Box<[glam::Vec3]>>
    where
        R: AsyncReadExt + AsyncSeekExt + Unpin + Send,
    {
        const VERTEX_ENTRY_SIZE: usize = 0xC;
        reader
            .seek(std::io::SeekFrom::Start(lump.offset as u64))
            .await?;
        let vertex_count = lump.size as usize / VERTEX_ENTRY_SIZE;
        let mut vertexes = Vec::with_capacity(vertex_count);
        for _ in 0..vertex_count {
            let position = read_f32_vector3(&mut reader).await?;
            vertexes.push(position);
        }
        Ok(vertexes.into_boxed_slice())
    }

    async fn read_visibility<R>(reader: R, lump: Lump) -> anyhow::Result<Box<[u8]>>
    where
        R: AsyncReadExt + AsyncSeekExt + Unpin + Send,
    {
        Self::read_lump_as_bytes(reader, lump).await
    }

    async fn read_nodes<R>(mut reader: R, lump: Lump) -> anyhow::Result<Box<[Node]>>
    where
        R: AsyncReadExt + AsyncSeekExt + Unpin + Send,
    {
        const NODE_ENTRY_SIZE: usize = 0x18;
        reader
            .seek(std::io::SeekFrom::Start(lump.offset as u64))
            .await?;
        let nodes_count = lump.size as usize / NODE_ENTRY_SIZE;
        let mut nodes = Vec::with_capacity(nodes_count);
        for _ in 0..nodes_count {
            let node = Node::from_reader(&mut reader).await?;
            nodes.push(node);
        }
        Ok(nodes.into_boxed_slice())
    }

    async fn read_textures_info<R>(mut reader: R, lump: Lump) -> anyhow::Result<Box<[TextureInfo]>>
    where
        R: AsyncReadExt + AsyncSeekExt + Unpin + Send,
    {
        const TEX_INFO_ENTRY_SIZE: usize = 0x28;
        reader
            .seek(std::io::SeekFrom::Start(lump.offset as u64))
            .await?;
        let textures_info_count = lump.size as usize / TEX_INFO_ENTRY_SIZE;
        let mut textures_info = Vec::with_capacity(textures_info_count);
        for _ in 0..textures_info_count {
            let texture_info = TextureInfo::from_reader(&mut reader).await?;
            textures_info.push(texture_info)
        }
        Ok(textures_info.into_boxed_slice())
    }

    async fn read_faces<R>(mut reader: R, lump: Lump) -> anyhow::Result<Box<[Face]>>
    where
        R: AsyncReadExt + AsyncSeekExt + Unpin + Send,
    {
        const FACE_ENTRY_SIZE: usize = 0x14;
        reader
            .seek(std::io::SeekFrom::Start(lump.offset as u64))
            .await?;
        let faces_count = lump.size as usize / FACE_ENTRY_SIZE;
        let mut faces = Vec::with_capacity(faces_count);
        for _ in 0..faces_count {
            let face = Face::from_reader(&mut reader).await?;
            faces.push(face);
        }
        Ok(faces.into_boxed_slice())
    }

    async fn read_lightmap<R>(reader: R, lump: Lump) -> anyhow::Result<Box<[u8]>>
    where
        R: AsyncReadExt + AsyncSeekExt + Unpin + Send,
    {
        Self::read_lump_as_bytes(reader, lump).await
    }

    async fn read_clip_nodes<R>(mut reader: R, lump: Lump) -> anyhow::Result<Box<[ClipNode]>>
    where
        R: AsyncReadExt + AsyncSeekExt + Unpin + Send,
    {
        const CLIP_NODE_ENTRY_SIZE: usize = 0x8;
        reader
            .seek(std::io::SeekFrom::Start(lump.offset as u64))
            .await?;
        let clip_nodes_count = lump.size as usize / CLIP_NODE_ENTRY_SIZE;
        let mut clip_nodes = Vec::with_capacity(clip_nodes_count);
        for _ in 0..clip_nodes_count {
            let clip_node = ClipNode::from_reader(&mut reader).await?;
            clip_nodes.push(clip_node);
        }
        Ok(clip_nodes.into_boxed_slice())
    }

    async fn read_leaves<R>(mut reader: R, lump: Lump) -> anyhow::Result<Box<[Leaf]>>
    where
        R: AsyncReadExt + AsyncSeekExt + Unpin + Send,
    {
        const LEAF_ENTRY_SIZE: usize = 0x1C;
        reader
            .seek(std::io::SeekFrom::Start(lump.offset as u64))
            .await?;
        let leaves_count = lump.size as usize / LEAF_ENTRY_SIZE;
        let mut leaves = Vec::with_capacity(leaves_count);
        for _ in 0..leaves_count {
            let leaf = Leaf::from_reader(&mut reader).await?;
            leaves.push(leaf);
        }
        Ok(leaves.into_boxed_slice())
    }

    async fn read_mark_surfaces<R>(reader: R, lump: Lump) -> anyhow::Result<Box<[usize]>>
    where
        R: AsyncReadExt + AsyncSeekExt + Unpin + Send,
    {
        const MARK_SURFACE_ENTRY_SIZE: usize = 0x2;
        Self::read_primitive_array(reader, lump, MARK_SURFACE_ENTRY_SIZE, |r| {
            Box::pin(async move { Ok(r.read_u16_le().await? as usize) })
        })
        .await
    }

    async fn read_edges<R>(reader: R, lump: Lump) -> anyhow::Result<Box<[Edge]>>
    where
        R: AsyncReadExt + AsyncSeekExt + Unpin + Send,
    {
        const EDGE_ENTRY_SIZE: usize = 0x4;
        Self::read_primitive_array(reader, lump, EDGE_ENTRY_SIZE, |r| {
            Box::pin(async move { Edge::from_reader(r).await })
        })
        .await
    }

    async fn read_surface_edges<R>(reader: R, lump: Lump) -> anyhow::Result<Box<[SurfaceEdge]>>
    where
        R: AsyncReadExt + AsyncSeekExt + Unpin + Send,
    {
        const SURFACE_EDGE_ENTRY_SIZE: usize = 0x4;
        Self::read_primitive_array(reader, lump, SURFACE_EDGE_ENTRY_SIZE, |r| {
            Box::pin(async move { SurfaceEdge::from_reader(r).await })
        })
        .await
    }

    async fn read_brush_models<R>(reader: R, lump: Lump) -> anyhow::Result<Box<[BrushModel]>>
    where
        R: AsyncReadExt + AsyncSeekExt + Unpin + Send,
    {
        const BRUSH_MODEL_ENTRY_SIZE: usize = 0x40;
        Self::read_primitive_array(reader, lump, BRUSH_MODEL_ENTRY_SIZE, |r| {
            Box::pin(async move { BrushModel::from_reader(r).await })
        })
        .await
    }

    async fn read_lump_as_bytes<R>(mut reader: R, lump: Lump) -> anyhow::Result<Box<[u8]>>
    where
        R: AsyncReadExt + AsyncSeekExt + Unpin + Send,
    {
        reader
            .seek(std::io::SeekFrom::Start(lump.offset as u64))
            .await?;
        let mut buffer = vec![0u8; lump.size as usize];
        reader.read_exact(&mut buffer).await?;
        Ok(buffer.into_boxed_slice())
    }

    async fn read_primitive_array<R, T, F>(
        mut reader: R,
        lump: Lump,
        entry_size: usize,
        mut read_fn: F,
    ) -> anyhow::Result<Box<[T]>>
    where
        R: AsyncReadExt + AsyncSeekExt + Unpin + Send,
        F: for<'r> FnMut(&'r mut R) -> Pin<Box<dyn Future<Output = anyhow::Result<T>> + Send + 'r>>,
    {
        reader
            .seek(std::io::SeekFrom::Start(lump.offset as u64))
            .await?;
        let count = lump.size as usize / entry_size;
        let mut items = Vec::with_capacity(count);
        for _ in 0..count {
            items.push(read_fn(&mut reader).await?);
        }
        Ok(items.into_boxed_slice())
    }
}

#[async_trait::async_trait]
impl quake_traits::FromBytes for Bsp {
    async fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        Bsp::from_slice(bytes).await
    }
}

async fn read_f32_vector3<R>(reader: &mut R) -> anyhow::Result<glam::Vec3>
where
    R: AsyncReadExt + Unpin + Send,
{
    Ok([
        reader.read_f32_le().await?,
        reader.read_f32_le().await?,
        reader.read_f32_le().await?,
    ]
    .into())
}

#[derive(Clone, Debug)]
pub struct Entity {
    classname: String,
    components: HashMap<String, String>,
}

impl Entity {
    pub fn from_parser(input: &str) -> nom::IResult<&str, Self> {
        map(
            many0(delimited(
                multispace0,
                Self::parse_key_value_pair,
                multispace0,
            )),
            Self::build_entity_from_pairs,
        )
        .parse(input)
    }

    fn build_entity_from_pairs(pairs: Vec<(&str, &str)>) -> Entity {
        let (classname, components) = pairs.into_iter().fold(
            (String::new(), HashMap::new()),
            |(mut classname, mut components), (key, value)| {
                if key == "classname" {
                    classname = value.to_string();
                } else {
                    components.insert(key.to_string(), value.to_string());
                }
                (classname, components)
            },
        );

        Entity {
            classname,
            components,
        }
    }

    fn parse_key_value_pair(input: &str) -> nom::IResult<&str, (&str, &str)> {
        map(
            (
                Self::parse_quoted_string,
                multispace0,
                Self::parse_quoted_string,
            ),
            |(key, _, value)| (key, value),
        )
        .parse(input)
    }

    fn parse_quoted_string(input: &str) -> nom::IResult<&str, &str> {
        delimited(char('"'), take_while(|c| c != '"'), char('"')).parse(input)
    }
}

slotmap::new_key_type! {
    pub struct EntityKey;
}

#[derive(Clone, Debug)]
pub struct Plane {
    normal: glam::Vec3,
    distance: f32,
}

impl Plane {
    pub async fn from_reader<R>(reader: &mut R) -> anyhow::Result<Self>
    where
        R: AsyncReadExt + Unpin + Send,
    {
        let normal = read_f32_vector3(reader).await?;
        let distance = reader.read_f32_le().await?;
        let _plane_type = reader.read_u32_le().await?;

        Ok(Self { normal, distance })
    }
}

#[derive(Clone, Debug)]
pub enum MipTexture {
    WadTexture,
    BspTexture {
        name: String,
        textures: Box<[Texture]>,
    },
}

impl MipTexture {
    pub async fn from_reader<R>(mut reader: R, offset: u32) -> anyhow::Result<Self>
    where
        R: AsyncReadExt + AsyncSeekExt + Unpin + Send,
    {
        const TEXTURE_NAME_SIZE: usize = 0x10;
        const MIPMAP_LEVEL_COUNT: usize = 4;

        reader.seek(std::io::SeekFrom::Start(offset as u64)).await?;

        let name = read_null_terminated_string(&mut reader, TEXTURE_NAME_SIZE).await?;

        let base_width = reader.read_u32_le().await?;
        let base_height = reader.read_u32_le().await?;
        let mut mipmap_offsets = Vec::with_capacity(MIPMAP_LEVEL_COUNT);
        for _ in 0..MIPMAP_LEVEL_COUNT {
            mipmap_offsets.push(reader.read_u32_le().await?);
        }

        let mut textures = Vec::with_capacity(MIPMAP_LEVEL_COUNT);
        for i in 0..MIPMAP_LEVEL_COUNT {
            let (mip_width, mip_height) =
                Self::calculate_mipmap_dimensions(base_width, base_height, i);
            let data_size = mip_width as usize * mip_height as usize;
            let mut data = vec![0u8; data_size];

            reader
                .seek(std::io::SeekFrom::Start(
                    (offset + mipmap_offsets[i]) as u64,
                ))
                .await?;
            reader.read_exact(&mut data).await?;

            textures.push(Texture {
                width: mip_width,
                height: mip_height,
                data: data.into_boxed_slice(),
            });
        }

        Ok(BspTexture {
            name,
            textures: textures.into_boxed_slice(),
        })
    }

    fn calculate_mipmap_dimensions(
        base_width: u32,
        base_height: u32,
        mip_level: usize,
    ) -> (u32, u32) {
        let scale_factor = 1 << mip_level;
        (base_width / scale_factor, base_height / scale_factor)
    }
}

#[derive(Clone, Debug)]
pub struct Texture {
    width: u32,
    height: u32,
    data: Box<[u8]>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NodeChildType {
    Leaf(usize),
    Branch(usize),
}

impl NodeChildType {
    pub async fn from_reader<R>(reader: &mut R) -> anyhow::Result<Self>
    where
        R: AsyncReadExt + Unpin + Send,
    {
        let value = reader.read_i32_le().await?;
        let child_type = if value < 0 {
            Self::Leaf(!value as usize)
        } else {
            Self::Branch(value as usize)
        };

        Ok(child_type)
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    plane_index: usize,
    children: [NodeChildType; 2],
    bounding_volume: BoundingVolume,
    face_index: usize,
    faces_count: u32,
}

impl Node {
    pub async fn from_reader<R>(reader: &mut R) -> anyhow::Result<Self>
    where
        R: AsyncReadExt + Unpin + Send,
    {
        let plane_index = reader.read_u32_le().await? as usize;
        let children = Self::read_node_children(reader).await?;
        let bounding_volume = read_i16_bounding_box(reader).await?;
        let face_index = reader.read_u16_le().await? as usize;
        let faces_count = reader.read_u16_le().await? as u32;

        Ok(Self {
            plane_index,
            children,
            bounding_volume,
            face_index,
            faces_count,
        })
    }

    async fn read_node_children<R>(reader: &mut R) -> anyhow::Result<[NodeChildType; 2]>
    where
        R: AsyncReadExt + Unpin + Send,
    {
        let children = [
            NodeChildType::from_reader(reader).await?,
            NodeChildType::from_reader(reader).await?,
        ];

        Ok(children)
    }
}

#[derive(Clone, Debug)]
pub struct TextureInfo {
    s_vector: glam::Vec3,
    s_offset: f32,
    t_vector: glam::Vec3,
    t_offset: f32,
    texture_index: usize,
    flags: u32,
}

impl TextureInfo {
    pub async fn from_reader<R>(reader: &mut R) -> anyhow::Result<Self>
    where
        R: AsyncReadExt + Unpin + Send,
    {
        let s_vector = read_f32_vector3(reader).await?;
        let s_offset = reader.read_f32_le().await?;

        let t_vector = read_f32_vector3(reader).await?;
        let t_offset = reader.read_f32_le().await?;

        let texture_index = reader.read_u32_le().await? as usize;
        let flags = reader.read_u32_le().await?;

        Ok(Self {
            s_vector,
            s_offset,
            t_vector,
            t_offset,
            texture_index,
            flags,
        })
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FaceType {
    Front,
    Back,
}

impl FaceType {
    pub async fn from_reader<R>(reader: &mut R) -> anyhow::Result<Self>
    where
        R: AsyncReadExt + Unpin + Send,
    {
        let side_value = reader.read_u16_le().await?;
        Ok(Self::from_i32(side_value as i32))
    }

    pub fn from_i32(i: i32) -> Self {
        match i {
            0 => FaceType::Front,
            _ => FaceType::Back,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Face {
    plane_index: usize,
    side: FaceType,
    edge_index: usize,
    edge_count: u32,
    tex_info_index: usize,
    light_styles: [u8; 4],
    light_offset: i32,
}

impl Face {
    pub async fn from_reader<R>(reader: &mut R) -> anyhow::Result<Self>
    where
        R: AsyncReadExt + Unpin + Send,
    {
        let plane_index = reader.read_u16_le().await? as usize;
        let side = FaceType::from_reader(reader).await?;
        let edge_index = reader.read_u32_le().await? as usize;
        let edge_count = reader.read_u16_le().await? as u32;
        let tex_info_index = reader.read_u16_le().await? as usize;
        let light_styles = Self::read_light_styles(reader).await?;
        let light_offset = reader.read_i32_le().await?;

        Ok(Face {
            plane_index,
            side,
            edge_index,
            edge_count,
            tex_info_index,
            light_styles,
            light_offset,
        })
    }

    async fn read_light_styles<R>(reader: &mut R) -> anyhow::Result<[u8; 4]>
    where
        R: AsyncReadExt + Unpin + Send,
    {
        let mut light_styles = [0u8; 4];
        reader.read_exact(&mut light_styles).await?;
        Ok(light_styles)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ContentType {
    Empty,
    Solid,
    Water,
    Slime,
    Lava,
    Sky,
}

impl ContentType {
    pub fn from_i32(i: i32) -> anyhow::Result<Self> {
        match i {
            -1 => Ok(ContentType::Empty),
            -2 => Ok(ContentType::Solid),
            -3 => Ok(ContentType::Water),
            -4 => Ok(ContentType::Slime),
            -5 => Ok(ContentType::Lava),
            -6 => Ok(ContentType::Sky),
            _ => Err(anyhow::anyhow!("Invalid content type: {:x}", i)),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ClipNodeChildType {
    Contents(ContentType),
    Branch(usize),
}

impl ClipNodeChildType {
    pub async fn from_reader<R>(reader: &mut R) -> anyhow::Result<Self>
    where
        R: AsyncReadExt + Unpin + Send,
    {
        match reader.read_i16_le().await? {
            i if i < 0 => Ok(ClipNodeChildType::Contents(ContentType::from_i32(
                i as i32,
            )?)),
            i => Ok(ClipNodeChildType::Branch(i as usize)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ClipNode {
    plane_index: usize,
    children: [ClipNodeChildType; 2],
}

impl ClipNode {
    pub async fn from_reader<R>(reader: &mut R) -> anyhow::Result<Self>
    where
        R: AsyncReadExt + Unpin + Send,
    {
        let plane_index = reader.read_u32_le().await? as usize;
        let children = [
            ClipNodeChildType::from_reader(reader).await?,
            ClipNodeChildType::from_reader(reader).await?,
        ];

        Ok(Self {
            plane_index,
            children,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Leaf {
    contents: ContentType,
    visibility_index: Option<usize>,
    bounding_volume: BoundingVolume,
    mark_surface_index: usize,
    mark_surfaces_count: u32,
    ambient_sounds: [u8; 4],
}

impl Leaf {
    pub async fn from_reader<R>(reader: &mut R) -> anyhow::Result<Self>
    where
        R: AsyncReadExt + Unpin + Send,
    {
        let contents = ContentType::from_i32(reader.read_i32_le().await?)?;
        let visibility_index = Self::read_visibility_index(reader).await?;
        let bounding_volume = read_i16_bounding_box(reader).await?;
        let mark_surface_index = reader.read_u16_le().await? as usize;
        let mark_surfaces_count = reader.read_u16_le().await? as u32;
        let ambient_sounds = Self::read_ambient_sounds(reader).await?;

        Ok(Self {
            contents,
            visibility_index,
            bounding_volume,
            mark_surface_index,
            mark_surfaces_count,
            ambient_sounds,
        })
    }

    async fn read_visibility_index<R>(reader: &mut R) -> anyhow::Result<Option<usize>>
    where
        R: AsyncReadExt + Unpin + Send,
    {
        match reader.read_i32_le().await? {
            i if i >= 0 => Ok(Some(i as usize)),
            _ => Ok(None),
        }
    }

    async fn read_ambient_sounds<R>(reader: &mut R) -> anyhow::Result<[u8; 4]>
    where
        R: AsyncReadExt + Unpin + Send,
    {
        let mut ambient_sounds = [0u8; 4];
        reader.read_exact(&mut ambient_sounds).await?;
        Ok(ambient_sounds)
    }
}

#[derive(Clone, Debug)]
pub struct Edge {
    indices: [u32; 2],
}

impl Edge {
    pub async fn from_reader<R>(reader: &mut R) -> anyhow::Result<Self>
    where
        R: AsyncReadExt + Unpin + Send,
    {
        let indices = [
            reader.read_u16_le().await? as u32,
            reader.read_u16_le().await? as u32,
        ];

        Ok(Self { indices })
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SurfaceEdge {
    Forward(usize),
    Backward(usize),
}

impl SurfaceEdge {
    pub async fn from_reader<R>(reader: &mut R) -> anyhow::Result<Self>
    where
        R: AsyncReadExt + Unpin + Send,
    {
        let surface_edge = match reader.read_i32_le().await? {
            i if i >= 0 => SurfaceEdge::Forward(i as usize),
            i => SurfaceEdge::Backward(-i as usize),
        };

        Ok(surface_edge)
    }
}

#[derive(Clone, Debug)]
pub struct BrushModel {
    bounding_volume: BoundingVolume,
    origin: glam::Vec3,
    hull_indices: [usize; 4],
    leaves_count: usize,
    face_index: usize,
    faces_count: u32,
}

impl BrushModel {
    pub async fn from_reader<R>(reader: &mut R) -> anyhow::Result<Self>
    where
        R: AsyncReadExt + Unpin + Send,
    {
        let bounding_volume = read_f32_bounding_box(reader).await?;
        let origin = read_f32_vector3(reader).await?;
        let hull_indices = Self::read_hull_indices(reader).await?;
        let leaves_count = reader.read_u32_le().await? as usize;
        let face_index = reader.read_u32_le().await? as usize;
        let faces_count = reader.read_u32_le().await?;

        Ok(Self {
            bounding_volume,
            origin,
            hull_indices,
            leaves_count,
            face_index,
            faces_count,
        })
    }

    async fn read_hull_indices<R>(reader: &mut R) -> anyhow::Result<[usize; 4]>
    where
        R: AsyncReadExt + Unpin + Send,
    {
        Ok([
            reader.read_u32_le().await? as usize,
            reader.read_u32_le().await? as usize,
            reader.read_u32_le().await? as usize,
            reader.read_u32_le().await? as usize,
        ])
    }
}
