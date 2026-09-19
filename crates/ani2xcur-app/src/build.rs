use std::io;
use std::path::PathBuf;

use ani2xcur_core::{Package, Size};

/// Request to build a package.
pub struct BuildPackageRequest {
    pub path: PathBuf,
    pub sizes: Vec<Size>,
}

/// Errors that can occur while building a package.
#[derive(Debug, thiserror::Error)]
pub enum BuildPackageError {
    // Failed to check if the package is already initialized (e.g., insufficient permissions).
    #[error("failed to check if package is already initialized")]
    CheckPackageInitialized(#[source] io::Error),

    #[error("package not initialized")]
    PackageNotInitialized,
}

/// Converts all ANI cursors into Xcursors.
pub fn build_package(request: BuildPackageRequest) -> Result<(), BuildPackageError> {
    let package = Package::new(request.path);

    let is_initialized = package
        .is_initialized()
        .map_err(BuildPackageError::CheckPackageInitialized)?;

    if !is_initialized {
        return Err(BuildPackageError::PackageNotInitialized);
    }

    // Read the package manifest.

    // Construct the theme directory.

    // Convert each cursor in the manifest.

    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foo() {}
}
