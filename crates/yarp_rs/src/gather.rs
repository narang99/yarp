// use core::unimplemented;
// // given a yarp manifest, gather all the nodes that we can discover
// use std::{collections::HashSet, path::PathBuf, rc::Rc};

// use anyhow::Result;
// use log::warn;
// use walkdir::{DirEntry, WalkDir};

// use crate::{
//     ftypes::{BinaryFile, PlainFile},
//     manifest::YarpManifest,
//     node::{Node, GraphNode},
// };

// type Done = HashSet<Rc<GraphNode>>;

// /**
//  * Currently, we have nodes in a graph. Its an enum
//  * Each node is also hashable
//  * When we add a node, we also add its dependents
//  * The identity of a node is ALWAYS its absolute path (for all of them)
//  * For shared libraries, we have an edge case. Each node is symlinked to only one final node in the reals directory
//  * For these cases, its to_reals allows duplication, it would check for existence and only add it if its not present
//  * The current problem is that a node's absolute path is not the only used component for hashing everywhere
//  * We are also handling the python interpreter separately
//  * Internally, the `DistNode` attached to the node is responsible for every other behavior in a polymorphic sense
//  */
// pub fn get_python_universe(manifest: &YarpManifest, cwd: &PathBuf) -> Result<()> {
//     // given a manifest, we have exec_prefix, prefix and site-packages
//     // go through all recursively and get corresponding nodes for each
//     let mut nodes = Vec::new();
//     let sys = &manifest.python.sys;
//     let exec_path = &manifest.python.sys.executable;

//     for site_pkg in &sys.path {
//         let paths = get_paths_recursive_from_dir(&site_pkg)?;
//         nodes.extend(get_nodes(&paths, exec_path, cwd));
//     }

//     Ok(())
// }


// fn get_paths_recursive_from_dir(base_path: &PathBuf) -> Result<Vec<PathBuf>> {
//     let mut paths = Vec::new();
//     for maybe_d in WalkDir::new(base_path) {
//         match maybe_d {
//             Ok(d) => {
//                 paths.push(d.path().to_path_buf());
//             }
//             Err(e) => {
//                 return Err(e)?;
//             }
//         }
//     }
//     Ok(paths)
// }

// // fn gather_all_nodes(manifest: &YarpManifest, cwd: &PathBuf) -> Result<()> {
// //     let mut done: Done = HashSet::new();
// //     let exe = &manifest.python.sys.executable;
// //     let loads = get_nodes(
// //         &manifest
// //             .loads
// //             .iter()
// //             .map(|load| load.path.clone())
// //             .collect(),
// //         exe,
// //         cwd,
// //         &mut done,
// //     )?;
// //     let exts = get_nodes(
// //         &manifest
// //             .modules
// //             .extensions
// //             .iter()
// //             .map(|ext| ext.path.clone())
// //             .collect(),
// //         &exe,
// //         cwd,
// //         &mut done,
// //     )?;

// //     Ok(())
// // }

// fn get_nodes(
//     paths: &Vec<PathBuf>,
//     executable_path: &PathBuf,
//     cwd: &PathBuf,
// ) -> Result<Vec<Node>> {
//     let mut res = Vec::new();
//     for path in paths {
//         let maybe_node = get_node_from_path(path, executable_path, cwd);
//         match maybe_node {
//             None => {}
//             Some(node_or_err) => match node_or_err {
//                 Err(e) => return Err(e),
//                 Ok(node) => {
//                     res.push(node)
//                 }
//             },
//         };
//     }
//     Ok(res)
// }

// fn get_node_from_path(
//     p: &PathBuf,
//     executable_path: &PathBuf,
//     cwd: &PathBuf,
// ) -> Option<Result<Node>> {
//     match path_type(p) {
//         PathType::PyFile => Some(mk_py_file_node(p)),
//         PathType::SharedLibrary => Some(mk_shared_lib_node(executable_path, &cwd, p)),
//         PathType::Unknown => {
//             warn!("unknown file type in loads, ignoring, path={}", p.display());
//             None
//         }
//     }
// }

// enum PathType {
//     PyFile,
//     SharedLibrary,
//     Unknown,
// }

// fn path_type(p: &PathBuf) -> PathType {
//     if p.ends_with(".py") {
//         PathType::PyFile
//     } else if p.ends_with(".so") || p.ends_with(".dylib") {
//         PathType::SharedLibrary
//     } else {
//         PathType::Unknown
//     }
// }

// fn mk_shared_lib_node(
//     executable_path: &PathBuf,
//     cwd: &PathBuf,
//     lib_path: &PathBuf,
// ) -> Result<Node> {
//     let dylib = BinaryFile {
//         executable_path: executable_path.clone(),
//         cwd: cwd.clone(),
//         path: lib_path.clone(),
//     };
//     let lib_node = Rc::new(GraphNode {
//         path: dylib.path.clone(),
//     });
//     Ok(Node {
//         node: lib_node,
//         deps: Rc::new(dylib),
//     })
// }

// fn mk_py_file_node(path: &PathBuf) -> Result<Node> {
//     let py_file = PlainFile { path: path.clone() };
//     let node = Rc::new(GraphNode { path: path.clone() });
//     Ok(Node {
//         node,
//         deps: Rc::new(py_file),
//     })
// }
