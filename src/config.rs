use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::Context as _;

pub fn default_sizes() -> Vec<Size> {
    vec![Size(32), Size(48), Size(64), Size(96)]
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Config {
    theme: String,

    #[serde(default = "default_sizes")]
    sizes: Vec<Size>,

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
    pub fn new(theme: String, sizes: Vec<Size>, cursors: Vec<Cursor>) -> Self {
        Self {
            theme,
            sizes,
            cursors,
        }
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
    pub fn sizes(&self) -> &[Size] {
        &self.sizes
    }

    #[must_use]
    pub fn cursors(&self) -> &[Cursor] {
        &self.cursors
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub struct Size(pub u8);

impl Size {
    pub const VALID_VALUES: &[u8] = &[24, 32, 48, 64, 96];
}

impl<'de> serde::Deserialize<'de> for Size {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as SerdeError;

        let value = u8::deserialize(deserializer)?;

        if Self::VALID_VALUES.contains(&value) {
            Ok(Self(value))
        } else {
            Err(SerdeError::custom(format!(
                "invalid cursor size {value}, allowed values: {:?}",
                Self::VALID_VALUES
            )))
        }
    }
}

impl FromStr for Size {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = s
            .parse::<u8>()
            .map_err(|_| anyhow::anyhow!("{s:?} is not a valid number"))?;

        if Self::VALID_VALUES.contains(&value) {
            Ok(Size(value))
        } else {
            Err(anyhow::anyhow!(
                "invalid cursor size {value}, allowed values: {:?}",
                Self::VALID_VALUES
            ))
        }
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
