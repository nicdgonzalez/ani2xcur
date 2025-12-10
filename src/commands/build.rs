use std::fmt::Write as _;
use std::fs::File;
use std::io::{self, ErrorKind, Write as _};
use std::path::Path;
use std::process::Command;
use std::{fs, iter, path, thread};

use ani::de::{Ani, JIFFY};
use anyhow::{anyhow, Context as _};
use colored::Colorize as _;
use tracing::{error, error_span, info};

use crate::commands::Run;
use crate::config::{Config, Cursor};
use crate::context::Context;
use crate::package::{Build as BuildDir, Package};
use crate::verbosity::VerbosityLevel;

#[derive(Debug, Clone, Default, clap::Args)]
pub struct Build {
    /// Throw an error for ANI files that do not strictly follow the ANI file format.
    #[clap(long)]
    strict: bool,

    /// Ignore cursors that fail to build.
    #[clap(long)]
    skip_broken: bool,
}

impl Build {
    pub fn new(strict: bool, skip_broken: bool) -> Self {
        Self {
            strict,
            skip_broken,
        }
    }
}

impl Run for Build {
    fn run(&self, ctx: &mut Context) -> anyhow::Result<()> {
        let package = &ctx.package;

        let config = ctx.config.get_or_try_init(|| {
            let path = package.config();
            Config::from_file(&path)
        })?;

        setup_build_directory(package.build(), config.theme())?;

        let handles = config
            .cursors()
            .to_owned()
            .into_iter()
            .map(|cursor| {
                // Attach context so we know which thread is emitting the events.
                let span = error_span!("", cursor = ?cursor.name());

                let package = package.clone();
                let name = cursor.name().to_owned();
                let strict = self.strict;

                let handle = thread::spawn(move || {
                    span.in_scope(move || process_cursor(&cursor, &package, strict))
                });

                (name, handle)
            })
            .collect::<Vec<_>>();

        let mut error_count = 0;
        for (name, handle) in handles {
            match handle.join() {
                Ok(result) => {
                    if let Err(err) = result {
                        let mut error_message = err.to_string();

                        if ctx.level >= VerbosityLevel::Verbose {
                            error_message.push('\n');

                            for cause in err.chain() {
                                _ = writeln!(error_message, "  Cause: {cause}");
                            }
                        }

                        error!("failed to process cursor: {name}: {error_message}");
                        error_count += 1;
                    }
                }
                Err(err) => {
                    // The thread most likely panicked.
                    error!("failed to join on the associated thread: {err:#?}");
                    error_count += 1;
                }
            }
        }

        if error_count > 0 && !self.skip_broken {
            Err(anyhow!("failed to create ({error_count}) cursors"))
        } else {
            let mut stderr = io::stderr();
            writeln!(stderr, "{}", "Successfully built theme!".bold().green())?;

            Ok(())
        }
    }
}

fn setup_build_directory(build: &BuildDir, theme_name: &str) -> anyhow::Result<()> {
    fs::create_dir_all(build.path()).context("failed to create build directory")?;
    info!("created directory: {:#}", build.path().display());

    let frames = build.frames();
    fs::create_dir_all(&frames).context("failed to create frames directory")?;
    info!("created directory: {:#}", frames.display());

    let theme = build.theme();
    fs::create_dir_all(theme.path()).context("failed to create theme directory")?;
    info!("created directory: {:#}", theme.path().display());

    let cursors = theme.cursors();
    fs::create_dir_all(&cursors).context("failed to create theme directory")?;
    info!("created directory: {:#}", cursors.display());

    let index_theme = theme.index_theme();
    let contents = format!(
        "[Icon Theme]\n\
        Name = {theme_name}\n\
        Inherits = Adwaita"
    );
    fs::write(&index_theme, &contents).context("failed to create index.theme file")?;
    info!("created file: {:#}", index_theme.display());

    Ok(())
}

fn process_cursor(cursor: &Cursor, package: &Package, strict: bool) -> anyhow::Result<()> {
    let path = path::absolute(package.path().join(cursor.input()))
        .context("failed to resolve cursor input path")?;
    let ani = Ani::open(&path, strict).context("failed to decode ANI file")?;

    let file_stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .context("expected path to be valid unicode")?;

    let build = package.build();
    let mut frames_dir = build.frames();
    frames_dir.push(file_stem);
    let frames_dir = frames_dir;
    fs::create_dir_all(&frames_dir).context("failed to create frame output directory")?;

    let frame_names = extract_frames(&ani, &frames_dir)?;

    let cursor_config_path = frames_dir.join(format!("{file_stem}.cursor"));
    build_xcursor_config(&ani, &frame_names, &cursor_config_path)?;

    let xcursor_output = frames_dir.join(file_stem);
    create_xcursor(&frames_dir, &cursor_config_path, &xcursor_output)
        .context("failed to create Xcursor")?;

    link_to_theme(
        &build.theme().cursors(),
        cursor.name(),
        cursor.aliases(),
        &xcursor_output,
    )?;

    Ok(())
}

