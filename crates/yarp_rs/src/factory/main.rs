use std::{collections::HashMap, path::PathBuf};

use anyhow::{Result, anyhow, bail};

pub use crate::factory::core::{Factory, NodeSpec};

use crate::{
    factory::spec::generate_node,
    manifest::{Version, YarpManifest},
    node::Node,
    paths::normalize_path,
    pkg::paths::is_shared_library,
    site_pkgs::SitePkgs,
};

#[derive(Debug)]
pub struct NodeFactory {
    site_pkgs: SitePkgs,
    version: Version,
    executable: PathBuf,
    cwd: PathBuf,
    env: HashMap<String, String>,
}

impl NodeFactory {
    pub fn new(
        site_pkgs: SitePkgs,
        version: Version,
        executable: PathBuf,
        cwd: PathBuf,
        env: HashMap<String, String>,
    ) -> NodeFactory {
        Self {
            site_pkgs,
            version,
            executable: executable,
            cwd: cwd,
            env,
        }
    }
}

impl Factory for NodeFactory {
    fn make_with_symlinks(
        &self,
        path: &PathBuf,
        symlinks: &Vec<String>,
        known_libs: &HashMap<String, PathBuf>,
    ) -> Result<Node> {
        if !is_shared_library(path) {
            bail!(
                "cannot make_with_symlinks if the path is not a shared library, path={}",
                path.display()
            );
        }
        let spec = NodeSpec::BinaryInLdPath {
            path: path.clone(),
            symlinks: symlinks.clone(),
        };
        generate_node(&spec, &self.executable, &self.cwd, &self.env, known_libs)
    }

    fn make_from_spec(
        &self,
        spec: &NodeSpec,
        known_libs: &HashMap<String, PathBuf>,
    ) -> Result<Node> {
        generate_node(&spec, &self.executable, &self.cwd, &self.env, known_libs)
    }

    fn make_spec(&self, path: &PathBuf) -> Result<NodeSpec> {
        let p = normalize_path(path);
        if !p.exists() {
            bail!(
                "tried creating node from path={}, but the path does not exist",
                path.display()
            );
        }

        if p.starts_with(&self.site_pkgs.lib_dynload) {
            return Ok(NodeSpec::ExecPrefixPkg {
                original_prefix: self.site_pkgs.lib_dynload.clone(),
                version: self.version.clone(),
                path: path.clone(),
            });
        }

        if p.starts_with(&self.site_pkgs.stdlib) {
            return Ok(NodeSpec::PrefixPkg {
                original_prefix: self.site_pkgs.stdlib.clone(),
                version: self.version.clone(),
                path: path.clone(),
            });
        }

        for (site_pkg, alias) in &self.site_pkgs.site_pkg_by_alias {
            if p.starts_with(site_pkg) {
                return Ok(NodeSpec::SitePkg {
                    site_pkg_path: site_pkg.clone(),
                    alias: alias.clone(),
                    version: self.version.clone(),
                    path: path.clone(),
                });
            }
        }

        if !is_shared_library(path) {
            bail!(
                "found a path which is not inside site packages and is not a shared library. Only plain files inside site-packages are allowed, path={}",
                path.display()
            );
        }

        Ok(NodeSpec::Binary { path: path.clone() })
    }
}
