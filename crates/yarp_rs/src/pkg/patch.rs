// patching libraries to work with the new symlink tree
// basically all install_name_tool operations

use std::{path::PathBuf, process::Command};

use anyhow::{bail, Result};


pub fn rm_rpath(rpath: &str, path: &PathBuf) -> Result<()> {
    let status = Command::new("install_name_tool")
        .arg("-delete_rpath")
        .arg(rpath)
        .arg(path)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        bail!("failed in running install_name_tool to delete rpath path={} rpath={} status={:?}", rpath, path.display(), status)
    }
}


pub fn modify_load_command(old: &str, new: &str, path: &PathBuf) -> Result<()> {
    let status = Command::new("install_name_tool")
        .arg("-change")
        .arg(old)
        .arg(new)
        .arg(path)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        bail!("failed in running install_name_tool to modifying load command path={} old={} new={} status={:?}", old, new, path.display(), status)
    }
}