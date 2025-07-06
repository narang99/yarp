use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use crate::parse::Macho;

#[derive(Debug, Clone)]
pub enum Binary {
    Macho(Macho),
}

#[derive(Debug, Clone)]
pub enum Deps {
    Plain,
    Binary(Binary),

    #[cfg(test)]
    Mock {
        paths: Vec<PathBuf>,
    },
}
