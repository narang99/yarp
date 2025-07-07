use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;
use serde::de::Expected;

use crate::{manifest::Version, node::Node};

pub trait Factory {
    fn make_spec(&self, path: &PathBuf) -> Result<NodeSpec>;

    fn make(&self, path: &PathBuf, known_libs: &HashMap<String, PathBuf>) -> Result<Node> {
        let spec = self.make_spec(path)?;
        self.make_from_spec(&spec, known_libs)
    }

    fn make_from_spec(&self, spec: &NodeSpec, known_libs: &HashMap<String, PathBuf>) -> Result<Node>;
    fn make_with_symlinks(
        &self,
        path: &PathBuf,
        symlinks: &Vec<String>,
        known_libs: &HashMap<String, PathBuf>,
    ) -> Result<Node>;
}


#[derive(Debug, Clone)]
pub enum NodeSpec {
    Executable {
        path: PathBuf,
    },
    ExecPrefixPkg {
        original_prefix: PathBuf,
        version: Version,
        path: PathBuf,
    },
    PrefixPkg {
        original_prefix: PathBuf,
        version: Version,
        path: PathBuf,
    },
    SitePkg {
        site_pkg_path: PathBuf,
        alias: String,
        version: Version,
        path: PathBuf,
    },
    BinaryInLdPath {
        path: PathBuf,
        symlinks: Vec<String>,
    },
    Binary {
        path: PathBuf,
    },
}

impl NodeSpec {
    pub fn path(&self) -> &PathBuf {
        match self {
            NodeSpec::ExecPrefixPkg { original_prefix, version, path } => path,
            NodeSpec::Executable { path } => path,
            NodeSpec::PrefixPkg { original_prefix, version, path } => path,
            NodeSpec::SitePkg { site_pkg_path, alias, version, path } => path,
            NodeSpec::Binary { path } => path,
            NodeSpec::BinaryInLdPath { path, symlinks } => path,
        }
    }
}