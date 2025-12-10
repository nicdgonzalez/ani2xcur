use std::cell::OnceCell;

use crate::config::Config;
use crate::package::Package;
use crate::verbosity::VerbosityLevel;

#[derive(Debug, Clone)]
pub struct Context {
    pub config: OnceCell<Config>,
    pub package: Package,
    pub level: VerbosityLevel,
}

impl Context {
    pub fn new(package: Package, level: VerbosityLevel) -> Self {
        Self {
            config: OnceCell::new(),
            package,
            level,
        }
    }
}
