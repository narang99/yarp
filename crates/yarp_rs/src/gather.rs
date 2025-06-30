use core::unimplemented;
// given a yarp manifest, gather all the nodes that we can discover
use std::{collections::HashSet, hash::Hash, path::PathBuf, rc::Rc};

use anyhow::{Result, anyhow};
use log::{info, warn};

use crate::{
    ftypes::{Dylib, PyFileInSitePackages, PythonExe},
    manifest::{AssociatedFile, Load, YarpManifest},
    node::{DistNode, Kind, Node},
};

type Done = HashSet<Rc<Node>>;

/**
 * Currently, we have nodes in a graph. Its an enum
 * Each node is also hashable
 * When we add a node, we also add its dependents
 * The identity of a node is ALWAYS its absolute path (for all of them)
 * For shared libraries, we have an edge case. Each node is symlinked to only one final node in the reals directory
 * For these cases, its to_reals allows duplication, it would check for existence and only add it if its not present
 * The current problem is that a node's absolute path is not the only used component for hashing everywhere
 * We are also handling the python interpreter separately
 * Internally, the `DistNode` attached to the node is responsible for every other behavior in a polymorphic sense
 */
fn get_all_mods(manifest: &YarpManifest, cwd: &PathBuf) -> Result<()> {
    // we have a list of pure modules, which i honestly dont care about right now
    // we have a list of site packages
    // each node is actually simply recognised by the path, nothing else
    // the data structures now are confusing, need to checkout how i can make this easier to work with
    unimplemented!()
}

fn gather_all_nodes(manifest: &YarpManifest, cwd: &PathBuf) -> Result<()> {
    let mut done: Done = HashSet::new();
    let exe = &manifest.python.sys.executable;
    let loads = get_nodes(&manifest.loads, exe, cwd, &mut done)?;
    let pures = get_nodes(&manifest.modules.pure, &exe, cwd, &mut done)?;
    let exts = get_nodes(&manifest.modules.extensions, &exe, cwd, &mut done)?;

    Ok(())
}

fn get_nodes(
    loads: &Vec<impl AssociatedFile>,
    executable_path: &str,
    cwd: &PathBuf,
    done: &mut Done,
) -> Result<Vec<DistNode>> {
    let mut res = Vec::new();
    for load in loads {
        let maybe_node = get_node_from_path(load.get_path(), executable_path, cwd);
        match maybe_node {
            None => {}
            Some(node_or_err) => match node_or_err {
                Err(e) => return Err(e),
                Ok(node) => {
                    done.insert(Rc::clone(&node.node));
                    res.push(node)
                }
            },
        };
    }
    Ok(res)
}

fn get_node_from_path(p: &str, executable_path: &str, cwd: &PathBuf) -> Option<Result<DistNode>> {
    match path_type(p) {
        PathType::PyFile => Some(mk_py_file_node(PathBuf::from(p))),
        PathType::SharedLibrary => Some(mk_shared_lib_node(executable_path, &cwd, p)),
        PathType::Unknown => {
            warn!("unknown file type in loads, ignoring, path={}", p);
            None
        }
    }
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
            lib_path: dylib.path.clone(),
        },
    });
    Ok(DistNode {
        node: lib_node,
        dist_file: Rc::new(dylib),
    })
}

fn mk_py_file_node(path: PathBuf) -> Result<DistNode> {
    let py_file = PyFileInSitePackages { path: path.clone() };
    let node = Rc::new(Node {
        kind: Kind::PyFile {
            src_path: path.clone(),
        },
    });
    Ok(DistNode {
        node,
        dist_file: Rc::new(py_file),
    })
}
