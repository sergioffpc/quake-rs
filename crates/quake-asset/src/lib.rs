use itertools::Itertools;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

pub mod pak;

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

pub trait Archive: Send + Sync {
    fn by_name_bytes(&self, name: &str) -> anyhow::Result<Vec<u8>>;

    fn file_names(&self) -> Box<dyn Iterator<Item = String> + '_>;
}

pub trait ArchiveExt: Archive {
    fn by_name<T: FromBytes>(&self, name: &str) -> anyhow::Result<T> {
        let bytes = self.by_name_bytes(name)?;
        T::from_bytes(&bytes)
    }
}

impl<A: Archive + ?Sized> ArchiveExt for A {}

pub struct AssetManager {
    base_path: PathBuf,
    archives: Vec<Box<dyn Archive>>,
}

impl AssetManager {
    pub fn new<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let path_ref = path.as_ref();

        if !path_ref.exists() || !path_ref.is_dir() {
            return Err(anyhow::anyhow!("Directory not found or is not a directory"));
        }

        debug!(path=?std::fs::canonicalize(path_ref)?, "loading assets from");

        let base_path = path.as_ref().to_owned().canonicalize()?;
        let pak = pak::Pak::new(path.as_ref())?;

        Ok(Self {
            base_path,
            archives: vec![Box::new(pak)],
        })
    }

    pub fn by_name<T: FromBytes>(&self, name: &str) -> anyhow::Result<T> {
        info!(?name, "loading asset");

        // Try loading from filesystem first, then fall back to archives
        match self.load_from_filesystem(name) {
            Ok(result) => Ok(result),
            Err(_) => {
                for archive in &self.archives {
                    if let Ok(result) = archive.by_name::<T>(name) {
                        return Ok(result);
                    }
                }
                Err(anyhow::anyhow!("File not found: {}", name))
            }
        }
    }

    pub fn file_names(&self) -> impl Iterator<Item = String> {
        let pattern = format!("{}/**/*", self.base_path.display());

        fn is_ignored_extension(path: &Path) -> bool {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ["pak"].contains(&ext))
        }

        let filesystem_name_iter: Box<dyn Iterator<Item = String> + '_> = match glob::glob(&pattern)
        {
            Ok(paths) => Box::new(
                paths
                    .filter_map(Result::ok)
                    .filter(|path| path.is_file() && !is_ignored_extension(path))
                    .filter_map(|path| {
                        path.strip_prefix(&self.base_path)
                            .ok()
                            .and_then(|p| p.to_str())
                            .map(str::to_owned)
                    }),
            ),
            Err(_) => Box::new(std::iter::empty()),
        };

        let archive_name_iter = self
            .archives
            .iter()
            .flat_map(|archive| archive.file_names())
            .sorted();

        filesystem_name_iter.chain(archive_name_iter)
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
