use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;

pub mod core;

// TODO: remove and move this, no need for two identifiers
pub use core::Deps;

use crate::parse::{BinaryParseError};
// use crate::node::deps::{core::BinaryParseError, macho::get_deps_from_macho};

impl Deps {
    pub fn new_binary(
        path: &PathBuf,
        executable_path: &PathBuf,
        cwd: &PathBuf,
        env: &HashMap<String, String>,
        known_libs: &HashMap<String, PathBuf>,
    ) -> Result<Deps> {
        let bin = crate::parse::parse_and_search(path, executable_path, cwd, env, known_libs, &Vec::new());
        match bin {
            Ok((bin, _)) => Ok(Deps::Binary(bin)),
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
    }

    pub fn find(&self) -> Result<Vec<PathBuf>> {
        match &self {
            Deps::Plain => Ok(Vec::new()),
            Deps::Binary(binary) => Ok(binary.dependencies()),
            #[cfg(test)]
            Deps::Mock { paths } => Ok(paths.clone()),
        }
    }

    pub fn from_path(
        path: &PathBuf,
        executable_path: &PathBuf,
        cwd: &PathBuf,
        env: &HashMap<String, String>,
        known_libs: &HashMap<String, PathBuf>,
    ) -> Result<Deps> {
        let ext = path.extension();
        match ext {
            None => Ok(Deps::Plain),
            Some(ext) => {
                if ext == "so" || ext == "dylib" {
                    Ok(Deps::new_binary(
                        path,
                        executable_path,
                        cwd,
                        env,
                        known_libs,
                    )?)
                } else {
                    Ok(Deps::Plain)
                }
            }
        }
    }

    #[cfg(test)]
    pub fn mock(deps: Vec<PathBuf>) -> Deps {
        Deps::Mock { paths: deps }
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, path::PathBuf};

    use crate::node::deps::Deps;

    // todo: this only works on my machine
    #[test]
    fn test_local() {
        let path =
            PathBuf::from("/Users/hariomnarang/miniconda3/envs/platform/lib/libpango-1.0.0.dylib");
        let executable_path = PathBuf::from("/Users/hariomnarang/miniconda3/bin/python");
        let env = HashMap::new();
        let known_libs = HashMap::new();
        let cwd = PathBuf::from(".");
        let dylib = Deps::new_binary(&path, &executable_path, &cwd, &env, &known_libs).unwrap();
        let dylib = dylib.find().unwrap();
        dbg!(dylib);
    }
}
