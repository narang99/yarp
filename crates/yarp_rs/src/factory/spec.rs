use std::{collections::HashMap, path::PathBuf};

use anyhow::{Result, anyhow};
use pathdiff::diff_paths;

use crate::{
    factory::core::NodeSpec,
    manifest::Version,
    node::{Node, Pkg, PkgSitePackages, PrefixPackages, deps::Deps},
    pkg::paths::is_shared_library,
};

pub fn generate_node(
    payload: &NodeSpec,
    executable_path: &PathBuf,
    cwd: &PathBuf,
    env: &HashMap<String, String>,
    known_libs: &HashMap<String, PathBuf>,
) -> Result<Node> {
    let make_node = |pkg, path| mk_node(path, executable_path, cwd, env, pkg, known_libs);
    match payload {
        NodeSpec::Executable { path } => {
            let deps = Deps::new_binary(executable_path, executable_path, cwd, env, known_libs)?;
            Node::new(path.clone(), Pkg::Executable, deps)
        }
        NodeSpec::ExecPrefixPkg {
            path,
            original_prefix,
            version,
        } => {
            get_exec_prefix_pkg(path, original_prefix, version).and_then(|pkg| make_node(pkg, path))
        }
        NodeSpec::PrefixPkg {
            path,
            original_prefix,
            version,
        } => get_prefix_pkg(path, original_prefix, version).and_then(|pkg| make_node(pkg, path)),
        NodeSpec::SitePkg {
            path,
            site_pkg_path,
            alias,
            version,
        } => get_site_packages_pkg(path, site_pkg_path, alias, version)
            .and_then(|pkg| make_node(pkg, path)),
        NodeSpec::BinaryInLdPath { path, symlinks } => Node::new(
            path.clone(),
            Pkg::BinaryInLDPath {
                symlinks: symlinks.clone(),
            },
            Deps::new_binary(&path, executable_path, cwd, env, known_libs)?,
        ),
        NodeSpec::Binary { path } => Node::new(
            path.clone(),
            Pkg::Binary,
            Deps::new_binary(&path, executable_path, cwd, env, known_libs)?,
        ),
    }
}

fn mk_node(
    p: &PathBuf,
    executable_path: &PathBuf,
    cwd: &PathBuf,
    env: &HashMap<String, String>,
    pkg: Pkg,
    known_libs: &HashMap<String, PathBuf>,
) -> Result<Node> {
    let deps = Deps::from_path(p, executable_path, cwd, env, known_libs)?;
    Node::new(p.clone(), pkg, deps)
}

fn get_exec_prefix_pkg(
    path: &PathBuf,
    original_prefix: &PathBuf,
    version: &Version,
) -> Result<Pkg> {
    let rel_path = diff_paths(&path, &original_prefix).ok_or_else(|| {
        anyhow!(
            "failed in finding relative path of file inside prefix file={} prefix={}",
            path.display(),
            original_prefix.display()
        )
    })?;
    let prefix_pkg = PrefixPackages {
        original_prefix: original_prefix.clone(),
        version: version.clone(),
        rel_path,
    };
    if is_shared_library(path) {
        Ok(Pkg::ExecPrefixBinary(prefix_pkg))
    } else {
        Ok(Pkg::ExecPrefixPlain(prefix_pkg))
    }
}

fn get_prefix_pkg(path: &PathBuf, original_prefix: &PathBuf, version: &Version) -> Result<Pkg> {
    let rel_path = diff_paths(&path, &original_prefix).ok_or_else(|| {
        anyhow!(
            "failed in finding relative path of file inside prefix file={} prefix={}",
            path.display(),
            original_prefix.display()
        )
    })?;
    let prefix_pkg = PrefixPackages {
        original_prefix: original_prefix.clone(),
        version: version.clone(),
        rel_path,
    };
    if is_shared_library(path) {
        Ok(Pkg::PrefixBinary(prefix_pkg))
    } else {
        Ok(Pkg::PrefixPlain(prefix_pkg))
    }
}

fn get_site_packages_pkg(
    path: &PathBuf,
    site_pkg_path: &PathBuf,
    alias: &str,
    _version: &Version,
) -> Result<Pkg> {
    let rel_path = diff_paths(&path, &site_pkg_path).ok_or_else(|| {
        anyhow!(
            "failed in finding relative path of file inside site-packages, file={} site-packages={}",
            path.display(),
            site_pkg_path.display()
        )
    })?;
    let res = PkgSitePackages {
        site_packages: site_pkg_path.clone(),
        alias: alias.to_string(),
        rel_path: rel_path,
    };
    if is_shared_library(path) {
        Ok(Pkg::SitePackagesBinary(res))
    } else {
        Ok(Pkg::SitePackagesPlain(res))
    }
}
