
#[cfg(test)]
use std::path::PathBuf;

use crate::parse::Binary;

#[derive(Debug, Clone)]
pub enum Deps {
    Plain,
    Binary(Binary),

    #[cfg(test)]
    Mock {
        paths: Vec<PathBuf>,
    },
}
