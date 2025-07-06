mod auxval;
mod ldconfig;
mod rpath;

use std::path::PathBuf;

use log::warn;

use crate::paths::{get_valid_paths, to_path_buf};

macro_rules! try_find_in_dirs {
    ($name:expr, $dirs:expr) => {
        if let Some(p) = find_in_dirs($name, $dirs) {
            return Some(p);
        }
    };
}

pub struct LinuxSearchResult {
    pub path: PathBuf,
}

pub fn search(
    name: &str,
    dt_rpaths: &Vec<PathBuf>,
    extra_rpaths: &Vec<PathBuf>,
    dt_runpaths: &Vec<PathBuf>,
    ld_preload: &Vec<PathBuf>,
    ld_library_path: &Vec<PathBuf>,
    cwd: &PathBuf,
) -> Option<PathBuf> {
    // this function intentionally does not fail
    // we simply log warnings if there is a failure as rpath resolution is always about trying multiple stuff

    // search as a path
    if let Some(path) = search_name_as_path(name, cwd) {
        return Some(path);
    }

    // search LD_PRELOAD
    try_find_in_dirs!(name, &ld_preload);

    // search DT_RPATH
    let search_rpaths = dt_runpaths.len() == 0;
    if search_rpaths {
        try_find_in_dirs!(name, dt_rpaths);
        try_find_in_dirs!(name, extra_rpaths);
    }

    // search LD_LIBRARY_PATH
    try_find_in_dirs!(name, &ld_library_path);

    // search DT_RUNPATH
    try_find_in_dirs!(name, dt_runpaths);

    // fallback, ask ldconfig
    if let Ok(path) = ldconfig::find(name) {
        return Some(path);
    }

    None
}

fn search_name_as_path(name: &str, cwd: &PathBuf) -> Option<PathBuf> {
    if !name.contains("/") {
        None
    } else {
        match to_path_buf(name) {
            Ok(p) => {
                if p.is_absolute() {
                    if p.exists() { Some(p) } else { None }
                } else {
                    let p = cwd.join(p);
                    if p.exists() { Some(p) } else { None }
                }
            }
            Err(e) => {
                warn!(
                    "relative path could not be resolved, could not convert name to pathbuf: p={} e={}",
                    name, e
                );
                None
            }
        }
    }
}

fn find_in_dirs(file_name: &str, dirs: &Vec<PathBuf>) -> Option<PathBuf> {
    for dir in dirs {
        let candidate = dir.join(file_name);
        if candidate.exists() {
            return Some(candidate.clone());
        }
    }
    None
}
