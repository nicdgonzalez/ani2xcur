use std::path::{Path, PathBuf};
use std::io;

pub use theme::*;

use crate::manifest::{Manifest, ManifestError};

mod theme;

/// Directory containing a manifest (`Cursor.toml`) file.
#[derive(Debug, Clone)]
pub struct Package {
    path: PathBuf,
}

impl Package {
    /// Creates a new package.
    #[must_use]
    pub const fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Path to the package's root directory.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Path to the package's manifest file.
    #[must_use]
    pub fn manifest_path(&self) -> PathBuf {
        self.path.join("Cursor.toml")
    }

    /// Directory containing the built cursor theme.
    #[must_use]
    pub fn theme(&self) -> Theme {
        Theme::new(self.path.join("theme"))
    }

    /// Returns `true` if the package has been initialized.
    ///
    /// The package is considered initialized if the manifest file exists at the package's root.
    ///
    /// # Errors
    ///
    /// Returns an error if we are unable to check if the manifest file exists
    /// (e.g., insufficient permissions).
    pub fn is_initialized(&self) -> io::Result<bool> {
        self.manifest_path().try_exists()
    }

    /// Convenience function for opening a package manifest at [`Self::manifest_path`].
    ///
    /// # Errors
    ///
    /// Returns an error if opening the manifest fails.
    ///
    /// # See Also
    ///
    /// - [`Manifest::open`] for error cases.
    pub fn manifest(&self) -> Result<Manifest, ManifestError> {
        Manifest::open(self.manifest_path())
    }
}
