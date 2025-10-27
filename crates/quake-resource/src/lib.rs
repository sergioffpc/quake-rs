pub mod pak;
pub mod wad;

pub struct Resources {
    basedir: std::path::PathBuf,
    pak: pak::Pak,
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

        let basedir = path.as_ref().to_owned().canonicalize()?;
        let pak = pak::Pak::new(path.as_ref())?;

        Ok(Self { basedir, pak })
    }

    pub fn by_name<T: FromBytes>(&mut self, name: &str) -> anyhow::Result<T> {
        // Try loading from filesystem first, then fall back to PAK archives
        self.load_from_filesystem(name)
            .or_else(|_| self.pak.by_name(name))
    }

    fn load_from_filesystem<T: FromBytes>(&mut self, name: &str) -> anyhow::Result<T> {
        let path = self
            .basedir
            .join(name)
            .canonicalize()
            .map_err(|_| anyhow::anyhow!("File not found in filesystem: {}", name))?;

        let bytes = std::fs::read(path)?;
        T::from_bytes(&bytes)
    }
}

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

pub(crate) fn read_null_terminated_string<R>(
    reader: &mut R,
    buffer_size: usize,
) -> anyhow::Result<String>
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
