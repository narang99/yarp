use std::{collections::HashMap, path::PathBuf};

use anyhow::{Result, bail};

pub mod core;
mod macho;

// TODO: remove and move this, no need for two identifiers
pub use core::Deps;

use crate::node::deps::{core::BinaryParseError, macho::get_deps_from_macho};

impl Deps {
    pub fn new_binary(
        path: &PathBuf,
        executable_path: &PathBuf,
        cwd: &PathBuf,
        dyld_library_path: &Vec<PathBuf>,
    ) -> Result<Deps> {
        let bin = Deps::new_macho_binary(path, executable_path, cwd, dyld_library_path);
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
    }

    fn new_macho_binary(
        path: &PathBuf,
        executable_path: &PathBuf,
        cwd: &PathBuf,
        dyld_library_path: &Vec<PathBuf>,
    ) -> Result<core::Binary> {
        match path.to_str() {
            None => {
                bail!(
                    "failed in parsing macho binary, path could not be converted to string, path={}",
                    path.display()
                );
            }
            Some(p) => {
                // TODO: get this from function params
                let known_libs = HashMap::new();
                let parsed = macho::parse(p, executable_path, cwd, dyld_library_path, &known_libs)?;
                Ok(core::Binary::Macho(parsed))
            }
        }
    }

    pub fn find(&self) -> Result<Vec<PathBuf>> {
        match &self {
            Deps::Plain => Ok(Vec::new()),
            Deps::Binary(binary) => match binary {
                core::Binary::Macho(mach) => Ok(get_deps_from_macho(mach)),
            },
            #[cfg(test)]
            Deps::Mock { paths } => Ok(paths.clone()),
        }
    }

    pub fn from_path(
        path: &PathBuf,
        executable_path: &PathBuf,
        cwd: &PathBuf,
        dyld_library_path: &Vec<PathBuf>,
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
                        dyld_library_path,
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
    use std::path::PathBuf;

    use crate::node::deps::Deps;

    // todo: this only works on my machine
    #[test]
    fn test_local() {
        let path =
            PathBuf::from("/Users/hariomnarang/miniconda3/envs/platform/lib/libpango-1.0.0.dylib");
        let executable_path = PathBuf::from("/Users/hariomnarang/miniconda3/bin/python");
        let dyld_library_path = Vec::new();
        let cwd = PathBuf::from(".");
        let dylib = Deps::new_binary(&path, &executable_path, &cwd, &dyld_library_path).unwrap();
        let dylib = dylib.find().unwrap();
        dbg!(dylib);
    }
}
