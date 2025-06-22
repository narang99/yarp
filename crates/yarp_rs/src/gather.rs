// given a yarp manifest, gather all the nodes that we can discover
use std::{collections::HashSet, hash::Hash, path::PathBuf, rc::Rc};

use anyhow::{Result, anyhow};
use log::{info, warn};

use crate::{
    ftypes::{Dylib, PyFile, PythonExe},
    manifest::{AssociatedFile, Load, YarpManifest},
    node::{DistNode, Kind, Node},
};

type Done = HashSet<Rc<Node>>;

fn gather_all_nodes(manifest: &YarpManifest, cwd: &PathBuf) -> Result<()> {
    let mut done: Done = HashSet::new();
    let exe = &manifest.python.sys.executable;
    let loads = get_nodes(
        &manifest.loads,
        exe,
        cwd,
        &mut done,
    )?;
    let pures = get_nodes(&manifest.modules.pure, &exe, cwd, &mut done)?;
    let exts = get_nodes(&manifest.modules.extensions, &exe, cwd, &mut done)?;

    Ok(())
}

fn gather_all_files_from_manifest(manifest: &YarpManifest) {
}

fn get_nodes(
    loads: &Vec<impl AssociatedFile>,
    executable_path: &str,
    cwd: &PathBuf,
    done: &mut Done,
) -> Result<Vec<DistNode>> {
    let mut res = Vec::new();
    for load in loads {
        let p = load.get_path();
        match path_type(&p) {
            PathType::PyFile => {
                let node = mk_py_file_node(PathBuf::from(p))?;
                done.insert(Rc::clone(&node.node));
                res.push(node);
            }
            PathType::SharedLibrary => {
                let node = mk_shared_lib_node(executable_path, &cwd, p)?;
                done.insert(Rc::clone(&node.node));
                res.push(node);
            }
            PathType::Unknown => {
                warn!("unknown file type in loads, ignoring, path={}", p);
            }
        };
    }
    Ok(res)
}

enum PathType {
    PyFile,
    SharedLibrary,
    Unknown,
}

fn path_type(p: &str) -> PathType {
    if p.ends_with(".py") {
        PathType::PyFile
    } else if p.ends_with(".so") || p.ends_with(".dylib") {
        PathType::SharedLibrary
    } else {
        PathType::Unknown
    }
}

fn mk_py_exe_node(executable_path: &str, cwd: &PathBuf) -> Result<DistNode> {
    let py_node = Rc::new(Node { kind: Kind::Python });
    let path = PathBuf::from(executable_path);
    let python_exe = PythonExe {
        path,
        cwd: cwd.clone(),
    };
    Ok(DistNode {
        node: py_node,
        dist_file: Rc::new(python_exe),
    })
}

fn mk_shared_lib_node(executable_path: &str, cwd: &PathBuf, lib_path: &str) -> Result<DistNode> {
    let dylib = Dylib {
        executable_path: PathBuf::from(executable_path),
        cwd: cwd.clone(),
        path: PathBuf::from(lib_path),
    };
    let lib_node = Rc::new(Node {
        kind: Kind::SharedLibrary {
            name: Dylib::file_name_from_path(&dylib.path)?,
        },
    });
    Ok(DistNode {
        node: lib_node,
        dist_file: Rc::new(dylib),
    })
}

fn mk_py_file_node(path: PathBuf) -> Result<DistNode> {
    let py_file = PyFile { path: path.clone() };
    let node = Rc::new(Node {
        kind: Kind::PyFile {
            src_path: path
                .to_str()
                .ok_or(anyhow!("path contains invalid UTF-8"))?
                .to_string(),
        },
    });
    Ok(DistNode {
        node,
        dist_file: Rc::new(py_file),
    })
}
