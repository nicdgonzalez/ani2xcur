use std::io;
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::Context as _;
use colored::Colorize as _;

use crate::commands::Run;
use crate::commands::build::{Build, symlink};
use crate::commands::init::Init;
use crate::config::Config;
use crate::context::Context;

#[derive(Debug, Clone, Default, clap::Args)]
pub struct Install {
    #[clap(long)]
    strict: bool,

    #[clap(long)]
    skip_broken: bool,
}

impl Run for Install {
    fn run(&self, ctx: &mut Context) -> anyhow::Result<()> {
        let package = &ctx.package;

        if !package.manifest().exists() {
            Init::default().run(&mut ctx.clone())?;
        }

        let config = ctx.config.get_or_try_init(|| {
            let path = package.manifest();
            Config::from_file(&path)
        })?;

        Build::new(self.strict, self.skip_broken).run(&mut ctx.clone())?;

        let theme_input = package.build().theme().path();
        let theme_name = config.theme();

        install_theme(theme_input, theme_name)?;
        print_install_instructions(theme_name)?;

        Ok(())
    }
}

fn install_theme(theme_input: &Path, theme_name: &str) -> anyhow::Result<()> {
    let mut theme_output = dirs::data_dir().context("failed to get data directory")?;
    theme_output.extend(["icons", theme_name]);

    if !theme_output.exists() {
        symlink(theme_input, &theme_output)
            .with_context(|| format!("failed to create symlink to {}", theme_output.display()))?;
    }

    Ok(())
}

fn print_install_instructions(theme_name: &str) -> anyhow::Result<()> {
    let mut stderr = io::stderr();
    let mut stdout = io::stdout();

    writeln!(stderr, "{}", "Successfully installed theme!".bold().green())?;
    writeln!(stderr, "Use the following command to set the cursor theme:")?;

    let command = if has_command("gsettings") {
        format!("gsettings set org.gnome.desktop.interface cursor-theme {theme_name:?}")
    } else if has_command("xfconf-query") {
        format!("xfconf-query -c xsettings -p /Gtk/CursorThemeName -s {theme_name:?}")
    } else if has_command("kwriteconfig5") {
        format!("kwriteconfig5 --file kcminputrc --group Mouse --key cursorTheme {theme_name:?}")
    } else {
        "echo 'failed to set cursor theme: no known theme-setting command detected.'".to_owned()
    };

    writeln!(stdout, "  {}", command.cyan())?;
    Ok(())
}

fn has_command(cmd: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {cmd}"))
        .stdout(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}
