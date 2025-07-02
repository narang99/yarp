// this defines the folder structure of the final tree
// there are some properties in the final tree
// does the file exist in reals?
// do we symlink from reals? do we copy?
// where are the dependencies of the file relative to the file? for patching?
// where is the destination?

use std::path::PathBuf;

use log::error;

use crate::{
    manifest::Version,
    node::{Pkg, PkgSitePackages, PrefixPackages},
};

pub trait ExportedFileTree {
    // returns the destination if there is an actual destination
    fn destination(&self, path: &PathBuf, dist: &PathBuf) -> Option<PathBuf>;

    // reals location, if needed
    fn reals(&self, path: &PathBuf, dist: &PathBuf) -> Option<PathBuf>;

    // symlink farm location, if exists
    fn symlink_farm(&self, path: &PathBuf, dist: &PathBuf) -> Option<PathBuf>;
}

impl ExportedFileTree for Pkg {
    fn destination(&self, path: &PathBuf, dist: &PathBuf) -> Option<PathBuf> {
        match self {
            Pkg::SitePackagesPlain(site_pkgs) | Pkg::SitePackagesBinary(site_pkgs) => {
                Some(site_pkgs_path_in_dist(site_pkgs, dist))
            }
            Pkg::ExecPrefixBinary(prefix_pkgs) | Pkg::ExecPrefixPlain(prefix_pkgs) => {
                Some(exec_prefix_path_in_dist(prefix_pkgs, dist))
            }
            Pkg::PrefixBinary(prefix_pkgs) | Pkg::PrefixPlain(prefix_pkgs) => {
                Some(prefix_path_in_dist(prefix_pkgs, dist))
            }
            Pkg::Plain => None,
            Pkg::BinaryInLDPath => path.file_name().map(|p| dist.join("lib").join("l").join(p)),
            Pkg::Binary => None,
            Pkg::Executable => Some(dist.join("python").join("bin").join("python")),
        }
    }

    fn reals(&self, path: &PathBuf, dist: &PathBuf) -> Option<PathBuf> {
        match self {
            Pkg::SitePackagesPlain(_)
            | Pkg::Plain
            | Pkg::Executable
            | Pkg::ExecPrefixPlain(_)
            | Pkg::PrefixPlain(_) => None,

            Pkg::SitePackagesBinary(_)
            | Pkg::Binary
            | Pkg::BinaryInLDPath
            | Pkg::PrefixBinary(_)
            | Pkg::ExecPrefixBinary(_) => reals_path(path, dist),
        }
    }

    fn symlink_farm(&self, path: &PathBuf, dist: &PathBuf) -> Option<PathBuf> {
        match self {
            Pkg::SitePackagesPlain(_)
            | Pkg::Plain
            | Pkg::ExecPrefixPlain(_)
            | Pkg::PrefixPlain(_) => None,

            Pkg::SitePackagesBinary(_)
            | Pkg::Executable
            | Pkg::Binary
            | Pkg::BinaryInLDPath
            | Pkg::ExecPrefixBinary(_)
            | Pkg::PrefixBinary(_) => symlink_farm_path(path, dist),
        }
    }
}

fn site_pkgs_path_in_dist(site_pkgs: &PkgSitePackages, dist: &PathBuf) -> PathBuf {
    dist.join(site_pkgs_relative_path(&site_pkgs.alias))
        .join(&site_pkgs.rel_path)
}

pub fn site_pkgs_relative_path(alias: &str) -> PathBuf {
    PathBuf::from("site_packages").join(alias)
}

fn exec_prefix_path_in_dist(prefix_pkgs: &PrefixPackages, dist: &PathBuf) -> PathBuf {
    dist.join(lib_dynload_relative_path(&prefix_pkgs.version))
        .join(&prefix_pkgs.rel_path)
}

pub fn lib_dynload_relative_path(version: &Version) -> PathBuf {
    PathBuf::from("python")
        .join("lib")
        .join(version.get_python_version())
        .join("lib-dynload")
}

fn prefix_path_in_dist(prefix_pkgs: &PrefixPackages, dist: &PathBuf) -> PathBuf {
    dist.join(stdlib_relative_path(&prefix_pkgs.version))
        .join(&prefix_pkgs.rel_path)
}

pub fn stdlib_relative_path(version: &Version) -> PathBuf {
    PathBuf::from("python")
        .join("lib")
        .join(version.get_python_version())
}

fn reals_path(path: &PathBuf, dist: &PathBuf) -> Option<PathBuf> {
    loose_validate_path_is_file(path);
    let reals_dir = dist.join("reals").join("r");
    path.file_name().map(|file_name| reals_dir.join(file_name))
}

fn symlink_farm_path(path: &PathBuf, dist: &PathBuf) -> Option<PathBuf> {
    loose_validate_path_is_file(path);
    let symlinks_farm_dir = dist.join("symlinks");
    path.file_name()
        .map(|file_name| symlinks_farm_dir.join(file_name))
}

fn loose_validate_path_is_file(path: &PathBuf) {
    if !path.is_file() {
        if cfg!(debug_assertions) {
            panic!(
                "got a directory for moving to reals path={}",
                path.display()
            );
        } else {
            error!(
                "error: found a directory for moving to reals directory, please raise this with the developer, yarp will ignore this path and move on, path={}",
                path.display()
            );
        }
    }
}

pub fn is_shared_library(path: &PathBuf) -> bool {
    let ext = path.extension();
    match ext {
        None => false,
        Some(ext) => {
            if ext == "so" || ext == "dylib" {
                true
            } else {
                false
            }
        }
    }
}
