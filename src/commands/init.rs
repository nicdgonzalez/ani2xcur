use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::{fs, io};

use anyhow::Context as _;
use colored::Colorize as _;
use inf::{Entry, Inf, Section, Value};
use tracing::info;

use crate::commands::prelude::*;
use crate::config::{Config, Cursor};
use crate::cursors::{CURSORS, DEFAULT_FILE_NAMES};

#[derive(Debug, Default, clap::Args)]
pub struct Init {
    /// Unique name for the Cursor Theme. Defaults to the name specified in the INF file.
    #[arg(long)]
    pub theme: Option<String>,

    /// Path to INF file. Defaults to `./Install.inf`.
    #[arg(long)]
    pub inf: Option<PathBuf>,

    /// Overwrite existing Cursor.toml file if it already exists.
    #[arg(long)]
    pub overwrite: bool,

    /// Create a generic Manifest instead of parsing the INF file.
    #[arg(long, conflicts_with = "inf")]
    pub skip_inf: bool,
}

impl Run for Init {
    fn run(self, ctx: &mut Context) -> anyhow::Result<()> {
        let manifest_path = ctx.package.manifest();

        if manifest_path.exists() && !self.overwrite {
            anyhow::bail!(
                "Cursor.toml file already exists. Use --overwrite to replace the existing file"
            );
        }

        let config = if self.skip_inf {
            create_manifest_default(self.theme.as_deref())
        } else {
            create_manifest(self.theme, ctx.package.as_path(), self.inf)?
        };

        save_config(&config, &manifest_path)?;
        assert_eq!(manifest_path.try_exists().ok(), Some(true));

        writeln!(io::stderr(), "{}", "Created Cursor.toml".bold().green()).ok();

        Ok(())
    }
}

fn create_manifest_default(theme: Option<&str>) -> Config {
    let theme = theme.unwrap_or("Unnamed Theme").to_owned();
    let cursors = DEFAULT_FILE_NAMES
        .iter()
        .enumerate()
        .map(|(i, input)| {
            let cursor = &CURSORS[i];
            let name = cursor.name.to_owned();
            let aliases = cursor.aliases.iter().copied().map(String::from).collect();
            Cursor::new(name, aliases, PathBuf::from(input))
        })
        .collect::<Vec<_>>();

    Config::new(theme, cursors)
}

fn create_manifest(
    theme: Option<String>,
    current_dir: &Path,
    inf: Option<PathBuf>,
) -> anyhow::Result<Config> {
    let path = inf
        .unwrap_or_else(|| current_dir.join("Install.inf"))
        .canonicalize()
        .context("INF file not found")?;

    let mut reader = fs::File::open(path).context("failed to open INF file")?;
    let inf = Inf::from_reader(&mut reader).context("failed to parse INF file")?;
    let config = build_config(&inf, theme)?;

    Ok(config)
}

/// Read from the INF data to construct the Cursor.toml configuration.
fn build_config(inf: &Inf, theme_override: Option<String>) -> anyhow::Result<Config> {
    let strings = inf
        .get("Strings")
        .context("expected 'Strings' section in INF")?;

    let default_install = inf
        .get("DefaultInstall")
        .context("expected 'DefaultInstall' section in INF")?;

    let section_names = get_section_names(default_install)
        .context("missing 'AddReg' entry in 'DefaultInstall' section")?;
    let values = get_cursor_scheme_entry(inf, section_names)
        .context("failed to find entry with cursor scheme information")?;
    let cursor_files = get_cursor_files(values, strings)?.collect::<anyhow::Result<Vec<_>>>()?;

    let theme = resolve_cursor_theme(values, strings, theme_override)?;
    let cursors = get_cursors(&cursor_files).collect();

    Ok(Config::new(theme, cursors))
}

/// Search for the `AddReg` entry in `section` and return the section name(s) referenced in it's
/// value. If the `AddReg` entry is missing, `None` is returned instead.
fn get_section_names(section: &Section) -> Option<&[String]> {
    section.entries().iter().find_map(|entry| match entry {
        Entry::Item(key, v) if key.as_str() == "AddReg" => match v {
            Value::Raw(value) => Some(std::slice::from_ref(value)),
            Value::List(values) => Some(values.as_slice()),
        },
        _ => None,
    })
}

/// Search through the sections provided for an entry containing the cursor scheme definition.
/// If no entry matches the target `subkey`, `None` is returned instead.
fn get_cursor_scheme_entry<'a>(inf: &'a Inf, section_names: &[String]) -> Option<&'a [String]> {
    section_names.iter().find_map(|name| {
        let section = inf.get(name)?;
        section.entries().iter().find_map(|entry| {
            let Entry::Value(Value::List(values)) = entry else {
                return None;
            };

            values
                .get(1)
                .is_some_and(|subkey| subkey == "Control Panel\\Cursors\\Schemes")
                .then_some(values.as_ref())
        })
    })
}

/// Parse the cursor scheme entry to get the cursor files.
fn get_cursor_files(
    values: &[String],
    strings: &Section,
) -> anyhow::Result<impl Iterator<Item = anyhow::Result<String>>> {
    Ok(values
        .get(4)
        .context("missing value for cursor files")?
        .split_terminator(',')
        .map(|path| {
            // `split` always returns a value, even if it's just the original string.
            // TODO: Find out if the INF file has to be in the same directory as the cursors.
            let file_name = path.split('\\').next_back().unwrap();
            inf::util::expand_vars(file_name, strings).context("failed to expand cursor file value")
        }))
}

/// Parse the cursor scheme entry to get the scheme name.
fn resolve_cursor_theme(
    values: &[String],
    strings: &Section,
    theme_override: Option<String>,
) -> anyhow::Result<String> {
    if let Some(theme) = theme_override {
        Ok(theme)
    } else {
        let value = values.get(2).context("missing value for scheme name")?;
        inf::util::expand_vars(value, strings).context("failed to expand scheme name value")
    }
}

/// Convert cursor file names into [`crate::config::Cursor`]s with pre-defined data.
fn get_cursors(cursor_files: &[String]) -> impl Iterator<Item = Cursor> {
    cursor_files
        .iter()
        .enumerate()
        .filter_map(|(i, file_name)| {
            let cursor = &CURSORS[i];
            let name = cursor.name.to_owned();
            let aliases = cursor.aliases.iter().copied().map(String::from).collect();

            if file_name.is_empty() {
                None // Path to cursor was empty (e.g., `"path\to\cursor1,,path\to\cursor3"`).
            } else {
                Some(Cursor::new(name, aliases, PathBuf::from(file_name)))
            }
        })
}

/// Write `config` to the file at `manifest_path`.
fn save_config(config: &Config, manifest_path: &Path) -> anyhow::Result<()> {
    let text = toml::to_string(config).expect("expected config to be serializable as TOML");
    fs::write(manifest_path, text).context("failed to create Cursor.toml file")?;
    info!("created Cursor.toml file: {:#}", manifest_path.display());
    Ok(())
}
