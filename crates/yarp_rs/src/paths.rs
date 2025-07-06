// basic path operations
// given a macho to parse
// we return the rpaths if they exist
// loader-path would be simply the current path
// we also want executable-path as an input

use anyhow::{anyhow, Context, Result};

use crate::manifest::Env;
use std::{path::{Component, Path, PathBuf}, str::FromStr};

pub fn normalize_path(path: &Path) -> PathBuf {
    // copied from cargo
    // https://github.com/rust-lang/cargo/blob/fede83ccf973457de319ba6fa0e36ead454d2e20/src/cargo/util/paths.rs#L61
    // basically `canonicalize`, but does not require the path to exist
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

pub fn is_sys_lib(path: &str) -> bool {
    path.starts_with("/usr/lib/")
        || path.starts_with("/System/Library/Frameworks/")
        || path.starts_with("/System/Library/PrivateFrameworks/")
}

pub fn get_dyld_library_path(env: &Env) -> Vec<PathBuf> {
    if let Some(dyld_library_path) = env.get("DYLD_LIBRARY_PATH") {
        dyld_library_path.split(':').map(PathBuf::from).collect()
    } else {
        Vec::new()
    }
}

pub fn to_string_path(path: &Path) -> Result<String> {
    path.to_str().map(|s| s.to_string()).with_context(|| {
        anyhow!(
            "failed in getting string representation of file path={}",
            path.display()
        )
    })
}

pub fn to_path_buf(path: &str) -> Result<PathBuf> {
    PathBuf::from_str(path)
        .with_context(|| anyhow!("failed in getting path from string path={}", path))
}
