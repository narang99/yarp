// main function which moves stuff to dist

use std::{fs, path::PathBuf};

use anyhow::{anyhow, Context, Error, Result};
use log::{error, warn};
use pathdiff::diff_paths;

use crate::{
    graph::FileGraph,
    node::{Node, Pkg, PkgSitePackages},
    pkg::export::{Export, mk_parent_dirs},
    pkg::paths::ExportedFileTree,
};

pub mod export;
pub mod patch;
pub mod paths;

pub fn move_to_dist(node: &Node, deps: &Vec<Node>, dist: &PathBuf) -> Result<()> {
    // deps are already exported, now we export node

    let real_path = mk_reals(node, dist).with_context(|| {
        format!("could not create reals directory for path={} dist={}", node.path.display(), dist.display())
    })?;

    mk_symlink_farm(node, deps, dist).with_context(|| {
        format!("could not create symlink farm for path={} dist={}", node.path.display(), dist.display())
    })?;


    let path_to_cp_to_destination = real_path.unwrap_or(node.path.clone());
    node.pkg
        .destination(&node.path, dist)
        .map(|dest| {
            match node.pkg {
                Pkg::SitePackagesBinary(ref pkg) | Pkg::SitePackagesPlain(ref pkg) => {
                    if pkg.site_packages.to_string_lossy() == "/Users/hariomnarang/miniconda3/lib/python3.12/site-packages" {
                        println!("difffff alias={} path={} site={} rel={}", pkg.alias, dest.display(), pkg.site_packages.display(), pkg.rel_path.display());
                    }
                },
                _ => {}
            }
            node.pkg.to_destination(&path_to_cp_to_destination, &dest, &dist)
        })
        .transpose()
        .with_context(|| {
            format!("could not move to destination for path={} dist={}", node.path.display(), dist.display())
        })?;

    Ok(())
}


fn mk_reals(node: &Node, dist: &PathBuf) -> Result<Option<PathBuf>> {
    node.pkg
        .reals(&node.path, dist)
        .map(|dest| -> Result<PathBuf> {
            mk_parent_dirs(&dest)?;
            fs::copy(&node.path, &dest)?;
            Ok(dest)
        })
        .transpose()
}


fn mk_symlink_farm(node: &Node, deps: &Vec<Node>, dist: &PathBuf) -> Result<Option<()>> {
    node.pkg.symlink_farm(&node.path, dist).map(|symlink_dir| -> Result<()> {
        fs::create_dir_all(&symlink_dir)?;
        for dep in deps {
            let dep_reals_path = dep.pkg.reals(&dep.path, dist);
            match dep_reals_path {
                None => {},
                Some(p) => {
                    let file_name = p.file_name().ok_or_else(|| {
                        anyhow!("could not find file_name for creating symlink for dependency, path={}", p.display())
                    })?;
                    let rel_path = diff_paths(&p, &symlink_dir).ok_or_else(|| {
                        anyhow!(
                            "failed in finding relative path for creating symlink farm, symlink_dir={} path={}",
                            symlink_dir.display(),
                            p.display()
                        )
                    })?;
                    let dest = symlink_dir.join(file_name);
                    if dest.exists() {
                        fs::remove_file(&dest)?;
                    }
                    std::os::unix::fs::symlink(&rel_path, &dest)?;
                }
            };
        }
        Ok(())
    }).transpose()
}