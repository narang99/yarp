use std::path::PathBuf;

use anyhow::{Result, anyhow};
use pathdiff::diff_paths;

use crate::{
    manifest::Version,
    node::{Node, Pkg, PkgSitePackages, PrefixPackages, deps::Deps},
    pkg::paths::is_shared_library,
};

pub enum CreateNode {
    Executable {
        path: PathBuf,
    },
    ExecPrefixPkg {
        original_prefix: PathBuf,
        alias: String,
        version: Version,
        path: PathBuf,
    },
    PrefixPkg {
        original_prefix: PathBuf,
        alias: String,
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
    },
}

pub fn generate_node(
    payload: &CreateNode,
    executable_path: &PathBuf,
    cwd: &PathBuf,
    dyld_library_path: &Vec<PathBuf>,
) -> Result<Node> {
    let make_node = |pkg, path| mk_node(path, executable_path, cwd, dyld_library_path, pkg);
    match payload {
        CreateNode::Executable { path } => get_executable_node(path, cwd, dyld_library_path),
        CreateNode::ExecPrefixPkg {
            path,
            original_prefix,
            alias,
            version,
        } => get_exec_prefix_pkg(path, original_prefix, alias, version)
            .and_then(|pkg| make_node(pkg, path)),
        CreateNode::PrefixPkg {
            path,
            original_prefix,
            alias,
            version,
        } => get_prefix_pkg(path, original_prefix, alias, version)
            .and_then(|pkg| make_node(pkg, path)),
        CreateNode::SitePkg {
            path,
            site_pkg_path,
            alias,
            version,
        } => get_site_packages_pkg(path, site_pkg_path, alias, version)
            .and_then(|pkg| make_node(pkg, path)),
        CreateNode::BinaryInLdPath { path } => Ok(Node::new(
            path.clone(),
            Pkg::BinaryInLDPath,
            Deps::new_binary(&path, executable_path, cwd, dyld_library_path)?,
        )),
    }
}

fn mk_node(
    p: &PathBuf,
    executable_path: &PathBuf,
    cwd: &PathBuf,
    dyld_library_path: &Vec<PathBuf>,
    pkg: Pkg,
) -> Result<Node> {
    let deps = Deps::from_path(p, executable_path, cwd, dyld_library_path)?;
    Ok(Node::new(p.clone(), pkg, deps))
}

fn get_executable_node(
    executable_path: &PathBuf,
    cwd: &PathBuf,
    dyld_library_path: &Vec<PathBuf>,
) -> Result<Node> {
    let p = executable_path.clone();
    Ok(Node::new(
        p,
        Pkg::Executable,
        Deps::new_binary(executable_path, executable_path, cwd, dyld_library_path)?,
    ))
}

fn get_exec_prefix_pkg(
    path: &PathBuf,
    original_prefix: &PathBuf,
    _alias: &str,
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

fn get_prefix_pkg(
    path: &PathBuf,
    original_prefix: &PathBuf,
    _alias: &str,
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
