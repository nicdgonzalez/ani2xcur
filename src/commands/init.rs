use std::io::Write as _;
use std::path::PathBuf;
use std::{fs, io};

use anyhow::Context as _;
use colored::Colorize as _;
use inf::{Entry, Inf, Section, Value};

use crate::commands::prelude::*;
use crate::config::{Config, Cursor};

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
        let config = build_config_from_inf(&inf, self.theme.clone())?;

        // --8<--- Refactor ---

        let manifest_path = ctx.package.manifest();

        if manifest_path.exists() && !self.overwrite {
            // TODO: Show a diff of what changed between the two files.
            if !prompt_user("Cursor.toml file already exists. Overwrite it? (y/N): ")? {
                anyhow::bail!("Cursor.toml file already exists");
            }
        }

        let text = toml::to_string(&config).expect("expected config to be serializable as TOML");
        fs::write(&manifest_path, text).context("failed to create Cursor.toml file")?;
        tracing::info!("created Cursor.toml file at: {:#}", manifest_path.display());

        // ---

        writeln!(io::stderr(), "{}", "Cursor.toml created".bold().green()).ok();

        Ok(())
    }
}

fn build_config_from_inf(inf: &Inf, theme_override: Option<String>) -> anyhow::Result<Config> {
    let strings = inf
        .get("Strings")
        .context("expected 'Strings' section in INF")?;

    let default_install = inf
        .get("DefaultInstall")
        .context("expected 'DefaultInstall' section in INF")?;

    let section_names = get_section_names(default_install)?;
    let values = get_cursor_scheme_entry(inf, section_names)?;
    let cursor_files = get_cursor_files(values, strings)?;

    // --8<--- Refactor ---

    // Generate Cursor.toml
    let theme = if let Some(theme) = theme_override {
        theme
    } else {
        let value = values.get(2).context("missing value for scheme name")?;
        inf::util::expand_vars(value, strings).context("failed to expand scheme name value")?
    };

    let cursors = cursor_files
        .iter()
        .enumerate()
        .map(|(i, file_name)| {
            let cursor = &CURSOR_MAPPING[i];
            let name = cursor.name.to_owned();
            let aliases = cursor.aliases.iter().copied().map(String::from).collect();
            Cursor::new(name, aliases, PathBuf::from(file_name))
        })
        .collect::<Vec<Cursor>>();

    // ---

    Ok(Config::new(theme, cursors))
}

fn get_section_names(section: &Section) -> anyhow::Result<&[String]> {
    section
        .entries()
        .iter()
        .find_map(|entry| match entry {
            Entry::Item(key, v) if key.as_str() == "AddReg" => match v {
                Value::Raw(value) => Some(std::slice::from_ref(value)),
                Value::List(values) => Some(values.as_slice()),
            },
            _ => None,
        })
        .context("expected 'AddReg' directive in 'DefaultInstall' section")
}

fn get_cursor_scheme_entry<'a>(
    inf: &'a Inf,
    section_names: &[String],
) -> anyhow::Result<&'a [String]> {
    section_names
        .iter()
        .find_map(|name| {
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
        .context("failed to find section in 'AddReg' with the cursor scheme definition")
}

fn get_cursor_files(values: &[String], strings: &Section) -> anyhow::Result<Vec<String>> {
    values
        .get(4)
        .context("not enough values in AddReg-referenced section")?
        .split_terminator(',')
        .map(|path| {
            // split always returns a value, even if it's just the original string.
            let file_name = path.split('\\').next_back().unwrap();
            inf::util::expand_vars(file_name, strings).context("failed to expand value")
        })
        .collect::<anyhow::Result<Vec<_>>>()
}

fn prompt_user(prompt: &str) -> anyhow::Result<bool> {
    write!(io::stdout(), "{prompt}").ok();

    io::stdout().flush().context("failed to flush stdout")?;

    let mut input = String::new();

    io::stdin()
        .read_line(&mut input)
        .context("failed to read user input")?;

    Ok(matches!(input.trim().to_lowercase().as_ref(), "y" | "yes"))
}
