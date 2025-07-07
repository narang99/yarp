// use core::unimplemented;
// // given a yarp manifest, gather all the nodes that we can discover
use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use log::info;
use walkdir::WalkDir;

mod make;
pub use crate::factory::{Factory, NodeFactory, NodeSpec};
pub use crate::site_pkgs::PythonPathComponent;

use crate::{
    gather::make::mk_nodes_parallel,
    graph::FileGraph,
    manifest::{Env, Skip, YarpManifest},
    node::{Node, deps::Deps},
    paths::is_sys_lib,
    site_pkgs::SitePkgs,
};

pub fn build_graph_from_manifest(
    manifest: &Box<YarpManifest>,
    cwd: &PathBuf,
) -> Result<(FileGraph<NodeFactory>, Vec<PythonPathComponent>)> {
    let site_pkgs = SitePkgs::from_manifest(manifest);
    let factory = NodeFactory::new(
        site_pkgs.clone(),
        manifest.python.sys.version.clone(),
        manifest.python.sys.executable.clone(),
        cwd.clone(),
        manifest.env.clone(),
    );
    let (nodes, mut known_libs) = get_python_universe(manifest, cwd, &site_pkgs, &factory)?;
    let g = build_graph(
        nodes,
        manifest.python.sys.executable.clone(),
        cwd.clone(),
        manifest.env.clone(),
        &mut known_libs,
        factory,
    )?;
    Ok((g, site_pkgs.comps))
}

fn build_graph(
    nodes: Vec<Node>,
    executable_path: PathBuf,
    cwd: PathBuf,
    env: Env,
    known_libs: &mut HashMap<String, PathBuf>,
    factory: NodeFactory,
) -> Result<FileGraph<NodeFactory>> {
    info!(
        "building graph, number of nodes={} executable_path={} cwd={}",
        nodes.len(),
        executable_path.display(),
        cwd.display()
    );
    let mut g = FileGraph::new(executable_path, cwd, env, factory);
    info!("Build graph: pass 1, begin");
    let mut failures = Vec::new();

    for node in &nodes {
        g.add_node(node.clone());
    }

    let mut i = 0;
    let total = nodes.len();
    for node in nodes {
        match g.add_tree(node.clone(), &known_libs) {
            Ok(_) => {}
            Err(_) => {
                failures.push(node);
            }
        }
        i += 1;
        if i % (total / 10) == 0 {
            info!("graph: pass 1: {}/{} nodes", i, total);
        }
    }
    info!(
        "Build graph: pass 2, begin, number of nodes that failed to add: {}",
        failures.len()
    );
    for node in g.iter_nodes() {
        let (file_name, path) = get_lib_from_node(node)?;
        known_libs.insert(file_name, path);
    }
    for node in failures {
        g.add_tree(node, &known_libs)?;
    }

    info!(
        "graph built successfully, nodes inserted in graph={}",
        g.len()
    );
    Ok(g)
}

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
pub fn get_python_universe(
    manifest: &YarpManifest,
    cwd: &PathBuf,
    site_pkgs: &SitePkgs,
    factory: &NodeFactory,
) -> Result<(Vec<Node>, HashMap<String, PathBuf>)> {
    info!("gather files: Pass 1, begin");
    let create_nodes = get_node_specs(manifest, &factory, &site_pkgs)?;
    let (mut res, failures) = mk_nodes_parallel(&create_nodes, factory);

    info!(
        "gather files: Pass 2, begin, number of nodes to pass again: {}",
        failures.len()
    );
    let mut known_libs = get_libs_scanned_in_nodes(&res)?;
    for payload in failures {
        let node = factory.make_from_spec(&payload, &known_libs)?;
        let (file_name, path) = get_lib_from_node(&node)?;
        known_libs.insert(file_name, path);

        res.push(node);
    }
    info!("gather files done");
    Ok((res, known_libs))
}

fn should_skip(path: &PathBuf, skip: &Skip) -> bool {
    for prefix in &skip.prefixes {
        if path.starts_with(prefix) {
            return true;
        }
    }
    false
}

fn get_libs_scanned_in_nodes(nodes: &Vec<Node>) -> Result<HashMap<String, PathBuf>> {
    let mut res = HashMap::new();
    for node in nodes {
        if let Deps::Binary(_) = node.deps {
            let (file_name, path) = get_lib_from_node(node)?;
            res.insert(file_name, path);
        }
    }

    Ok(res)
}

fn get_lib_from_node(node: &Node) -> Result<(String, PathBuf)> {
    let file_name = node.path.file_name().expect(&format!(
        "fatal error, impossible: found a node whose file_name could not be generated, path={}",
        node.path.display()
    ));
    let file_name = file_name.to_str().with_context(|| {
        anyhow!(
            "failed in converting file_name to string, path={}",
            file_name.display()
        )
    })?;
    Ok((file_name.to_string(), node.path.clone()))
}

fn get_node_specs(
    manifest: &YarpManifest,
    factory: &NodeFactory,
    site_pkgs: &SitePkgs,
) -> Result<Vec<NodeSpec>> {
    let mut res = Vec::new();
    let sys = &manifest.python.sys;

    res.push(NodeSpec::Executable {
        path: sys.executable.clone(),
    });

    res.extend(get_node_specs_recursive(
        &site_pkgs.lib_dynload,
        &factory,
        &manifest.skip,
    )?);

    res.extend(get_node_specs_recursive(
        &site_pkgs.stdlib,
        &factory,
        &manifest.skip,
    )?);

    for site_pkg in &site_pkgs.resolved {
        if !site_pkg.exists() {
            // its valid for a pythonpath to not exist
            info!(
                "site-packages {} does not exist, ignoring",
                site_pkg.display()
            );
            continue;
        }
        res.extend(get_node_specs_recursive(
            site_pkg,
            &factory,
            &manifest.skip,
        )?);
    }
    for p in &manifest.loads {
        if should_skip(&p.path, &manifest.skip) {
            continue;
        }
        res.push(NodeSpec::BinaryInLdPath {
            path: p.path.clone(),
            symlinks: p.symlinks.clone(),
        });
    }
    for p in &manifest.libs {
        if should_skip(&p.path, &manifest.skip) {
            continue;
        }
        let is_sys_library = p.path.to_str().map_or(false, |p| is_sys_lib(p));
        if is_sys_library {
            continue;
        }
        res.push(NodeSpec::Binary {
            path: p.path.clone(),
        });
    }
    Ok(res)
}

fn get_node_specs_recursive(
    directory: &PathBuf,
    factory: &NodeFactory,
    skip: &Skip,
) -> Result<Vec<NodeSpec>> {
    if !directory.exists() {
        bail!(
            "fatal: tried finding nodes recursively for directory={}, but it does not exist",
            directory.display()
        );
    }
    let paths = get_paths_recursive_from_dir(directory)?;
    let mut nodes = Vec::with_capacity(paths.len());
    for p in paths {
        if should_skip(&p, skip) {
            continue;
        }
        nodes.push(factory.make_spec(&p)?);
    }
    Ok(nodes)
}

fn get_paths_recursive_from_dir(base_path: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for maybe_d in WalkDir::new(base_path).into_iter() {
        match maybe_d {
            Ok(d) => {
                let p = d.into_path();
                if p.is_file() {
                    paths.push(p);
                }
            }
            Err(e) => {
                return Err(e)?;
            }
        }
    }
    Ok(paths)
}
