/**
 * All file types that we handle in yarp
 * A file can be either a binary file or a plain file
 * This module takes care of creating `DistFile` implementations for every kind of node in the wild
 * We attach `DistNode` from this
 */
use std::{path::PathBuf, rc::Rc};

use anyhow::{Context, Result, anyhow};

use crate::{
    macho::{SharedLibCtx, parse},
    node::{DistFile, DistNode, Node},
};


#[derive(Debug)]
pub struct PlainFile {
    pub path: PathBuf,
}
impl DistFile for PlainFile {
    fn deps(&self) -> Result<Vec<DistNode>> {
        Ok(Vec::new())
    }

    fn file_path(&self) -> &PathBuf {
        &self.path
    }
}

#[derive(Debug)]
pub struct BinaryFile {
    pub executable_path: PathBuf,
    pub cwd: PathBuf,
    pub path: PathBuf,
}

impl BinaryFile {
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

impl DistFile for BinaryFile {
    fn deps(&self) -> Result<Vec<DistNode>> {
        let ctx = SharedLibCtx {
            executable_path: &self.executable_path,
            cwd: &self.cwd,
        };
        // for now we hardcode for getting macho deps
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
        .map(|(_, path)| BinaryFile {
            executable_path: ctx.executable_path.clone(),
            cwd: ctx.cwd.clone(),
            path: path,
        })
        .map(dist_node_from_dylib)
        .collect()
}

fn dist_node_from_dylib(dylib: BinaryFile) -> Result<DistNode> {
    let node = Node { path: dylib.path.clone() };
    Ok(DistNode {
        node: Rc::new(node),
        dist_file: Rc::new(dylib),
    })
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use crate::{ftypes::BinaryFile, node::DistFile};

    // todo: this only works on my machine
    #[test]
    fn test_local() {
        let dylib = BinaryFile {
            executable_path: PathBuf::from("/Users/hariomnarang/miniconda3/bin/python"),
            cwd: PathBuf::from("."),
            path: PathBuf::from(
                "/Users/hariomnarang/miniconda3/envs/platform/lib/libpango-1.0.0.dylib",
            ),
        };
        dbg!(dylib.deps().unwrap());
    }
}
