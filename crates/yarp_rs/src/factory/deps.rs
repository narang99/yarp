use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;

use crate::{node::deps::Deps, parse::BinaryParseError, pkg::paths::is_maybe_shared_library};

pub fn create_deps(
    path: &PathBuf,
    executable_path: &PathBuf,
    cwd: &PathBuf,
    env: &HashMap<String, String>,
    known_libs: &HashMap<String, PathBuf>,
    extra_search_paths: &Vec<PathBuf>,
) -> Result<Deps> {
    if is_maybe_shared_library(path) {
        let bin = crate::parse::parse_and_search(
            path,
            executable_path,
            cwd,
            env,
            known_libs,
            extra_search_paths,
        );
        match bin {
            Ok(bin) => Ok(Deps::Binary(bin)),
            Err(e) => {
                if let Some(parse_err) = e.downcast_ref::<BinaryParseError>() {
                    match parse_err {
                        BinaryParseError::UnsupportedArchitecture | BinaryParseError::NotBinary => {
                            Ok(Deps::Plain)
                        }
                    }
                } else {
                    Err(e)
                }
            }
        }
    } else {
        Ok(Deps::Plain)
    }
}
