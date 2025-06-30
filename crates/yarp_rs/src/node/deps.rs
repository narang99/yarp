use std::path::PathBuf;

use anyhow::Result;

mod macho;

#[derive(Debug, Clone)]
pub enum Deps {
    Plain,
    Binary,

    #[cfg(test)]
    Mock {
        paths: Vec<PathBuf>,
    },
}

impl Deps {
    pub fn find(
        &self,
        path: &PathBuf,
        executable_path: &PathBuf,
        cwd: &PathBuf,
    ) -> Result<Vec<PathBuf>> {
        match &self {
            Deps::Plain => Ok(Vec::new()),
            Deps::Binary => macho::get_deps(path, executable_path, cwd),

            #[cfg(test)]
            Deps::Mock { paths } => Ok(paths.clone()),
        }
    }

    pub fn from_path(path: &PathBuf) -> Deps {
        if path.ends_with(".so") || path.ends_with(".dylib") {
            Deps::Binary
        } else {
            Deps::Plain
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
        let dylib = Deps::Binary;
        let path =
            PathBuf::from("/Users/hariomnarang/miniconda3/envs/platform/lib/libpango-1.0.0.dylib");
        let executable_path = PathBuf::from("/Users/hariomnarang/miniconda3/bin/python");
        let cwd = PathBuf::from(".");
        let dylib = dylib.find(&path, &executable_path, &cwd).unwrap();
        dbg!(dylib);
    }
}
