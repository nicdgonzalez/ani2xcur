use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::{fs, io};

use anyhow::Context as _;
use colored::Colorize as _;
use inf::{Entry, Inf, Section, Value};

use crate::commands::prelude::*;
use crate::config::{Config, Cursor};

#[derive(Debug, Default, clap::Args)]
pub struct Init {
    /// Unique name for the Cursor Theme. Defaults to the name specified in the INF file.
    #[arg(long)]
    theme: Option<String>,

    /// Path to INF file. Defaults to `./Install.inf`.
    #[arg(long)]
    inf: Option<PathBuf>,

    /// Overwrite existing Cursor.toml file if it already exists.
    #[arg(long)]
    overwrite: bool,
}

impl Run for Init {
    fn run(&self, ctx: &mut Context) -> anyhow::Result<()> {
        let path = self
            .inf
            .clone()
            .unwrap_or_else(|| ctx.package.as_path().join("Install.inf"))
            .canonicalize()
            .context("INF file not found")?;

        let mut reader = fs::File::open(path).context("failed to open INF file")?;
        let inf = Inf::from_reader(&mut reader).context("failed to parse INF file")?;
        let config = build_config(&inf, self.theme.clone())?;

        let manifest_path = ctx.package.manifest();
        save_config(&config, &manifest_path, self.overwrite)?;

        writeln!(io::stderr(), "{}", "Cursor.toml created".bold().green()).ok();

        Ok(())
    }
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
    let cursor_files = get_cursor_files(values, strings)?;

    let theme = resolve_cursor_theme(values, strings, theme_override)?;
    let cursors = get_cursors(&cursor_files);

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
fn get_cursor_files(values: &[String], strings: &Section) -> anyhow::Result<Vec<String>> {
    values
        .get(4)
        .context("missing value for cursor files")?
        .split_terminator(',')
        .map(|path| {
            // split always returns a value, even if it's just the original string.
            let file_name = path.split('\\').next_back().unwrap();
            inf::util::expand_vars(file_name, strings).context("failed to expand cursor file value")
        })
        .collect::<anyhow::Result<Vec<_>>>()
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
fn get_cursors(cursor_files: &[String]) -> Vec<Cursor> {
    cursor_files
        .iter()
        .enumerate()
        .map(|(i, file_name)| {
            let cursor = &CURSOR_MAPPING[i];
            let name = cursor.name.to_owned();
            let aliases = cursor.aliases.iter().copied().map(String::from).collect();
            Cursor::new(name, aliases, PathBuf::from(file_name))
        })
        .collect()
}

/// Write `config` to the file at `manifest_path`.
fn save_config(config: &Config, manifest_path: &Path, overwrite: bool) -> anyhow::Result<()> {
    if manifest_path.exists() && !overwrite {
        // TODO: Show a diff of what changed between the two files.
        if !prompt_user("Cursor.toml file already exists. Overwrite it? (y/N): ")? {
            anyhow::bail!("Cursor.toml file already exists");
        }
    }

    let text = toml::to_string(config).expect("expected config to be serializable as TOML");
    fs::write(manifest_path, text).context("failed to create Cursor.toml file")?;
    tracing::info!("created Cursor.toml file at: {:#}", manifest_path.display());

    Ok(())
}

/// Print `prompt` to the user, and wait for user input. `y` to accept, `n` to decline.
fn prompt_user(prompt: &str) -> anyhow::Result<bool> {
    write!(io::stdout(), "{prompt}").ok();

    io::stdout().flush().context("failed to flush stdout")?;

    let mut input = String::new();

    io::stdin()
        .read_line(&mut input)
        .context("failed to read user input")?;

    Ok(matches!(input.trim().to_lowercase().as_ref(), "y" | "yes"))
}

struct CursorInfo {
    name: &'static str,
    aliases: &'static [&'static str],
}

// If your theme builds successfully and a cursor is not showing up as expected, it is likely
// because the names here don't match what your system is looking for when displaying the cursor.
// To fix, you need to find out the target Linux cursor name, then add that name here as an alias.
const CURSOR_MAPPING: [CursorInfo; 17] = [
    // Arrow
    CursorInfo {
        name: "default",
        aliases: &[
            "arrow",
            "dnd-move",
            "left_ptr",
            "move",
            "top_left_arrow",
            "X_cursor",
        ],
    },
    // Help
    CursorInfo {
        name: "help",
        aliases: &["question_arrow"],
    },
    // AppStarting
    CursorInfo {
        name: "progress",
        aliases: &[],
    },
    // Wait
    CursorInfo {
        name: "wait",
        aliases: &["watch"],
    },
    // Crosshair
    CursorInfo {
        name: "crosshair",
        aliases: &["cross", "cross_reverse", "diamond_cross", "tcross"],
    },
    // IBeam
    CursorInfo {
        name: "text",
        aliases: &["xterm"],
    },
    // NWPen
    CursorInfo {
        name: "hand",
        aliases: &[],
    },
    // No
    CursorInfo {
        name: "unavailable",
        aliases: &["not-allowed", "no-drop"],
    },
    // SizeNS
    CursorInfo {
        name: "ns-resize",
        aliases: &[
            "bottom_side",
            "sb_v_double_arrow",
            "top_side",
            "n-resize",
            "s-resize",
        ],
    },
    // SizeWE
    CursorInfo {
        name: "ew-resize",
        aliases: &[
            "left_side",
            "right_side",
            "sb_h_double_arrow",
            "w-resize",
            "e-resize",
        ],
    },
    // SizeNWSE
    CursorInfo {
        name: "nwse-resize",
        aliases: &[
            "bd_double_arrow",
            "bottom_right_corner",
            "top_left_corner",
            "se-resize",
            "nw-resize",
        ],
    },
    // SizeNESW
    CursorInfo {
        name: "nesw-resize",
        aliases: &[
            "bottom_left_corner",
            "fd_double_arrow",
            "top_right_corner",
            "sw-resize",
            "ne-resize",
        ],
    },
    // SizeAll
    CursorInfo {
        name: "move",
        aliases: &["crosshair", "cell", "cross", "tcross"],
    },
    // UpArrow
    CursorInfo {
        name: "alternate",
        aliases: &["alias"],
    },
    // Hand
    CursorInfo {
        name: "link",
        aliases: &["pointer", "hand2"],
    },
    // Location
    CursorInfo {
        name: "pin",
        aliases: &[],
    },
    // Person
    CursorInfo {
        name: "person",
        aliases: &[],
    },
];
