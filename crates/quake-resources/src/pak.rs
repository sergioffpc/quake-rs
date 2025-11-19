use crate::read_null_terminated_string;
use byteorder::{LittleEndian, ReadBytesExt};
use itertools::Itertools;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek};

#[derive(Debug)]
pub struct Pak {
    archives: Box<[PakArchive]>,
}

impl Pak {
    pub fn new<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<std::path::Path>,
    {
        let files = Self::find_pak_files(path)?;
        let archives = files
            .iter()
            .map(|f| PakArchive::new(f.as_path()))
            .collect::<Result<_, _>>()?;

        Ok(Self { archives })
    }

    pub fn by_name<T: quake_traits::FromBytes>(&self, name: &str) -> anyhow::Result<T> {
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

    fn find_pak_files<P>(path: P) -> anyhow::Result<Vec<std::path::PathBuf>>
    where
        P: AsRef<std::path::Path>,
    {
        let pattern = format!("{}/**/*.pak", path.as_ref().display());
        let mut pak_files = glob::glob(&pattern)?
            .filter_map(|entry| entry.ok())
            .collect::<Vec<_>>();

        pak_files.sort_by(|a, b| b.file_name().unwrap().cmp(a.file_name().unwrap()));

        Ok(pak_files)
    }
}

#[derive(Debug)]
struct PakArchive {
    path: std::path::PathBuf,
    entries: HashMap<String, (u64, u64)>,
}

impl PakArchive {
    fn new<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<std::path::Path>,
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

    fn read_directory_entries<T>(
        reader: &mut T,
        directory_offset: u64,
        directory_count: u64,
    ) -> anyhow::Result<HashMap<String, (u64, u64)>>
    where
        T: Read + Seek,
    {
        reader.seek(std::io::SeekFrom::Start(directory_offset))?;
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
