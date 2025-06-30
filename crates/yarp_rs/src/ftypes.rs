use std::{path::PathBuf, rc::Rc};

use anyhow::{Context, Result, anyhow};

use crate::{
    macho::{SharedLibCtx, parse},
    node::{DistFile, DistNode, Kind, Node},
};

// this can be any resource file inside a site-packages
#[derive(Debug)]
pub struct UnknownFileInSitePackages {
    pub path: PathBuf,
}
impl DistFile for UnknownFileInSitePackages {
    fn deps(&self) -> Result<Vec<DistNode>> {
        Ok(Vec::new())
    }

    fn file_path(&self) -> &PathBuf {
        &self.path
    }
}

#[derive(Debug)]
pub struct PyFileInSitePackages {
    pub path: PathBuf,
}

impl DistFile for PyFileInSitePackages {
    fn deps(&self) -> Result<Vec<DistNode>> {
        Ok(Vec::new())
    }

    fn file_path(&self) -> &PathBuf {
        &self.path
    }
}

#[derive(Debug)]
pub struct PythonExe {
    pub path: PathBuf,
    pub cwd: PathBuf,
}

impl DistFile for PythonExe {
    fn deps(&self) -> Result<Vec<crate::node::DistNode>> {
        let ctx = SharedLibCtx {
            executable_path: &self.path,
            cwd: &self.cwd,
        };
        get_deps_of_macho(&self.path, &ctx).with_context(|| {
            anyhow!(
                "failure in resolving dependencies of library={}",
                self.path.display()
            )
        })
    }

    fn file_path(&self) -> &std::path::PathBuf {
        &self.path
    }
}

#[derive(Debug)]
pub struct Dylib {
    pub executable_path: PathBuf,
    pub cwd: PathBuf,
    pub path: PathBuf,
}

impl Dylib {
    pub fn file_name_from_path(path: &PathBuf) -> Result<String> {
        path.file_name().ok_or_else(|| {
            anyhow!(
                "failed in getting file_name for dependency, path={}",
                path.display()
            )
        }).and_then(|file_name| {
            file_name.to_str().ok_or(anyhow!("failed in converting path of file to string, this most likely contains non-utf-8 characters in path, path={}", path.display()))
        }).map(|file_name| file_name.to_string())
    }
}

impl DistFile for Dylib {
    fn deps(&self) -> Result<Vec<DistNode>> {
        let ctx = SharedLibCtx {
            executable_path: &self.executable_path,
            cwd: &self.cwd,
        };
        get_deps_of_macho(&self.path, &ctx).with_context(|| {
            anyhow!(
                "failure in resolving dependencies of library={}",
                self.path.display()
            )
        })
    }

    fn file_path(&self) -> &PathBuf {
        &self.path
    }
}

fn get_deps_of_macho(
    macho_path: &PathBuf,
    ctx: &SharedLibCtx,
) -> Result<Vec<crate::node::DistNode>> {
    let string_path = macho_path.to_str().ok_or(anyhow!(
        "path {} could not be converted to string, `yarp` does not support these paths",
        macho_path.display()
    ))?;
    let macho = parse(&string_path, ctx)?;
    macho
        .load_cmds
        .into_iter()
        .map(|(_, path)| Dylib {
            executable_path: ctx.executable_path.clone(),
            cwd: ctx.cwd.clone(),
            path: path,
        })
        .map(dist_node_from_dylib)
        .collect()
}

fn dist_node_from_dylib(dylib: Dylib) -> Result<DistNode> {
    let node = Node {
        kind: Kind::SharedLibrary {
            lib_path: dylib.path.clone(),
        },
    };
    Ok(DistNode {
        node: Rc::new(node),
        dist_file: Rc::new(dylib),
    })
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use crate::{ftypes::Dylib, node::DistFile};

    // todo: this only works on my machine
    #[test]
    fn test_local() {
        let dylib = Dylib {
            executable_path: PathBuf::from("/Users/hariomnarang/miniconda3/bin/python"),
            cwd: PathBuf::from("."),
            path: PathBuf::from(
                "/Users/hariomnarang/miniconda3/envs/platform/lib/libpango-1.0.0.dylib",
            ),
        };
        dbg!(dylib.deps().unwrap());
    }
}
