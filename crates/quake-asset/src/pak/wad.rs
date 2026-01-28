use crate::FromBytes;
use crate::pak::read_null_terminated_string;
use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashMap;
use std::io::{Cursor, Read, Seek, SeekFrom};

impl FromBytes for Wad {
    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        Wad::from_slice(bytes)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum WadEntryType {
    Picture,
    MipTexture,
}

impl WadEntryType {
    fn from_i32(i: i32) -> anyhow::Result<Self> {
        match i {
            0x42 => Ok(Self::Picture),
            0x44 => Ok(Self::MipTexture),
            _ => Err(anyhow::anyhow!("Invalid WAD entry type: {:x}", i)),
        }
    }
}

#[derive(Clone, Debug)]
pub enum WadEntry {
    Picture {
        width: u32,
        height: u32,
        data: Box<[u8]>,
    },
    MipTexture(Box<[u8]>),
}

#[derive(Clone, Debug)]
pub struct Wad {
    data: Box<[u8]>,
    entries: HashMap<String, (u64, u64, WadEntryType)>,
}

impl Wad {
    pub fn from_slice(data: &[u8]) -> anyhow::Result<Self> {
        let mut reader = Cursor::new(data);

        let mut ident = [0u8; 4];
        reader.read_exact(&mut ident)?;
        if ident != *b"WAD2" {
            return Err(anyhow::anyhow!("Invalid WAD file"));
        }

        let directory_count = reader.read_u32::<LittleEndian>()? as u64;
        let directory_offset = reader.read_u32::<LittleEndian>()? as u64;

        let entries = Self::read_directory_entries(&mut reader, directory_offset, directory_count)?;

        Ok(Self {
            data: data.to_vec().into_boxed_slice(),
            entries,
        })
    }

    pub fn by_name(&self, name: &str) -> anyhow::Result<WadEntry> {
        let (entry_offset, entry_size, entry_type) = self.entries.get(name).ok_or(
            anyhow::anyhow!("File not found: {}", name.replace("\\", " \\ ")),
        )?;

        let mut reader = Cursor::new(&self.data);
        reader.seek(SeekFrom::Start(*entry_offset))?;

        let entry = match entry_type {
            WadEntryType::Picture => {
                let width = reader.read_u32::<LittleEndian>()?;
                let height = reader.read_u32::<LittleEndian>()?;
                let mut buffer = vec![0u8; (width * height) as usize];
                reader.read_exact(&mut buffer)?;
                WadEntry::Picture {
                    width,
                    height,
                    data: buffer.into_boxed_slice(),
                }
            }
            WadEntryType::MipTexture => {
                let mut buffer = vec![0u8; *entry_size as usize];
                reader.read_exact(&mut buffer)?;
                WadEntry::MipTexture(buffer.into_boxed_slice())
            }
        };

        Ok(entry)
    }

    pub fn file_names(&self) -> impl Iterator<Item = String> {
        self.entries.keys().cloned()
    }

    fn read_directory_entries<R>(
        reader: &mut R,
        directory_offset: u64,
        directory_count: u64,
    ) -> anyhow::Result<HashMap<String, (u64, u64, WadEntryType)>>
    where
        R: Read + Seek,
    {
        reader.seek(SeekFrom::Start(directory_offset))?;
        let mut entries = HashMap::with_capacity(directory_count as usize);

        for _ in 0..directory_count {
            const ENTRY_NAME_SIZE: usize = 0x10;

            let entry_offset = reader.read_u32::<LittleEndian>()? as u64;
            reader.read_u32::<LittleEndian>()?;

            let entry_size = reader.read_u32::<LittleEndian>()? as u64;
            let entry_type = WadEntryType::from_i32(reader.read_u8()? as i32)?;

            reader.read_u8()?;
            reader.read_u16::<LittleEndian>()?;

            let entry_name = read_null_terminated_string(reader, ENTRY_NAME_SIZE)?;

            entries.insert(entry_name, (entry_offset, entry_size, entry_type));
        }

        Ok(entries)
    }
}
