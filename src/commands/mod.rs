mod build;
mod init;
mod install;

use crate::context::Context;

pub trait Run {
    fn run(&self, ctx: &mut Context) -> anyhow::Result<()>;
}

#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    /// Generate the `Cursor.toml` configuration file
    Init(init::Init),

    /// Create the cursor theme
    Build(build::Build),

    /// Symlink the cursor theme to `$HOME/.local/share/icons`.
    Install(install::Install),
}

impl Subcommand {
    pub fn run(&self, ctx: &mut Context) -> anyhow::Result<()> {
        let handler: &dyn Run = match *self {
            Self::Init(ref inner) => inner,
            Self::Build(ref inner) => inner,
            Self::Install(ref inner) => inner,
        };

        handler.run(ctx)
    }
}

pub mod prelude {
    pub use super::Run;
    pub use crate::context::Context;
}
