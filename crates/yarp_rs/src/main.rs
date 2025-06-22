use std::{env, fmt::format, path::PathBuf, rc::Rc};

use anyhow::{Result, anyhow};
use log::{info, warn};

use crate::{
    ftypes::{Dylib, PyFile, PythonExe},
    graph::FileGraph,
    manifest::{Load, YarpManifest},
    node::{DistNode, Kind, Node},
};

pub mod ftypes;
pub mod graph;
pub mod macho;
pub mod manifest;
pub mod node;
pub mod gather;

fn main() {
    env_logger::init();
    let args: Vec<String> = std::env::args().collect();
    let yarp_manifest_path = args
        .get(1)
        .expect("Expected a single argument, the path the yarp manifest");
    let manifest_contents = std::fs::read_to_string(yarp_manifest_path).expect(&format!(
        "Failed to read yarp manifest file {}",
        yarp_manifest_path
    ));
    let manifest: YarpManifest =
        serde_json::from_str(&manifest_contents).expect("Failed to parse yarp manifest as JSON");
    let cwd = env::current_dir().unwrap();
    let py_exec = mk_py_exe_node(&manifest.python.sys.executable, &cwd).unwrap();
    let loads = get_load_nodes(&manifest.loads, &manifest.python.sys.executable, &cwd).unwrap();

    let py_node = Rc::new(Node { kind: Kind::Python });
    let py_path = PathBuf::from("/Users/hariomnarang/miniconda3/bin/python");
    let cwd = PathBuf::from("/Users/hariomnarang/Desktop/work/blog/linker/yarp/crates/yarp_rs");
    let python_exe = PythonExe {
        path: py_path.clone(),
        cwd: cwd.clone(),
    };

    let top_lib = Dylib {
        executable_path: py_path.clone(),
        cwd: cwd.clone(),
        path: PathBuf::from(
            "/Users/hariomnarang/miniconda3/envs/platform/lib/libpango-1.0.0.dylib",
        ),
    };
    let lib_node = Rc::new(Node {
        kind: Kind::SharedLibrary {
            name: "libpango-1.0.0.dylib".to_string(),
        },
    });

    graph
        .add_node(DistNode {
            node: py_node,
            dist_file: Rc::new(python_exe),
        })
        .unwrap();
    graph
        .add_node(DistNode {
            node: lib_node,
            dist_file: Rc::new(top_lib),
        })
        .unwrap();
    println!("{}", graph);
}

fn get_load_nodes(
    loads: &Vec<Load>,
    executable_path: &str,
    cwd: &PathBuf,
) -> Result<Vec<DistNode>> {
    let mut res = Vec::new();
    for load in loads {
        match path_type(&load.path) {
            PathType::PyFile => {
                res.push(mk_py_file_node(PathBuf::from(&load.path))?);
            }
            PathType::SharedLibrary => {
                res.push(mk_shared_lib_node(executable_path, &cwd, &load.path)?);
            }
            PathType::Unknown => {
                warn!("unknown file type in loads, ignoring, path={}", load.path);
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
