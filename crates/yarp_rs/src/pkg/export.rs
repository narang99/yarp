use std::{fs, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use pathdiff::diff_paths;

use crate::node::Pkg;

pub trait Export {
    fn to_destination(&self, path: &PathBuf, dest: &PathBuf, dist: &PathBuf) -> Result<()>;
}

impl Export for Pkg {
    fn to_destination(&self, path: &PathBuf, dest: &PathBuf, dist: &PathBuf) -> Result<()> {
        mk_parent_dirs(dest)?;
        match self {
            Pkg::SitePackagesPlain(_)
            | Pkg::Plain
            | Pkg::Executable
            | Pkg::PrefixPlain(_)
            | Pkg::ExecPrefixPlain(_) => {
                fs::copy(path, dest)?;
            }

            Pkg::SitePackagesBinary(_)
            | Pkg::Binary
            | Pkg::PrefixBinary(_)
            | Pkg::ExecPrefixBinary(_) => {
                if !dest.starts_with(&dist) {
                    panic!(
                        "failed in moving path to destination, it is mandatory for a shared lib path to be inside dist, dist={} source_path={} destination={}",
                        dist.display(),
                        path.display(),
                        dest.display()
                    );
                }
                let parent_dir = dest.parent().expect(&format!("fatal error: tried symlinking file at dest={}, but it does not have any parent", dest.display()));
                let rel_path = diff_paths(&path, &parent_dir).ok_or_else(|| {
                    anyhow!(
                        "failed in finding relative path for symlinking to destination, destination={} path={}",
                        dest.display(),
                        path.display()
                    )
                })?;
                if dest.exists() {
                    fs::remove_file(&dest)?;
                }
                std::os::unix::fs::symlink(&rel_path, &dest).with_context(|| {
                    format!(
                        "failed in creating symlink to destination, rel_path={} dest={}",
                        rel_path.display(),
                        dest.display()
                    )
                })?;
            }
        };
        Ok(())
    }
}

pub fn mk_parent_dirs(dest: &PathBuf) -> std::io::Result<()> {
    if let Some(parent) = dest.parent() {
        if parent.exists() {
            return Ok(());
        }
        fs::create_dir_all(parent)?;
    }
    Ok(())
}
