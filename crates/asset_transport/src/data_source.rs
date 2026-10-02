use std::io::{self, Read, Seek};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

pub trait GameDataReader: Read + Seek + Send {}
impl<T: Read + Seek + Send> GameDataReader for T {}

#[derive(Clone, Debug)]
pub struct GameDataMetadata {
    pub directory: bool,
    pub size: Option<u64>,
    pub modified: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct GameDataEntry {
    pub path: String,
    pub metadata: GameDataMetadata,
}

pub trait GameDataSource: Send + Sync {
    fn open(&self, path: &str) -> io::Result<Box<dyn GameDataReader>>;
    fn metadata(&self, path: &str) -> io::Result<GameDataMetadata>;
    fn read_dir(&self, path: &str) -> io::Result<Vec<GameDataEntry>>;
    fn exists(&self, path: &str) -> io::Result<bool> {
        match self.metadata(path) {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }
    fn read(&self, path: &str) -> io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        self.open(path)?.read_to_end(&mut bytes)?;
        Ok(bytes)
    }
}

#[derive(Clone)]
pub struct GameDataFile {
    pub source: Arc<dyn GameDataSource>,
    pub path: String,
}

pub struct DesktopPathDataSource {
    root: PathBuf,
}

impl DesktopPathDataSource {
    pub fn new(root: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self {
            root: root.as_ref().canonicalize()?,
        })
    }
    fn resolve(&self, path: &str) -> io::Result<PathBuf> {
        validate_relative_path(path)?;
        let resolved = self.root.join(path).canonicalize()?;
        if !resolved.starts_with(&self.root) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Game data path leaves the selected installation",
            ));
        }
        Ok(resolved)
    }
}

pub fn validate_relative_path(path: &str) -> io::Result<()> {
    if path.contains('\\')
        || Path::new(path)
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Expected an installation-relative game data path",
        ));
    }
    Ok(())
}

impl GameDataSource for DesktopPathDataSource {
    fn open(&self, path: &str) -> io::Result<Box<dyn GameDataReader>> {
        Ok(Box::new(std::fs::File::open(self.resolve(path)?)?))
    }
    fn metadata(&self, path: &str) -> io::Result<GameDataMetadata> {
        let metadata = std::fs::metadata(self.resolve(path)?)?;
        Ok(GameDataMetadata {
            directory: metadata.is_dir(),
            size: Some(metadata.len()),
            modified: metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|time| time.as_millis() as u64),
        })
    }
    fn read_dir(&self, path: &str) -> io::Result<Vec<GameDataEntry>> {
        std::fs::read_dir(self.resolve(path)?)?
            .map(|entry| {
                let entry = entry?;
                let name = entry.file_name().into_string().map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Game data filename is not UTF-8",
                    )
                })?;
                let relative = if path.is_empty() {
                    name
                } else {
                    format!("{path}/{name}")
                };
                Ok(GameDataEntry {
                    metadata: self.metadata(&relative)?,
                    path: relative,
                })
            })
            .collect()
    }
}

pub fn find_game_files(source: &dyn GameDataSource) -> io::Result<Vec<GameDataEntry>> {
    let mut directories = vec![String::new()];
    let mut files = Vec::new();
    let mut visited = 0;
    while let Some(directory) = directories.pop() {
        if directory.split('/').count() > 16 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Game installation is too deeply nested"));
        }
        for entry in source.read_dir(&directory)? {
            visited += 1;
            if entry.metadata.directory {
                directories.push(entry.path);
            } else {
                files.push(entry);
            }
        }
        if visited > 50_000 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Game installation contains too many entries",
            ));
        }
    }
    Ok(files)
}

pub fn read_iwd_entry(
    source: &dyn GameDataSource,
    archive: &str,
    entry: &str,
) -> Result<Vec<u8>, String> {
    let file = source.open(archive).map_err(|error| error.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|error| error.to_string())?;
    let mut entry = zip.by_name(entry).map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    entry
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    Ok(bytes)
}
