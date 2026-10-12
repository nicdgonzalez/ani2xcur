use std::io;
use std::os::unix::fs::symlink;
use std::path::PathBuf;

use ani2xcur_core::{ManifestError, Package, default_sizes};
use anyhow::Context as _;
use thiserror::Error;

use crate::build::{BuildPackageError, BuildPackageRequest, build_package};
use crate::init::{InitializePackageError, InitializePackageRequest, initialize_package};

pub struct InstallThemeRequest {
    pub input: PathBuf,
    pub default_init: bool,
}

#[derive(Debug, Error)]
pub enum InstallThemeError {
    #[error("failed to check if package is already initialized")]
    CheckPackageInitialized(#[source] io::Error),

    #[error("failed to intitialize package")]
    InitFailed(#[source] InitializePackageError),

    #[error("package not initialized")]
    NotInitialized,

    #[error("failed to check if theme already exists")]
    CheckThemeExists(#[source] io::Error),

    #[error("failed to build package")]
    BuildFailed(#[source] BuildPackageError),

    #[error("failed to check if target theme already exists")]
    CheckTargetThemeExists(#[source] io::Error),

    #[error("failed to open manifest file")]
    OpenManifestFailed(#[source] ManifestError),

    #[error("failed to get `icons` directory")]
    GetIconsDir(#[source] anyhow::Error),

    #[error("theme {theme:?} is already installed")]
    AlreadyInstalled { theme: String },

    #[error("failed to symlink target theme")]
    SymlinkFailed(#[source] io::Error),
}

/// # Errors
///
/// todo
pub fn install_theme(request: InstallThemeRequest) -> Result<String, InstallThemeError> {
    let package = Package::new(request.input);
    let is_initialized = package
        .is_initialized()
        .map_err(InstallThemeError::CheckPackageInitialized)?;

    if !is_initialized {
        if request.default_init {
            let request = InitializePackageRequest {
                path: package.path().to_path_buf(),
                overwrite: false,
                inf: None,
                theme: None,
            };

            initialize_package(request).map_err(InstallThemeError::InitFailed)?;
        } else {
            return Err(InstallThemeError::NotInitialized);
        }
    }

    let theme = package.theme();
    let theme_exists = theme
        .path()
        .try_exists()
        .map_err(InstallThemeError::CheckThemeExists)?;

    if !theme_exists {
        let request = BuildPackageRequest {
            path: package.path().to_owned(),
            sizes: default_sizes().to_vec(),
        };

        build_package(request).map_err(InstallThemeError::BuildFailed)?;
    }

    let manifest = package
        .manifest()
        .map_err(InstallThemeError::OpenManifestFailed)?;

    let mut target_theme = get_icons_dir().map_err(InstallThemeError::GetIconsDir)?;
    target_theme.push(manifest.theme());

    let target_theme_exists = target_theme
        .try_exists()
        .map_err(InstallThemeError::CheckTargetThemeExists)?;

    if target_theme_exists {
        Err(InstallThemeError::AlreadyInstalled {
            theme: manifest.theme().to_owned(),
        })
    } else {
        symlink(theme.path(), &target_theme).map_err(InstallThemeError::SymlinkFailed)?;
        Ok(manifest.theme().to_owned())
    }
}

// TODO: This should probably be in the CLI... I need to reconsider what goes inside the `app` vs
// what is CLI specific.
pub(crate) fn get_icons_dir() -> anyhow::Result<PathBuf> {
    let mut legacy_path = dirs::home_dir().context("failed to get home directory")?;
    legacy_path.push(".icons");

    if legacy_path.exists() {
        return Ok(legacy_path);
    }

    let mut modern = dirs::data_local_dir().context("failed to get data directory")?;
    modern.push("icons");

    Ok(modern)
}
