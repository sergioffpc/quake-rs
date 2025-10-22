use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashMap;
use std::fs::{DirEntry, File};
use std::io::BufReader;

pub trait FromBytes: Sized {
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

#[derive(Debug)]
pub struct Pack {
    archives: Box<[PackArchive<BufReader<File>>]>,
}

impl Pack {
    pub fn new<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<std::path::Path>,
    {
        let pak_files = Self::find_pak_files(path)?;
        let archives = Self::load_archives(pak_files)?;
        Ok(Self { archives })
    }

    fn find_pak_files<P>(path: P) -> anyhow::Result<Vec<DirEntry>>
    where
        P: AsRef<std::path::Path>,
    {
        let files = std::fs::read_dir(path)?;
        let mut pak_files = files
            .filter_map(|f| f.ok())
            .filter(Self::is_pak_file)
            .collect::<Vec<_>>();

        pak_files.sort_by(|a, b| {
            b.path()
                .file_name()
                .unwrap()
                .cmp(a.path().file_name().unwrap())
        });

        Ok(pak_files)
    }

    fn is_pak_file(entry: &DirEntry) -> bool {
        const PAK_EXTENSION: &str = "pak";

        entry
            .path()
            .extension()
            .map(|e| e.to_ascii_lowercase() == PAK_EXTENSION)
            .unwrap_or(false)
    }

    fn load_archives(files: Vec<DirEntry>) -> anyhow::Result<Box<[PackArchive<BufReader<File>>]>> {
        files
            .iter()
            .map(|f| {
                let reader = BufReader::new(File::open(f.path())?);
                PackArchive::new(reader)
            })
            .collect::<Result<_, _>>()
    }

    pub fn by_name<T: FromBytes>(&mut self, name: &str) -> anyhow::Result<T> {
        for archive in &mut self.archives {
            if let Ok(data) = archive.by_name(name) {
                return T::from_bytes(&data);
            }
        }
        Err(anyhow::anyhow!("File not found"))
    }

    pub fn file_names(&self) -> impl Iterator<Item = &str> {
        self.archives.iter().flat_map(|a| a.file_names())
    }
}

#[derive(Debug)]
struct PackArchive<R>
where
    R: std::io::Read + std::io::Seek,
{
    reader: R,
    entries: HashMap<String, (u64, u64)>,
}

impl<R> PackArchive<R>
where
    R: std::io::Read + std::io::Seek,
{
    fn new(mut reader: R) -> anyhow::Result<Self> {
        Self::validate_pak_header(&mut reader)?;
        let (directory_offset, directory_length) = Self::read_directory_info(&mut reader)?;
        let entries =
            Self::read_directory_entries(&mut reader, directory_offset, directory_length)?;

        Ok(Self { reader, entries })
    }

    fn validate_pak_header<T: std::io::Read>(reader: &mut T) -> anyhow::Result<()> {
        let mut ident = [0u8; 4];
        reader.read_exact(&mut ident)?;
        if ident != *b"PACK" {
            return Err(anyhow::anyhow!("Invalid PAK file"));
        }
        Ok(())
    }

    fn read_directory_info<T: std::io::Read>(reader: &mut T) -> anyhow::Result<(u64, u64)> {
        const DIRECTORY_ENTRY_SIZE: u64 = 0x40;

        let directory_offset = reader.read_u32::<LittleEndian>()? as u64;
        let directory_length = reader.read_u32::<LittleEndian>()? as u64 / DIRECTORY_ENTRY_SIZE;
        Ok((directory_offset, directory_length))
    }

    fn read_directory_entries<T: std::io::Read + std::io::Seek>(
        reader: &mut T,
        directory_offset: u64,
        directory_length: u64,
    ) -> anyhow::Result<HashMap<String, (u64, u64)>> {
        reader.seek(std::io::SeekFrom::Start(directory_offset))?;
        let mut entries = HashMap::with_capacity(directory_length as usize);

        for _ in 0..directory_length {
            let entry_name = Self::read_entry_name(reader)?;
            let entry_offset = reader.read_u32::<LittleEndian>()? as u64;
            let entry_size = reader.read_u32::<LittleEndian>()? as u64;
            entries.insert(entry_name, (entry_offset, entry_size));
        }

        Ok(entries)
    }

    fn read_entry_name<T: std::io::Read>(reader: &mut T) -> anyhow::Result<String> {
        const ENTRY_NAME_SIZE: usize = 0x38;

        let mut entry_name_buffer = [0u8; ENTRY_NAME_SIZE];
        reader.read_exact(&mut entry_name_buffer)?;
        let null_terminated_bytes: Vec<u8> = entry_name_buffer
            .iter()
            .take_while(|c| **c != 0)
            .cloned()
            .collect();
        Ok(String::from_utf8_lossy(&null_terminated_bytes).to_string())
    }

    fn by_name(&mut self, name: &str) -> anyhow::Result<Vec<u8>> {
        let (offset, size) = self
            .entries
            .get(name)
            .ok_or(anyhow::anyhow!("File not found"))?;
        let mut buffer = vec![0u8; *size as usize];
        self.reader.seek(std::io::SeekFrom::Start(*offset))?;
        self.reader.read_exact(&mut buffer)?;
        Ok(buffer)
    }

    fn file_names(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(|s| s.as_str())
    }
}
