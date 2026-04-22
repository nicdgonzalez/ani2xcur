use crate::package::Package;

#[derive(Debug, Clone)]
pub struct Context {
    /// Represents a Cursor Scheme and all of it's related files.
    pub package: Package,
}
