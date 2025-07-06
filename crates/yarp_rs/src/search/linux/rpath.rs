use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use log::error;

use crate::{
    paths::{to_path_buf, to_string_path},
    search::linux::auxval::get_at_platform,
};

pub fn get_valid_rpaths(paths: &Vec<String>, object_path: &PathBuf) -> Vec<PathBuf> {
    // we dont fail on rpath parse failures, log the error and move on
    let mut res = Vec::new();
    for p in paths {
        let rpath = parse_rpath(p, object_path);
        match rpath {
            Ok(rpath) => match rpath {
                Some(rpath) => {
                    res.push(rpath);
                }
                None => {}
            },
            Err(e) => {
                error!("failure in parsing rpath: {}, ignoring. error={}", p, e);
            }
        };
    }
    res
}

fn parse_rpath(rpath: &str, object_path: &PathBuf) -> Result<Option<PathBuf>> {
    let parent_path_str = object_path
        .parent()
        .with_context(|| anyhow!("fatal: failed to get parent of {:}", object_path.display()))
        .and_then(|parent| to_string_path(&parent))?;
    let is_64_bit = cfg!(target_pointer_width = "64");
    let lib = if is_64_bit { "lib64" } else { "lib" };
    let at_platform = get_at_platform();

    let rpath = rpath
        .replace("$ORIGIN", &parent_path_str)
        .replace("${ORIGIN}", &parent_path_str)
        .replace("$LIB", lib)
        .replace("${LIB}", lib)
        .replace("$PLATFORM", &at_platform)
        .replace("${PLATFORM}", &at_platform);

    let path = to_path_buf(&rpath)?;
    if path.exists() {
        Ok(Some(path))
    } else {
        Ok(None)
    }
}
