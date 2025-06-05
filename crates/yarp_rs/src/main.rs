use std::{path::PathBuf, rc::Rc};

use lief::macho::Commands;

use crate::{
    ftypes::{Dylib, PythonExe},
    graph::FileGraph,
    node::{DistNode, Kind, Node},
};

pub mod ftypes;
pub mod graph;
pub mod macho;
pub mod manifest;
pub mod node;

fn main() {
    env_logger::init();
    let mut graph = FileGraph::new();

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
        path: PathBuf::from("/Users/hariomnarang/miniconda3/envs/platform/lib/libpango-1.0.0.dylib"),
    };
    let lib_node = Rc::new(Node {kind: Kind::SharedLibrary { name: "libpango-1.0.0.dylib".to_string() }});

    graph.add_node(DistNode{node: py_node, dist_file: Rc::new(python_exe)}).unwrap();
    graph.add_node(DistNode{node: lib_node, dist_file: Rc::new(top_lib)}).unwrap();
    println!("{}", graph);
}