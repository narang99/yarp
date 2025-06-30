use std::{env, fmt::format, path::PathBuf, rc::Rc};

use anyhow::{Result, anyhow};
use log::{info, warn};

use crate::{
    ftypes::{Dylib, PyFileInSitePackages, PythonExe},
    graph::FileGraph,
    manifest::{Load, YarpManifest},
    node::{DistNode, Kind, Node},
};

pub mod ftypes;
pub mod gather;
pub mod graph;
pub mod macho;
pub mod manifest;
pub mod node;

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
}
