use std::io::Write as _;
use std::process::{Command, Stdio};
use std::{fs, io};

use anyhow::Context as _;
use colored::Colorize as _;
use tracing::error;

use crate::commands::Run;
use crate::context::Context;

#[derive(Debug, Clone, clap::Args)]
pub struct Init {
    #[arg(long)]
    name: Option<String>,
}

impl Init {
    pub fn new(name: Option<String>) -> Self {
        Self { name }
    }
}

impl Run for Init {
    fn run(&self, ctx: &mut Context) -> anyhow::Result<()> {
        let cwd = ctx
            .package
            .path()
            .canonicalize()
            .context("failed to canonicalize package path")?;

        // TODO: Handled in the Python script, but this can really be named anything so it
        // shouldn't be defined here like this. This can be fixed when the Python script is
        // rewritten into Rust. If no INF files are found, copy the template `Cursor.toml` file.
        let install_inf = cwd.join("Install.inf");
        let cursor_toml = cwd.join("Cursor.toml");

        let name = if let Some(ref name) = self.name {
            name
        } else {
            assert!(cwd.is_absolute());

            // The theme can only be unnamed if the cursors are saved in root (`/`).
            cwd.file_name().map_or("unnamed-cursor-theme", |name| {
                name.to_str()
                    // I don't know how to gracefully handle this type of situation,
                    // so if you ever hit this panic, I'd love to know more about it!
                    .expect("expected directory name to be valid unicode")
            })
        };

        let child = Command::new("python3")
            .args([
                "-c",
                include_str!("./init.py"),
                "-vvv",
                "--name",
                name,
                "--input",
                install_inf
                    .to_str()
                    .context("expected path to be valid unicode")?,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("failed to execute python3")?;

        let output = child
            .wait_with_output()
            .context("failed to get output from child process")?;

        if !output.stderr.is_empty() {
            let err = String::from_utf8_lossy(&output.stderr).to_string();
            error!("child process returned an error:\n{err}");
        }

        let text = if output.stdout.is_empty() {
            error!("failed to get output from child process: using default Cursor.toml");
            // bail!("failed to get output from child process");
            include_str!("../../Cursor.toml").to_owned()
        } else {
            String::from_utf8_lossy(&output.stdout).to_string()
        };
        fs::write(&cursor_toml, &text).context("failed to print Cursor.toml contents")?;

        let mut stderr = io::stderr();
        writeln!(stderr, "{}", "Ready!".bold().green())?;

        Ok(())
    }
}
