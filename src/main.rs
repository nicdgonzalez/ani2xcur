#![doc = include_str!("../README.md")]
#![warn(
    clippy::correctness,
    clippy::suspicious,
    clippy::complexity,
    clippy::perf,
    clippy::style,
    clippy::pedantic
)]

mod commands;
mod config;
mod context;
mod cursors;
mod package;

use std::io::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;
use std::{env, io};

use anyhow::Context as _;
use clap::Parser as _;
use colored::Colorize as _;

use crate::context::Context;
use crate::package::Package;

/// Convert Windows animated cursors to Unix-like systems running the X Window System.
#[derive(clap::Parser)]
#[clap(version)]
pub struct Parser {
    #[command(subcommand)]
    subcommand: commands::Subcommand,

    /// Change to the specified directory prior to running the command.
    #[clap(long, global = true)]
    directory: Option<PathBuf>,

    #[clap(flatten)]
    verbosity: clap_verbosity_flag::Verbosity,
}

fn main() -> ExitCode {
    try_main().unwrap_or_else(|err| {
        let mut stderr = io::stderr().lock();
        writeln!(stderr, "{}", "ani2xcur failed".bold().red()).ok();

        for cause in err.chain() {
            writeln!(stderr, "  {}: {}", "Cause".bold(), cause).ok();
        }

        ExitCode::FAILURE
    })
}

fn try_main() -> anyhow::Result<ExitCode> {
    let args = Parser::parse();
    tracing_subscriber::fmt()
        .with_max_level(args.verbosity)
        .with_writer(io::stderr)
        .init();

    let directory = if let Some(path) = args.directory {
        path
    } else {
        env::current_dir().context("failed to get current directory")?
    };

    let mut ctx = Context {
        package: Package::new(directory),
    };

    args.subcommand.run(&mut ctx).map(|()| ExitCode::SUCCESS)
}
