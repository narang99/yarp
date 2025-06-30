use core::unimplemented;
// given a yarp manifest, gather all the nodes that we can discover
use std::{collections::HashSet, path::PathBuf, rc::Rc};

use anyhow::Result;
use log::warn;

use crate::{
    ftypes::{BinaryFile, PlainFile},
    manifest::YarpManifest,
    node::{DistNode, Node},
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
fn get_python_universe(manifest: &YarpManifest, cwd: &PathBuf) -> Result<()> {
    // given a manifest, we have exec_prefix, prefix and site-packages
    // go through all recursively and get corresponding nodes for each
    unimplemented!()
}

fn gather_all_nodes(manifest: &YarpManifest, cwd: &PathBuf) -> Result<()> {
    let mut done: Done = HashSet::new();
    let exe = &manifest.python.sys.executable;
    let loads = get_nodes(
        &manifest
            .loads
            .iter()
            .map(|load| load.path.clone())
            .collect(),
        exe,
        cwd,
        &mut done,
    )?;
    let exts = get_nodes(
        &manifest
            .modules
            .extensions
            .iter()
            .map(|ext| ext.path.clone())
            .collect(),
        &exe,
        cwd,
        &mut done,
    )?;

    Ok(())
}

fn get_nodes(
    paths: &Vec<PathBuf>,
    executable_path: &PathBuf,
    cwd: &PathBuf,
    done: &mut Done,
) -> Result<Vec<DistNode>> {
    let mut res = Vec::new();
    for path in paths {
        let maybe_node = get_node_from_path(path, executable_path, cwd);
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

fn get_node_from_path(
    p: &PathBuf,
    executable_path: &PathBuf,
    cwd: &PathBuf,
) -> Option<Result<DistNode>> {
    match path_type(p) {
        PathType::PyFile => Some(mk_py_file_node(p)),
        PathType::SharedLibrary => Some(mk_shared_lib_node(executable_path, &cwd, p)),
        PathType::Unknown => {
            warn!("unknown file type in loads, ignoring, path={}", p.display());
            None
        }
    }
}

enum PathType {
    PyFile,
    SharedLibrary,
    Unknown,
}

fn path_type(p: &PathBuf) -> PathType {
    if p.ends_with(".py") {
        PathType::PyFile
    } else if p.ends_with(".so") || p.ends_with(".dylib") {
        PathType::SharedLibrary
    } else {
        PathType::Unknown
    }
}

fn mk_py_exe_node(executable_path: &PathBuf, cwd: &PathBuf) -> Result<DistNode> {
    let py_node = Rc::new(Node {
        path: executable_path.clone(),
    });
    let python_exe = BinaryFile {
        executable_path: executable_path.clone(),
        path: executable_path.clone(),
        cwd: cwd.clone(),
    };
    Ok(DistNode {
        node: py_node,
        dist_file: Rc::new(python_exe),
    })
}

fn mk_shared_lib_node(
    executable_path: &PathBuf,
    cwd: &PathBuf,
    lib_path: &PathBuf,
) -> Result<DistNode> {
    let dylib = BinaryFile {
        executable_path: executable_path.clone(),
        cwd: cwd.clone(),
        path: lib_path.clone(),
    };
    let lib_node = Rc::new(Node {
        path: dylib.path.clone(),
    });
    Ok(DistNode {
        node: lib_node,
        dist_file: Rc::new(dylib),
    })
}

fn mk_py_file_node(path: &PathBuf) -> Result<DistNode> {
    let py_file = PlainFile { path: path.clone() };
    let node = Rc::new(Node { path: path.clone() });
    Ok(DistNode {
        node,
        dist_file: Rc::new(py_file),
    })
}
