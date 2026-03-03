use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::Context as _;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Config {
    theme: String,

    #[serde(rename = "cursor")]
    cursors: Vec<Cursor>,
}

impl FromStr for Config {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        toml::from_str(s).context("failed to parse configuration")
    }
}

impl Config {
    #[must_use]
    pub fn new(theme: String, cursors: Vec<Cursor>) -> Self {
        Self { theme, cursors }
    }

    pub fn from_path(path: &Path) -> anyhow::Result<Self> {
        let contents = fs::read_to_string(path).context("failed to read configuration file")?;
        contents.parse()
    }

    #[must_use]
    pub fn theme(&self) -> &str {
        &self.theme
    }

    #[must_use]
    pub fn cursors(&self) -> &[Cursor] {
        &self.cursors
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Cursor {
    name: String,

    #[serde(default = "Vec::new")]
    aliases: Vec<String>,

    input: PathBuf,
}

impl Cursor {
    #[must_use]
    pub fn new(name: String, aliases: Vec<String>, input: PathBuf) -> Self {
        Self {
            name,
            aliases,
            input,
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn aliases(&self) -> &[String] {
        &self.aliases
    }

    #[must_use]
    pub fn input(&self) -> &Path {
        &self.input
    }
}
