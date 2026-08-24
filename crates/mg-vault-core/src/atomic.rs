use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{Error, Result};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_path(destination: &Path) -> Result<PathBuf> {
    let parent = destination
        .parent()
        .ok_or_else(|| Error::UnsafePath(format!("{} has no parent", destination.display())))?;
    let name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Error::UnsafePath("destination filename is invalid UTF-8".into()))?;
    let serial = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    Ok(parent.join(format!(
        ".{name}.mg-vault-{}-{serial}.tmp",
        std::process::id()
    )))
}

fn write_temp(destination: &Path, bytes: &[u8]) -> Result<PathBuf> {
    let temporary = temp_path(destination)?;
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| Error::io(&temporary, error))?;
        file.write_all(bytes)
            .map_err(|error| Error::io(&temporary, error))?;
        file.sync_all()
            .map_err(|error| Error::io(&temporary, error))?;
        Ok(temporary.clone())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

pub(crate) fn create_atomic(destination: &Path, bytes: &[u8]) -> Result<()> {
    let temporary = write_temp(destination, bytes)?;
    if let Err(error) = rename_without_replace(&temporary, destination) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    sync_parent(destination)
}

#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    target_os = "tvos",
    target_os = "watchos",
    target_os = "visionos",
    target_os = "redox"
))]
fn rename_without_replace(source: &Path, destination: &Path) -> Result<()> {
    use rustix::fs::{CWD, RenameFlags, renameat_with};
    use rustix::io::Errno;

    renameat_with(CWD, source, CWD, destination, RenameFlags::NOREPLACE).map_err(|error| {
        if error == Errno::EXIST {
            Error::Collision(destination.to_path_buf())
        } else {
            Error::io(destination, error.into())
        }
    })
}

#[cfg(not(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    target_os = "tvos",
    target_os = "watchos",
    target_os = "visionos",
    target_os = "redox"
)))]
fn rename_without_replace(source: &Path, destination: &Path) -> Result<()> {
    fs::hard_link(source, destination).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            Error::Collision(destination.to_path_buf())
        } else {
            Error::io(destination, error)
        }
    })?;
    fs::remove_file(source).map_err(|error| Error::io(source, error))
}

pub(crate) fn replace_atomic(destination: &Path, bytes: &[u8]) -> Result<()> {
    let temporary = write_temp(destination, bytes)?;
    if let Err(error) = fs::rename(&temporary, destination) {
        let _ = fs::remove_file(&temporary);
        return Err(Error::io(destination, error));
    }
    sync_parent(destination)
}

pub(crate) fn sync_parent(path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::UnsafePath(format!("{} has no parent", path.display())))?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| Error::io(parent, error))
}
