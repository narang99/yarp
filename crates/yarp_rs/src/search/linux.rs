mod auxval;
mod ldconfig;
mod rpath;

use std::{path::PathBuf, str::FromStr};

use log::warn;

macro_rules! try_find_in_dirs {
    ($name:expr, $dirs:expr, $rpaths:expr) => {
        if let Some(p) = find_in_dirs($name, $dirs) {
            return Some(LinuxSearchResult {
                path: p,
                rpaths: $rpaths,
            });
        }
    };
}

pub struct LinuxSearchResult {
    pub path: PathBuf,
    pub rpaths: Vec<PathBuf>,
}

pub fn search(
    name: &str,
    object_path: &PathBuf,
    dt_rpaths: &Vec<String>,
    dt_runpaths: &Vec<String>,
    ld_preload: &Vec<String>,
    ld_library_path: &Vec<String>,
    extra_rpaths: &Vec<PathBuf>,
) -> Option<LinuxSearchResult> {
    // this function intentionally does not fail
    // we simply log warnings if there is a failure as rpath resolution is always about trying multiple stuff

    // first get the rpaths which we will search, we need to return these
    let mut new_dt_rpaths = Vec::new();
    let search_rpaths = dt_runpaths.len() == 0;
    if search_rpaths {
        new_dt_rpaths = rpath::get_valid_rpaths(dt_rpaths, object_path);
    }

    // search LD_PRELOAD
    let ld_preload = get_valid_paths(ld_preload);
    try_find_in_dirs!(name, &ld_preload, new_dt_rpaths);

    // search DT_RPATH
    if search_rpaths {
        try_find_in_dirs!(name, &new_dt_rpaths, new_dt_rpaths);
        try_find_in_dirs!(name, extra_rpaths, new_dt_rpaths);
    }

    // search LD_LIBRARY_PATH
    let ld_library_path = get_valid_paths(ld_library_path);
    try_find_in_dirs!(name, &ld_library_path, new_dt_rpaths);

    // search DT_RUNPATH
    let dt_runpaths = rpath::get_valid_rpaths(dt_runpaths, object_path);
    try_find_in_dirs!(name, &dt_runpaths, new_dt_rpaths);

    // fallback, ask ldconfig
    if let Ok(res) = ldconfig::find(name) {
        return Some(LinuxSearchResult {
            path: res,
            rpaths: new_dt_rpaths,
        });
    }

    None
}

fn get_valid_paths(ps: &Vec<String>) -> Vec<PathBuf> {
    let mut res = Vec::new();
    for p in ps {
        let path = PathBuf::from_str(p);
        match path {
            Ok(path) => {
                if path.exists() && path.is_dir() {
                    res.push(path);
                }
            }
            Err(e) => {
                warn!("ld_library_path component parse failure: {p}: {e}");
            }
        }
    }
    res
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
