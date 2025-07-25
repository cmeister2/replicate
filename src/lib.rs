//! Copies the currently running program into a temporary location
//!
//! This crate copies the currently running executable into a
//! temporary location and returns the path to that executable.
//! This allows you to (for example):
//!
//! - Compile a program statically using something like musl
//! - Create a copy of that program while it's running
//! - Run Docker from your original program, mounting the copy as a Docker volume mount
//! - Run the copied program from within the Dockerized environment.
//!
//! Because this library uses [`NamedTempFile`] via [`Builder`] to generate a temporary location,
//! the following security restrictions apply to [`Replicate`]:
//!
//! 1. The copy has a short lifetime and your temporary file cleaner is sane (doesn’t delete recently accessed files).
//! 2. You trust every user on your system (i.e. you are the only user).
//! 3. You have disabled your system’s temporary file cleaner or verified that your system doesn’t have a temporary file cleaner.
//!
#![deny(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    unused_results
)]

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    fs::Permissions,
    io::Write,
    ops::Deref,
    path::{Path, PathBuf},
};

#[cfg(doc)]
use tempfile::NamedTempFile;
use tempfile::{Builder, TempDir, TempPath};

enum ReplicatePath {
    TempPath(TempPath),
    PathBuf(PathBuf),
}

/// A temporary copy of the running executable.
///
/// # Example
///
/// ```
/// use replicate::Replicate;
/// # fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
/// let copy = Replicate::new()?;
///
/// println!("My copy's path is {}", copy.display());
///
/// # Ok(())
/// # }
/// ```
pub struct Replicate {
    /// The parent folder where the copy is stored.
    parent: TempDir,
    /// The full path to the copy of the executable.
    path: ReplicatePath,
}

impl Replicate {
    /// Creates a replicate of the currently running program. The
    /// copy is deleted when this is dropped.
    pub fn new() -> Result<Self, std::io::Error> {
        // Create a temporary directory to hold the copy.
        let parent = tempfile::tempdir()?;

        // Create a new temporary file in the temporary directory.
        let mut copy = Builder::new()
            .prefix("replicate_")
            .rand_bytes(5)
            .tempfile_in(parent.path())?;

        // Copy the contents of this program into the copy.
        let _ = Self::copy_self_into_writer(&mut copy)?;

        // Convert the copy into a TempPath so we can pass around the path info.
        let path = copy.into_temp_path();

        // Try and make the copy executable.
        Self::make_executable(&path)?;

        // Return the Replicate.
        Ok(Self {
            parent,
            path: ReplicatePath::TempPath(path),
        })
    }

    fn copy_self_into_writer<W: ?Sized + Write>(writer: &mut W) -> std::io::Result<u64> {
        let mut self_exe = exe()?;
        std::io::copy(&mut self_exe, writer)
    }

    fn make_executable<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
        #[cfg(unix)]
        {
            let permissions = Permissions::from_mode(0o755);
            std::fs::set_permissions(path.as_ref(), permissions)
        }
        #[cfg(not(unix))]
        {
            Ok(())
        }
    }

    /// Creates a replicate of the currently running program with the same name.
    /// The parent directory is cleaned up when this is dropped.
    pub fn same_name() -> std::io::Result<Self> {
        let current_exe_path = std::env::current_exe()?;

        let filename = current_exe_path
            .file_name()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "No file name"))?;

        // Create a temporary directory to hold the copy.
        let parent = tempfile::tempdir()?;

        // Create a new temporary file in the temporary directory with the same name.
        let copy_path = parent.path().join(filename);

        {
            // Open the file for writing.
            let mut copy = std::fs::File::create(&copy_path)?;

            // Copy the contents of this program into the copy.
            let _ = Self::copy_self_into_writer(&mut copy)?;
        }

        // Try and make the copy executable.
        Self::make_executable(&copy_path)?;

        // Return the Replicate.
        Ok(Self {
            parent,
            path: ReplicatePath::PathBuf(copy_path),
        })
    }

    /// Returns the parent directory of the copy.
    pub fn parent(&self) -> &Path {
        self.parent.path()
    }

    /// Returns the path of the copy.
    pub fn path(&self) -> &Path {
        match &self.path {
            ReplicatePath::TempPath(temp_path) => temp_path.as_ref(),
            ReplicatePath::PathBuf(path_buf) => path_buf.as_ref(),
        }
    }
}

impl Deref for Replicate {
    type Target = Path;

    fn deref(&self) -> &Path {
        self.path()
    }
}

impl AsRef<Path> for Replicate {
    fn as_ref(&self) -> &Path {
        self.path()
    }
}

/// Returns a [File](std::fs::File) of the currently running executable. Akin to `fd::File::open("/proc/self/exe")` on Linux.
pub fn exe() -> std::io::Result<std::fs::File> {
    exe_path().and_then(std::fs::File::open)
}

/// Returns the path of the currently running executable. On Linux this is `/proc/self/exe`.
// https://stackoverflow.com/questions/1023306/finding-current-executables-path-without-proc-self-exe
pub fn exe_path() -> std::io::Result<PathBuf> {
    #[cfg(any(target_os = "android", target_os = "linux"))]
    {
        Ok(PathBuf::from("/proc/self/exe"))
    }
    #[cfg(target_os = "dragonfly")]
    {
        Ok(PathBuf::from("/proc/curproc/file"))
    }
    #[cfg(target_os = "netbsd")]
    {
        Ok(PathBuf::from("/proc/curproc/exe"))
    }
    #[cfg(target_os = "solaris")]
    {
        Ok(PathBuf::from(format!(
            "/proc/{}/path/a.out",
            nix::unistd::getpid()
        ))) // or /proc/{}/object/a.out ?
    }
    #[cfg(not(any(
        target_os = "android",
        target_os = "dragonfly",
        target_os = "linux",
        target_os = "netbsd",
        target_os = "solaris"
    )))]
    {
        std::env::current_exe()
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    #[test]
    fn create_replicate() -> anyhow::Result<()> {
        let copy = Replicate::new()?;
        println!("Created new copy: {}", copy.display());

        let name = copy
            .file_name()
            .and_then(OsStr::to_str)
            .expect("Failed to copy program");

        // Verify the name starts with "replicate"
        assert!(name.starts_with("replicate"));
        Ok(())
    }

    #[test]
    fn create_replicate_with_same_name() -> anyhow::Result<()> {
        let copy = Replicate::same_name()?;
        println!("Created new copy: {}", copy.display());

        let name = copy
            .file_name()
            .and_then(OsStr::to_str)
            .expect("Failed to copy program");

        // Verify the name starts with "replicate"
        assert!(name.starts_with("replicate"));
        Ok(())
    }

    #[test]
    fn test_that_files_are_cleared_up() -> anyhow::Result<()> {
        let path_str = {
            let copy = Replicate::new()?;
            println!("My copy's path is {}", copy.display());
            copy.parent().to_path_buf()
        }; // The copy should be cleaned up here

        // Verify that the temporary directory is deleted
        assert!(!path_str.exists(), "Temporary directory still exists");

        Ok(())
    }
}
