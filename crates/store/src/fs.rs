//! File helpers: missing files read as `None`, and writes are atomic and owner-only.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write as _},
    path::Path,
};

use crate::StoreError;

pub fn read(path: &Path) -> Result<Option<Vec<u8>>, StoreError> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Writes via a temporary file and rename, so a crash never leaves a half-written file behind.
pub fn write_private(dir: &Path, name: &str, contents: &[u8]) -> Result<(), StoreError> {
    create_private_dir(dir)?;
    let path = dir.join(name);
    let temp = dir.join(format!(".{name}.tmp"));

    let mut file = private_options().open(&temp)?;
    file.write_all(contents)?;
    file.sync_all()?;
    fs::rename(&temp, &path)?;
    // Make the rename itself durable.
    if let Ok(dir) = File::open(dir) {
        dir.sync_all().ok();
    }
    Ok(())
}

pub fn remove(path: &Path) -> Result<(), StoreError> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error.into()),
        _ => Ok(()),
    }
}

fn private_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options
}

fn create_private_dir(dir: &Path) -> io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    builder.create(dir)
}
