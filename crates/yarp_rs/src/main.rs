use std::env;


use log::info;

use crate::{gather::build_graph_from_manifest, manifest::YarpManifest, pkg::{move_to_dist, patch::patch_lib}};

pub mod pkg;
pub mod gather;
pub mod graph;
pub mod manifest;
pub mod node;



/**
 * Algorithm:
 * - enumerate all files that are part of this python package
 * - create a node for each of these that can be inserted in the graph
 * - insert all nodes in the graph, without doing dependency analysis
 * - add all nodes in the second pass, but with dependency analysis this time, creating edges
 *   - this operation can add new nodes that are not present in the graph too
 * - Topologically sort the graph, call to_destination for each node
 * - create bootstrap scripts
 * 
 * All file types
 * - python file inside a site-packages folder
 * - unknown file inside a site-packages folder
 *   - both have same handling (plain file basically)
 * - plain file inside prefix
 *   - the handling is similar to site-packages, other than destination in final directory
 * - dylib inside prefix
 *   - handling same as dylib, but destination different
 * - dylib inside exec_prefix
 *   - again, handling same as dylib, but destination different
 * - dylib in site-packages
 *   - again, handling same as dylib, but destination different
 * - might be useful to have a destination object in these files i think
 * - finally we have pythonexe, which has same handling as dylib, but destination is different
 * 
 * We call all of these binary file
 * 
 * 
 * - binary file (with destination)
 * - plain file (with destination)
 * 
 * reals is same for both, how they are symlinked to correct location is different
 */



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
    let graph = build_graph_from_manifest(&manifest, &cwd).expect("failed in building graph");
    let dist = cwd.join("dist");
    info!("moving files to dist");
    for node in graph.toposort().unwrap() {
        let deps = graph.get_node_dependencies(&node);
        move_to_dist(&node, &deps, &dist).unwrap();
    }
}
