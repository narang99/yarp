// patching libraries to work with the new symlink tree
// basically all install_name_tool operations

use std::{path::PathBuf, process::{Command}};

use anyhow::{Result, anyhow, bail};
use pathdiff::diff_paths;

use crate::node::deps::{
    Deps,
    core::{Binary, Macho},
};

pub trait LibPatch {
    fn patch(&self, real_path: &PathBuf, symlink_farm_path: &PathBuf) -> Result<()>;
}

impl LibPatch for Deps {
    fn patch(&self, real_path: &PathBuf, symlink_farm_path: &PathBuf) -> Result<()> {
        match self {
            Deps::Plain => Ok(()),
            Deps::Binary(binary) => {
                patch_lib(real_path, &binary, symlink_farm_path)?;
                Ok(())
            }
            #[cfg(test)]
            Deps::Mock { paths: _ } => Ok(()),
        }
    }
}

pub fn patch_lib(reals_path: &PathBuf, binary: &Binary, symlink_farm_path: &PathBuf) -> Result<()> {
    // deps is a vector of shared library names, generated from the graph
    // im assuming that symlink farm location is hardcoded here
    // TODO: make this less hardcoded, we should simply find the relative path of symlink farm from reals
    // rpaths etc should use that string instead of hardcoding everything
    match binary {
        Binary::Macho(mach) => {
            for (rpath, _) in mach.rpaths.iter() {
                rm_rpath(rpath, reals_path)?;
            }
            let lib_name = get_lib_name(reals_path)?;
            if lib_name == "python" {
                println!("patchingg python");
            }
            let rpath = get_new_rpath(reals_path, symlink_farm_path)?;
            add_rpath(&rpath, reals_path)?;
            modify_load_cmds(reals_path, symlink_farm_path, mach)?;
            set_dylib_id(dylib_id(&lib_name), &reals_path)?;
            sign_dylib(&reals_path)?;
        }
    };
    Ok(())
}

fn modify_load_cmds(reals_path: &PathBuf, symlink_farm_path: &PathBuf, mach: &Macho) -> Result<()> {
    for (load_cmd, parent_path) in &mach.load_cmds {
        let lib_name = get_lib_name(&parent_path)?;
        let lib_in_farm = symlink_farm_path.join(&lib_name);
        if !lib_in_farm.exists() {
            bail!(
                "fatal: did not find dependency in symlink farm, path={} symlink-farm={} lib_name={}",
                reals_path.display(),
                symlink_farm_path.display(),
                lib_name
            );
        }
        modify_load_command(&load_cmd, &dylib_id(&lib_name), reals_path)?;
        sign_dylib(reals_path)?;
    }
    Ok(())
}

fn dylib_id(lib_name: &str) -> String {
    format!("@rpath/{}", lib_name)
}

fn get_lib_name(path: &PathBuf) -> Result<String> {
    let lib_name = path
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .ok_or_else(|| {
            anyhow!(
                "failed in getting real file name for path={}",
                path.display()
            )
        })?;
    Ok(lib_name.to_string())
}

fn get_new_rpath(real_path: &PathBuf, symlink_farm: &PathBuf) -> Result<String> {
    let real_path_dir = real_path.parent().ok_or_else(|| {
        anyhow!(
            "failed in getting parent of real_path while patching it, path={}",
            real_path.display()
        )
    })?;
    let rel_path = diff_paths(symlink_farm, real_path_dir).ok_or_else(|| {
        anyhow!(
            "failed in finding relative path of symlink farm during patching, symlink_farm={} real_path={}",
            symlink_farm.display(),
            real_path.display()
        )
    })?;
    let rel_path = rel_path.to_str().map(|s| s.to_string()).ok_or_else(|| {
        anyhow!("failed in converting path to string for adding as rpath, path={}", rel_path.display())
    })?;
    Ok(format!("@loader_path/{}/", rel_path))
}

fn rm_rpath(rpath: &str, path: &PathBuf) -> Result<()> {
    let status = Command::new("install_name_tool")
        .arg("-delete_rpath")
        .arg(rpath)
        .arg(path)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        bail!(
            "failed in running install_name_tool to delete rpath path={} rpath={} status={:?}",
            rpath,
            path.display(),
            status
        )
    }
}

fn add_rpath(rpath: &str, path: &PathBuf) -> Result<()> {
    let status = Command::new("install_name_tool")
        .arg("-add_rpath")
        .arg(rpath)
        .arg(path)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        bail!(
            "failed in running install_name_tool to add rpath path={} rpath={} status={:?}",
            rpath,
            path.display(),
            status
        )
    }
}

fn modify_load_command(old: &str, new: &str, path: &PathBuf) -> Result<()> {
    let status = Command::new("install_name_tool")
        .arg("-change")
        .arg(old)
        .arg(new)
        .arg(path)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        bail!(
            "failed in running install_name_tool to modifying load command path={} old={} new={} status={:?}",
            old,
            new,
            path.display(),
            status
        )
    }
}

fn set_dylib_id(id: String, path: &PathBuf) -> Result<()> {
    let status = Command::new("install_name_tool")
        .arg("-id")
        .arg(&id)
        .arg(path)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        bail!(
            "failed in running install_name_tool to set dylib_id path={} id={} status={:?}",
            id,
            path.display(),
            status
        )
    }
}

fn sign_dylib(path: &PathBuf) -> Result<()> {
    let status = Command::new("codesign")
        .arg("-s")
        .arg("-")
        .arg("-f")
        .arg(&path)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        bail!(
            "failed in running signing dylib_id path={} status={:?}",
            path.display(),
            status
        )
    }
}
