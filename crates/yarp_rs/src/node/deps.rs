use std::path::PathBuf;

use anyhow::{Result, bail};

mod core;
mod macho;

pub use core::Deps;

use crate::node::deps::macho::get_deps_from_macho;

impl Deps {
    pub fn new_binary(path: &PathBuf, executable_path: &PathBuf, cwd: &PathBuf) -> Result<Deps> {
        let bin = Deps::new_macho_binary(path, executable_path, cwd)?;
        Ok(Deps::Binary(bin))
    }

    fn new_macho_binary(
        path: &PathBuf,
        executable_path: &PathBuf,
        cwd: &PathBuf,
    ) -> Result<core::Binary> {
        match path.to_str() {
            None => {
                bail!(
                    "failed in parsing macho binary, path could not be converted to string, path={}",
                    path.display()
                );
            }
            Some(p) => {
                let parsed = macho::parse(p, executable_path, cwd)?;
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

    pub fn from_path(path: &PathBuf, executable_path: &PathBuf, cwd: &PathBuf) -> Result<Deps> {
        let ext = path.extension();
        match ext {
            None => Ok(Deps::Plain),
            Some(ext) => {
                if ext == "so" || ext == "dylib" {
                    Ok(Deps::new_binary(path, executable_path, cwd)?)
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
        let cwd = PathBuf::from(".");
        let dylib = Deps::new_binary(&path, &executable_path, &cwd).unwrap();
        let dylib = dylib.find().unwrap();
        dbg!(dylib);
    }
}
