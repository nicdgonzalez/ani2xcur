use std::fs::{self, File};
use std::io;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use ani::{Ani, Image, JIFFY};
use anyhow::Context as _;
use colored::Colorize as _;

use crate::commands::prelude::*;
use crate::package::Package;

#[derive(Debug, Default, clap::Args)]
pub struct Convert {
    input: PathBuf,
}

struct Extracted<'a> {
    image: &'a Image,
    path: PathBuf,
}

struct XcursorEntry {
    size: u32,
    x: u16,
    y: u16,
    path: PathBuf,
    duration: u64,
}

impl Run for Convert {
    fn run(self, ctx: &mut Context) -> anyhow::Result<()> {
        let path = ctx.package.as_path().join(&self.input);
        let xcursor = convert_cursor(&path, &ctx.package).context("failed to create Xcursor")?;

        writeln!(
            io::stderr(),
            "{}: {:#}",
            "Created Xcursor".bold().green(),
            xcursor.display()
        )
        .ok();

        Ok(())
    }
}

// TODO: Figure out a way to decouple some of this code - This function is doing a lot because
// it controls a lot of how the output directory is structured.
/// Convert from ANI to Xcursor.
pub(crate) fn convert_cursor(input: &Path, package: &Package) -> anyhow::Result<PathBuf> {
    let ani = Ani::open(input).context("failed to decode ANI file")?;

    // To keep everything organized, we'll reuse the original input file name (minus the extension)
    // for our generated files and directories.
    let file_stem = input
        .file_stem()
        .expect("expected file name to exist")
        .to_str()
        .context("expected file name to be valid unicode")?;

    // Extract animation frames
    let build = package.build();
    debug_assert!(build.as_path().try_exists().is_ok_and(|exists| exists));

    let mut output_dir = build.frames();
    debug_assert!(output_dir.try_exists().is_ok_and(|exists| exists));

    output_dir.push(file_stem);
    fs::create_dir_all(&output_dir).context("failed to create frame output directory")?;

    let frames = extract_frames(ani.frames(), &output_dir).collect::<Vec<_>>();

    // Save frames to the file system
    debug_assert!(output_dir.try_exists().is_ok_and(|exists| exists));

    for frame in &frames {
        for variant in frame {
            let writer = File::create(&variant.path)?;
            tracing::info!("created file: {:#}", variant.path.display());
            variant.image.write_png(writer)?;
        }
    }

    // Generate the xcursorgen configuration file
    let config = output_dir.join(format!("{file_stem}.cursor"));
    create_xcursorgen_configuration(&ani, &frames, &config)
        .context("failed to create xcursorgen configuration file")?;

    // Generate Xcursor
    let xcursor = output_dir.join(file_stem);
    call_xcursorgen(&output_dir, &config, &xcursor).context("failed to create xcursor")?;

    Ok(xcursor)
}

fn extract_frames<'a>(
    frames: &'a [Vec<Image>],
    output_dir: &Path,
) -> impl Iterator<Item = Vec<Extracted<'a>>> {
    frames.iter().enumerate().map(move |(i, frame)| {
        frame
            .iter()
            .map(move |image| {
                let width = image.width();
                let file_name = format!("{i:02}-{width}.png");
                let path = output_dir.join(&file_name);
                Extracted { image, path }
            })
            .collect::<Vec<_>>()
    })
}

fn create_xcursorgen_configuration(
    ani: &Ani,
    frames: &[Vec<Extracted<'_>>],
    output: &Path,
) -> anyhow::Result<()> {
    let rates = ani.rates_or_default();
    let sequence = ani.sequence_or_default();
    let mut entries = get_xcursorgen_entries(&sequence, &rates, frames)?;
    entries.sort_by_key(|entry| entry.size);

    let contents = entries
        .into_iter()
        .map(|entry| {
            format!(
                "{size} {x} {y} {file_name} {duration}",
                size = entry.size,
                x = entry.x,
                y = entry.y,
                file_name = entry
                    .path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .expect("code-generated file name is not valid unicode"),
                duration = entry.duration
            )
        })
        .collect::<Vec<String>>()
        .join("\n");

    fs::write(output, contents).context("failed to write xcursorgen configuration file")?;

    Ok(())
}

fn get_xcursorgen_entries(
    sequence: &[u32],
    rates: &[u32],
    frames: &[Vec<Extracted<'_>>],
) -> anyhow::Result<Vec<XcursorEntry>> {
    let mut entries = Vec::new();

    for &i in sequence {
        let index = usize::try_from(i).expect("u32 overflowed usize");
        let frame = frames
            .get(index)
            .with_context(|| format!("frame not found at index {index}"))?;

        for variant in frame {
            // TODO: There is no guarantee that all frames contain the same number of variants,
            // which should result in an error since it affects the animation in an unexpected way.
            let size = variant.image.width();
            let (x, y) = variant.image.cursor_hotspot().unwrap_or((0, 0));
            let path = variant.path.clone();

            let rate = *rates
                .get(index)
                .with_context(|| format!("rate not found at index {index}"))?;
            // TODO: It would be more accurate to multiply as f32s and then convert to u64 after.
            let duration = u64::from(JIFFY.round() as u32) * u64::from(rate);

            entries.push(XcursorEntry {
                size,
                x,
                y,
                path,
                duration,
            });
        }
    }

    Ok(entries)
}

fn call_xcursorgen(current_dir: &Path, config_path: &Path, target: &Path) -> anyhow::Result<()> {
    let status = Command::new("xcursorgen")
        .args([config_path, target])
        .current_dir(current_dir)
        .status()
        .context("failed to execute xcursorgen")?;

    match status.code() {
        Some(0) => Ok(()),
        Some(code) => Err(anyhow::anyhow!("process failed with exit code: {code}")),
        None => Err(anyhow::anyhow!("process terminated due to signal")),
    }
}