fn extract_frames(ani: &Ani, output_dir: &Path) -> anyhow::Result<Vec<Vec<String>>> {
    let mut names = Vec::with_capacity(ani.frames().len());

    // TODO: (See also todo in `build_xcursor_config`):
    // Maybe sort PNGs by size to make it easier to bulk delete undesired cursors?

    for (i, frame) in ani.frames().iter().enumerate() {
        let mut size_names = Vec::with_capacity(frame.len());
        for image in frame {
            let width = image.width();
            let name = format!("{i:0>2}-{width}.png");
            let path = output_dir.join(&name);

            let file = File::create(&path)?;

            image.write_png(&file)?;
            size_names.push(name);
        }
        names.push(size_names);
    }

    Ok(names)
}

#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn build_xcursor_config(
    ani: &Ani,
    frame_names: &[Vec<String>],
    output: &Path,
) -> anyhow::Result<()> {
    let sequence = ani.sequence().map_or_else(
        || {
            info!("ANI sequence missing, using default");
            (0..ani.header().steps())
                .map(|i| i % ani.header().frames())
                .collect()
        },
        ToOwned::to_owned,
    );
    let rates = ani.rates().map_or_else(
        || {
            info!("ANI frame rates missing, using default");
            iter::repeat_n(ani.header().jif_rate(), ani.frames().len()).collect()
        },
        ToOwned::to_owned,
    );

    let mut contents = String::new();

    // TODO: Sort the entries by size.
    // Right now, the `.cursor` file looks like:
    //
    // 00-32.png
    // 00-48.png
    // 00-64.png
    // 01-32.png
    // 01-48.png
    // 01-64.png
    //
    // Instead, I want:
    //
    // 00-32.png
    // 01-32.png
    //
    // 00-48.png
    // 01-48.png
    //
    // 00-64.png
    // 01-64.png
    //
    // This way, you can delete a whole group of cursors very easily if you wanted to.
    //
    // (Maybe also group them when saving them as well...)

    for i in sequence {
        let i = usize::try_from(i).context("invalid sequence index")?;
        let frame = &ani.frames()[i];

        for (j, entry) in frame.iter().enumerate() {
            let size = entry.width();
            let (x, y) = entry.cursor_hotspot().unwrap_or((0, 0));
            let file_name = &frame_names[i][j];
            let duration = rates[i] * (JIFFY.round() as u32);

            writeln!(contents, "{size} {x} {y} {file_name} {duration}",)?;
        }
    }

    fs::write(output, contents).context("failed to create Xcursor configuration file")?;
    Ok(())
}

fn create_xcursor(frames_dir: &Path, config: &Path, output: &Path) -> anyhow::Result<()> {
    let status = Command::new("xcursorgen")
        .args([config.display().to_string(), output.display().to_string()])
        .current_dir(frames_dir)
        .status()
        .context("failed to execute xcursorgen")?;

    match status.code() {
        Some(0) => {
            info!("created Xcursor: {:#}", output.display());
            Ok(())
        }
        Some(code) => Err(anyhow!("process failed with exit code: {code}")),
        None => Err(anyhow!("process terminated due to signal")),
    }
}

fn link_to_theme(
    theme_cursors_dir: &Path,
    cursor_name: &str,
    aliases: &[String],
    target: &Path,
) -> anyhow::Result<()> {
    let target_link = theme_cursors_dir.join(cursor_name);

    // TODO: Remove todo when `symlink()` doesn't use `fs::remove_file` --
    // Path.exists() is weird about symbolic links... investigate reasons why this is failing.
    if !target_link.exists() {
        symlink(target, &target_link)?;
    }

    for alias in aliases {
        let alias_link = theme_cursors_dir.join(alias);

        if alias_link.exists() {
            continue;
        }

        symlink(&target_link, &alias_link)?;
        info!("created alias: {alias}");
    }

    Ok(())
}

pub fn symlink(source: &Path, target: &Path) -> anyhow::Result<()> {
    match fs::remove_file(target) {
        Ok(()) => {}
        Err(err) => match err.kind() {
            ErrorKind::NotFound => {}
            _ => return Err(err).context("failed to remove existing file")?,
        },
    }

    let status = Command::new("ln")
        .args([
            "--symbolic",
            &source.display().to_string(),
            &target.display().to_string(),
        ])
        .status()
        .context("failed to execute ln")?;

    match status.code() {
        Some(0) => Ok(()),
        Some(code) => Err(anyhow!("process failed with exit code: {code}")),
        None => Err(anyhow!("process terminated due to signal")),
    }
}
