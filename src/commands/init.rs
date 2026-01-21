use std::io::Write as _;
use std::path::PathBuf;
use std::{fs, io};

use anyhow::Context as _;
use colored::Colorize as _;
use inf::{Entry, Inf, Value};

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

        // Parse the INF file to identify cursor files.
        let mut reader = fs::File::open(path).context("failed to open INF file")?;
        let inf = Inf::from_reader(&mut reader).context("failed to parse INF file")?;

        let strings = inf
            .sections()
            .iter()
            .find(|section| "strings" == section.name().to_lowercase())
            .context("expected 'Strings' section to exist")?;

        // TODO: After some research, I think the best flow would be:
        //
        // Check [DefaultInstall]
        // -> entry key: AddReg
        // -> Go to each section listed and look for: subkey == "Control Panel\Cursors\Schemes"
        // -> Parse 5th value
        //
        // <https://learn.microsoft.com/en-us/windows-hardware/drivers/install/inf-addreg-directive>
        let scheme_reg = inf
            .sections()
            .iter()
            .find(|section| "scheme.reg" == section.name().to_lowercase())
            .context("expected 'Scheme.Reg' section to exist")?;

        let Some(Entry::Value(Value::List(entry))) = scheme_reg.entries().first() else {
            anyhow::bail!("failed to get necessary entry from INF file");
        };

        let cursor_files = entry
            .get(4)
            .context("expected AddReg-referenced section to have entries with 5 or more values")?
            .split_terminator(',')
            .map(|path| {
                // Split always returns a value, even if it's just the original string.
                let file_name = path.split('\\').next_back().unwrap();
                inf::util::expand_vars(file_name, strings).context("failed to expand value")
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        // Generate Cursor.toml
        let theme = self.theme.as_deref().map_or_else(
            || {
                entry
                    .get(2)
                    .and_then(|scheme_name| inf::util::expand_vars(scheme_name, strings).ok())
                    .expect("failed to get default scheme name from the INF file")

                // We could further fallback to the directory name if we aren't able to determine
                // the scheme name from the INF file, though, I feel this doesn't fit right with
                // the goals of this subcommand.

                // .unwrap_or_else(|| {
                //     ctx.package
                //         .as_path()
                //         .file_name()
                //         .and_then(|f| f.to_str())
                //         .expect("package path has a valid UTF-8 file name")
                //         .to_owned()
                // })
            },
            str::to_owned,
        );
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
        let config = Config::new(theme, cursors);

        // Write to disk
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

        writeln!(io::stderr(), "{}", "Cursor.toml created".bold().green()).ok();

        Ok(())
    }
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
