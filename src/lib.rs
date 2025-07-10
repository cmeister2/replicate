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
use std::{fs::Permissions, ops::Deref, path::Path};

use palaver::env::exe;
#[cfg(doc)]
use tempfile::NamedTempFile;
use tempfile::{Builder, TempDir, TempPath};

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
    path: TempPath,
}

impl Replicate {
    /// Creates a replicate of the currently running program. The
    /// copy is deleted when this is dropped.
    pub fn new() -> Result<Self, std::io::Error> {
        // Use palaver to get a `File` reference to the currently running program.
        let mut self_exe = exe()?;

        // Create a temporary directory to hold the copy.
        let parent = tempfile::tempdir()?;

        // Create a new temporary file in the temporary directory.
        let mut copy = Builder::new()
            .prefix("replicate_")
            .rand_bytes(5)
            .tempfile_in(parent.path())?;

        // Copy the contents of this program into the copy.
        let _ = std::io::copy(&mut self_exe, &mut copy)?;

        // Convert the copy into a TempPath so we can pass around the path info.
        let path = copy.into_temp_path();

        // If necessary make the copy executable.
        #[cfg(unix)]
        {
            let permissions = Permissions::from_mode(0o755);
            std::fs::set_permissions(&path, permissions)?;
        }

        // Return the Replicate.
        Ok(Self { parent, path })
    }

    /// Returns the parent directory of the copy.
    pub fn parent(&self) -> &Path {
        self.parent.path()
    }

    /// Returns the path of the copy.
    pub fn path(&self) -> &Path {
        self.path.as_ref()
    }
}

impl Deref for Replicate {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<Path> for Replicate {
    fn as_ref(&self) -> &Path {
        &self.path
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
}
