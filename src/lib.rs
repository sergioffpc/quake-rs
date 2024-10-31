#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use core::str;
use std::{
    fs::File,
    io::{BufReader, Cursor, Read, Seek},
    path::{Path, PathBuf},
};

use anyhow::bail;
use byteorder::LittleEndian;
use indexmap::IndexMap;

pub mod app;
pub mod audio;
pub mod console;
pub mod graphics;
pub mod input;
pub mod message;

pub trait ReadSeek: Read + Seek + Send + Sync {}

impl<R: Read + Seek + Send + Sync> ReadSeek for R {}

pub struct ResourceFiles {
    dir_path: PathBuf,
    packs: Box<[Pack<BufReader<File>>]>,
}

impl ResourceFiles {
    pub fn new<P: AsRef<Path>>(dir_path: P) -> anyhow::Result<Self> {
        let pattern = format!("{}/**/*.pak", dir_path.as_ref().display());
        let packs = glob::glob(pattern.as_str())?
            .filter_map(Result::ok)
            .map(|file_path| {
                let file = File::open(&file_path)?;
                let file_reader = BufReader::new(file);
                let pack = Pack::new(file_reader)?;

                Ok(pack)
            })
            .collect::<anyhow::Result<Vec<_>>>()?
            .into_boxed_slice();

        Ok(Self {
            dir_path: dir_path.as_ref().to_path_buf(),
            packs,
        })
    }

    pub fn take<P: AsRef<Path>>(&mut self, file_path: P) -> anyhow::Result<Box<dyn ReadSeek>> {
        let full_path = self.dir_path.join(file_path.as_ref());
        if full_path.is_file() {
            let mut file = File::open(full_path)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;

            return Ok(Box::new(Cursor::new(buf)));
        } else {
            let file_name = file_path.as_ref().to_str().unwrap();
            for pack in self.packs.iter_mut().rev() {
                if pack.file_names().any(|e| e == file_name) {
                    return pack.take(file_name);
                }
            }
            bail!("file not found: {}", file_name)
        }
    }
}

struct Pack<R> {
    reader: R,
    files: IndexMap<String, (u64, u64)>,
}

impl<R> Pack<R>
where
    R: ReadSeek,
{
    fn new(mut reader: R) -> anyhow::Result<Self> {
        let mut ident = [0u8; 4];
        reader.read_exact(&mut ident)?;
        if &ident != b"PACK" {
            bail!("invalid signature");
        }

        use byteorder::ReadBytesExt;
        let dir_offset = reader.read_i32::<LittleEndian>()?;
        let dir_length = reader.read_i32::<LittleEndian>()?;

        reader.seek(std::io::SeekFrom::Start(dir_offset as u64))?;

        let file_count = dir_length / 64;
        let mut files = IndexMap::with_capacity(file_count as usize);

        for _ in 0..file_count {
            let mut buf = [0u8; 56];
            reader.read_exact(&mut buf)?;

            // Convert buffer to string and trim null bytes
            let file_name = match str::from_utf8(&buf) {
                Ok(name) => name.trim_end_matches('\0').to_string(),
                Err(_) => bail!("invalid UTF-8 file name"),
            };

            let file_offset = reader.read_u32::<LittleEndian>()?;
            let file_length = reader.read_u32::<LittleEndian>()?;
            files.insert(file_name.into(), (file_offset as u64, file_length as u64));
        }

        Ok(Self { reader, files })
    }

    fn file_names(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(|s| s.as_ref())
    }

    fn take(&mut self, name: &str) -> anyhow::Result<Box<dyn ReadSeek>> {
        match self.files.get(name) {
            Some((file_offset, file_length)) => {
                self.reader.seek(std::io::SeekFrom::Start(*file_offset))?;

                let mut buf = vec![0; *file_length as usize];
                self.reader.read_exact(&mut buf)?;

                Ok(Box::new(Cursor::new(buf)))
            }
            None => bail!("file not found: {}", name),
        }
    }
}
