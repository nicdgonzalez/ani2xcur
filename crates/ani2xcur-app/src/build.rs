use std::error::Error;
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use std::{io, iter, thread};

use ani::Ani;
use ani2xcur_core::{Cursor, InitializeThemeError, ManifestError, Package, Size};
use tracing::{error, error_span};

use crate::convert::{ConvertCursorError, ConvertCursorRequest, xcursor_from_ani};

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

    #[error("failed to open manifest")]
    OpenManifest(#[source] ManifestError),

    #[error("failed to initialize theme directory")]
    InitializeTheme(#[source] InitializeThemeError),

    #[error("failed to decode ANI")]
    DecodeAni(#[source] ani::ParseError),

    #[error("failed to convert cursor")]
    ConvertCursor(#[source] ConvertCursorError),

    #[error("failed to save Xcursor")]
    SaveXcursor(#[source] io::Error),

    #[error("failed to symlink alias")]
    Symlink(#[source] io::Error),

    #[error("failed to create ({count}) cursors")]
    ErrorCount { count: usize },
}

/// Converts all ANI cursors into Xcursors.
#[expect(clippy::missing_panics_doc, clippy::missing_errors_doc)]
pub fn build_package(request: BuildPackageRequest) -> Result<(), BuildPackageError> {
    let package = Package::new(request.path);

    let is_initialized = package
        .is_initialized()
        .map_err(BuildPackageError::CheckPackageInitialized)?;

    if !is_initialized {
        return Err(BuildPackageError::PackageNotInitialized);
    }

    let manifest = package
        .manifest()
        .map_err(BuildPackageError::OpenManifest)?;

    package
        .theme()
        .initialize(manifest.theme())
        .map_err(BuildPackageError::InitializeTheme)?;

    thread::scope(|scope| {
        let handles = manifest
            .cursors()
            .iter()
            .map(|cursor| {
                // Attach context so we know which thread is emitting the events.
                let span = error_span!("", cursor = ?cursor.kind());

                let package = &package;
                let sizes = &request.sizes;
                let handle = scope
                    .spawn(move || span.in_scope(move || build_cursors(cursor, package, sizes)));

                (cursor.kind(), handle)
            })
            .collect::<Vec<_>>();

        let errors = handles
            .into_iter()
            .filter_map(|(name, handle)| match handle.join() {
                Ok(Ok(())) => None,
                Ok(Err(err)) => Some((name, err)),
                Err(err) => panic!("thread for {name:?} panicked: {err:?}"),
            })
            .collect::<Vec<_>>();

        let error_count = errors.len();

        for (name, error) in errors {
            let chain = iter::successors(error.source(), |e| Error::source(*e))
                .map(|cause| format!("  Cause: {cause}"))
                .collect::<Vec<_>>()
                .join("\n");

            error!("failed to convert cursor: {name}:\n{chain}");
        }

        if error_count > 0 {
            return Err(BuildPackageError::ErrorCount { count: error_count });
        }

        Ok(())
    })
}

fn build_cursors(
    cursor: &Cursor,
    package: &Package,
    sizes: &[Size],
) -> Result<(), BuildPackageError> {
    let input = package.path().join(cursor.path());
    let ani = Ani::open(&input).map_err(BuildPackageError::DecodeAni)?;

    let request = ConvertCursorRequest {
        ani,
        sizes: sizes.to_vec(),
    };
    let xcursor = xcursor_from_ani(request).map_err(BuildPackageError::ConvertCursor)?;

    let cursors = package.theme().cursors();
    let output = cursors.join(cursor.name());

    xcursor
        .save(&output)
        .map_err(BuildPackageError::SaveXcursor)?;

    let missing_aliases = cursor
        .aliases()
        .iter()
        .map(|alias| cursors.join(alias))
        .filter(|path| !path.try_exists().unwrap_or(false));

    for alias in missing_aliases {
        symlink(&output, &alias).map_err(BuildPackageError::Symlink)?;
    }

    Ok(())
}
