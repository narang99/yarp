use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Macho {
    // all load commands, along with the resolved path of the dependency
    pub load_cmds: HashMap<String, PathBuf>,

    // all rpaths, along with resolved rpath
    pub rpaths: HashMap<String, PathBuf>,

    // the current id of the dylib
    pub id_dylib: Option<String>,

    // path to the lib
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub enum BinaryParseError {
    UnsupportedArchitecture,
    NotBinary,
}

impl fmt::Display for BinaryParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryParseError::UnsupportedArchitecture => {
                write!(f, "Unsupported architecture")
            },
            BinaryParseError::NotBinary => {
                write!(f, "Not a binary")
            }
        }
    }
}

impl std::error::Error for BinaryParseError {}

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
