use std::{
    ffi::OsStr,
    fmt::Display,
    hash::{Hash, Hasher},
    path::PathBuf,
};

use anyhow::Result;

use crate::{digest::make_digest, manifest::Version, node::deps::Deps};

pub mod deps;

#[derive(Debug, Clone)]
pub struct PkgSitePackages {
    // original site-packages path
    pub site_packages: PathBuf,
    // to prevent collisions, we create an alias which is the name of this site-packages destination in dist
    pub alias: String,
    // the path relative to site-packages path, we simply copy data from node to this path inside alias in dist
    pub rel_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct PrefixPackages {
    pub original_prefix: PathBuf,

    pub rel_path: PathBuf,

    pub version: Version,
}

#[derive(Debug, Clone)]
pub enum Pkg {
    SitePackagesPlain(PkgSitePackages),
    SitePackagesBinary(PkgSitePackages),

    ExecPrefixPlain(PrefixPackages),
    ExecPrefixBinary(PrefixPackages),
    PrefixPlain(PrefixPackages),
    PrefixBinary(PrefixPackages),

    Executable,
    Binary,
    BinaryInLDPath { symlinks: Vec<String> },
    Plain,
}

impl Pkg {
    pub fn from_path(path: &PathBuf) -> Pkg {
        // uses simple heuristics to find the packager for a path
        // it would be either of binary or plain, as we don't have context of any site-packages
        // it is preferred to pass the correct Pkg manually (you might have your own heuristics)
        // create enum variant yourself and return it
        let ext = path.extension();
        match ext {
            None => Pkg::Plain,
            Some(ext) => {
                if ext == "so" || ext == "dylib" {
                    Pkg::Binary
                } else {
                    Pkg::Plain
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub path: PathBuf,

    pub deps: Deps,

    pub pkg: Pkg,

    pub sha: String,
}

impl Node {
    pub fn name(&self) -> Option<&OsStr> {
        self.path.file_name()
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Eq for Node {}

impl Hash for Node {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.path.hash(state);
    }
}

impl Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Node{{")?;
        write!(f, "{}", self.path.display())?;
        write!(f, "}}")
    }
}

impl Node {
    pub fn new(path: PathBuf, pkg: Pkg, deps: Deps) -> Result<Node> {
        let sha = make_digest(&path)?;
        Ok(Node {
            path,
            deps,
            pkg,
            sha,
        })
    }

    #[cfg(test)]
    pub fn mock(path: PathBuf, deps: Vec<PathBuf>) -> Result<Node> {
        let sha = make_digest(&path)?;
        Ok(Node {
            path,
            deps: Deps::mock(deps),
            pkg: Pkg::Binary,
            sha,
        })
    }
}