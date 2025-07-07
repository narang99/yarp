use std::{collections::HashMap, path::PathBuf};

use anyhow::{Result, bail};
use log::info;

pub use crate::factory::core::Factory;

use crate::{
    digest::make_digest,
    factory::{
        deps::create_deps,
        pkg::{get_exec_prefix_pkg, get_prefix_pkg, get_site_packages_pkg},
    },
    manifest::Version,
    node::{Node, Pkg, deps::Deps},
    paths::normalize_path,
    pkg::paths::is_shared_library,
    site_pkgs::SitePkgs,
};

#[derive(Debug, Clone)]
pub struct NodeFactory {
    site_pkgs: SitePkgs,
    version: Version,
    executable: PathBuf,
    cwd: PathBuf,
    env: HashMap<String, String>,
    skip_prefixes: Vec<PathBuf>,
}

impl NodeFactory {
    pub fn new(
        site_pkgs: SitePkgs,
        version: Version,
        executable: PathBuf,
        cwd: PathBuf,
        env: HashMap<String, String>,
        skip_prefixes: Vec<PathBuf>,
    ) -> NodeFactory {
        Self {
            site_pkgs,
            version,
            executable: executable,
            cwd: cwd,
            env,
            skip_prefixes,
        }
    }
}

impl NodeFactory {
    fn create_deps(&self, path: &PathBuf, known_libs: &HashMap<String, PathBuf>) -> Result<Deps> {
        create_deps(path, &self.executable, &self.cwd, &self.env, known_libs)
    }

    fn should_skip(&self, path: &PathBuf) -> bool {
        for prefix in &self.skip_prefixes {
            if path.starts_with(prefix) {
                return true;
            }
        }
        false
    }
}

impl Factory for NodeFactory {
    fn make_with_symlinks(
        &self,
        path: &PathBuf,
        symlinks: &Vec<String>,
        known_libs: &HashMap<String, PathBuf>,
    ) -> Result<Option<Node>> {
        if self.should_skip(path) {
            info!("skip: {}", path.display());
            return Ok(None)
        }
        if !is_shared_library(path) {
            bail!(
                "cannot make_with_symlinks if the path is not a shared library, path={}",
                path.display()
            );
        }
        Ok(Some(Node::new(
            path.clone(),
            Pkg::BinaryInLDPath {
                symlinks: symlinks.clone(),
                sha: make_digest(path)?,
            },
            self.create_deps(path, known_libs)?,
        )?))
    }

    fn make(&self, path: &PathBuf, known_libs: &HashMap<String, PathBuf>) -> Result<Option<Node>> {
        let p = normalize_path(path);
        if self.should_skip(path) {
            info!("skip: {}", path.display());
            return Ok(None)
        }
        if !p.exists() {
            bail!(
                "tried creating node from path={}, but the path does not exist",
                path.display()
            );
        }

        if p.starts_with(&self.site_pkgs.lib_dynload) {
            return Ok(Some(Node::new(
                path.clone(),
                get_exec_prefix_pkg(path, &self.site_pkgs.lib_dynload, &self.version)?,
                self.create_deps(path, known_libs)?,
            )?));
        }

        if p.starts_with(&self.site_pkgs.stdlib) {
            return Ok(Some(Node::new(
                path.clone(),
                get_prefix_pkg(path, &self.site_pkgs.stdlib, &self.version)?,
                self.create_deps(path, known_libs)?,
            )?));
        }

        for (site_pkg, alias) in &self.site_pkgs.site_pkg_by_alias {
            if p.starts_with(site_pkg) {
                return Ok(Some(Node::new(
                    path.clone(),
                    get_site_packages_pkg(path, site_pkg, alias, &self.version)?,
                    self.create_deps(path, known_libs)?,
                )?));
            }
        }

        if !is_shared_library(path) {
            bail!(
                "found a path which is not inside site packages and is not a shared library. Only plain files inside site-packages are allowed, path={}",
                path.display()
            );
        }

        Ok(Some(Node::new(
            path.clone(),
            Pkg::Binary {
                sha: make_digest(path)?,
            },
            self.create_deps(path, known_libs)?,
        )?))
    }

    fn make_py_executable(&self, path: &PathBuf) -> Result<Node> {
        Node::new(
            path.clone(),
            Pkg::Executable,
            self.create_deps(path, &HashMap::new())?,
        )
    }
}
