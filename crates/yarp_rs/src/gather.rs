// use core::unimplemented;
// // given a yarp manifest, gather all the nodes that we can discover
use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use log::info;
use rand::{Rng, rng};
use rayon::current_thread_index;
use walkdir::WalkDir;

mod node_factory;
mod site_pkgs_comp;
pub use site_pkgs_comp::PythonPathComponent;

use crate::{
    gather::{
        node_factory::{CreateNode, generate_node},
        site_pkgs_comp::get_python_path_mapping,
    },
    graph::FileGraph,
    manifest::{Env, Skip, Sys, Version, YarpManifest},
    node::{Node, deps::Deps},
    paths::{get_dyld_library_path, is_sys_lib},
};

pub fn build_graph_from_manifest(
    manifest: &Box<YarpManifest>,
    cwd: &PathBuf,
) -> Result<(FileGraph, Vec<PythonPathComponent>)> {
    let (nodes, path_components, mut known_libs) = get_python_universe(manifest, cwd)?;
    let g = build_graph(
        nodes,
        manifest.python.sys.executable.clone(),
        cwd.clone(),
        manifest.env.clone(),
        &mut known_libs,
    )?;
    Ok((g, path_components))
}

fn build_graph(
    nodes: Vec<Node>,
    executable_path: PathBuf,
    cwd: PathBuf,
    env: Env,
    known_libs: &mut HashMap<String, PathBuf>,
) -> Result<FileGraph> {
    info!(
        "building graph, number of nodes={} executable_path={} cwd={}",
        nodes.len(),
        executable_path.display(),
        cwd.display()
    );
    let mut g = FileGraph::new(executable_path, cwd, env);
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
) -> Result<(
    Vec<Node>,
    Vec<PythonPathComponent>,
    HashMap<String, PathBuf>,
)> {
    let dyld_library_path = get_dyld_library_path(&manifest.env);
    Deps::from_path(
        &PathBuf::from(
            "/Users/hariomnarang/miniconda3/envs/platform/lib/python3.9/site-packages/torch/lib/libtorch.dylib",
        ),
        &manifest.python.sys.executable,
        cwd,
        &dyld_library_path,
        &HashMap::new(),
    )?;
    info!("gather files: Pass 1, begin");
    let (create_nodes, py_comps) = get_create_node_payloads(manifest)?;
    let (mut res, failures) = mk_nodes_parallel(&create_nodes, manifest, cwd, &dyld_library_path);

    info!(
        "gather files: Pass 2, begin, number of nodes to pass again: {}",
        failures.len()
    );
    let mut known_libs = get_libs_scanned_in_nodes(&res)?;
    for payload in failures {
        let node = generate_node(
            &payload,
            &manifest.python.sys.executable,
            cwd,
            &dyld_library_path,
            &known_libs,
        )?;
        let (file_name, path) = get_lib_from_node(&node)?;
        known_libs.insert(file_name, path);

        res.push(node);
    }
    info!("gather files done");
    Ok((res, py_comps, known_libs))
}

fn mk_nodes_parallel(
    create_nodes: &Vec<CreateNode>,
    manifest: &YarpManifest,
    cwd: &PathBuf,
    dyld_library_path: &Vec<PathBuf>,
) -> (Vec<Node>, Vec<CreateNode>) {
    use rayon::prelude::*;

    let empty_known_libs = HashMap::new();
    let mut res = Vec::new();
    let mut failures = Vec::new();

    let num_threads = rayon::current_num_threads();
    let chunk_size = (create_nodes.len() + num_threads - 1) / num_threads;
    info!("gather: creating nodes, chunk_size={} threads={}", chunk_size, num_threads);

    let results: Vec<(Vec<Node>, Vec<CreateNode>)> = create_nodes
        .par_chunks(chunk_size)
        .map(|chunk| {
            let thread_idx = current_thread_index().unwrap_or(0);
            let mut local_res = Vec::new();
            let mut local_failures = Vec::new();
            let mut i = 0;
            let total = chunk.len();
            for payload in chunk {
                let node = generate_node(
                    payload,
                    &manifest.python.sys.executable,
                    cwd,
                    dyld_library_path,
                    &empty_known_libs,
                );
                match node {
                    Ok(node) => {
                        local_res.push(node);
                    }
                    Err(_) => {
                        local_failures.push(payload.clone());
                    }
                }
                i += 1;
                if i % (total / 10) == 0 {
                    info!("thread: {} exported {}/{} files", thread_idx, i, total);
                }
            }
            (local_res, local_failures)
        })
        .collect();

    for (local_res, local_failures) in results {
        res.extend(local_res);
        failures.extend(local_failures);
    }

    (res, failures)
}

fn should_skip(path: &PathBuf, skip: &Skip) -> bool {
    for prefix in &skip.path_prefixes {
        if path.starts_with(prefix) {
            return true;
        }
    }
    false
}

fn get_libs_scanned_in_nodes(nodes: &Vec<Node>) -> Result<HashMap<String, PathBuf>> {
    let mut res = HashMap::new();
    for node in nodes {
        let (file_name, path) = get_lib_from_node(node)?;
        res.insert(file_name, path);
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

fn get_create_node_payloads(
    manifest: &YarpManifest,
) -> Result<(Vec<CreateNode>, Vec<PythonPathComponent>)> {
    let mut res = Vec::new();
    let sys = &manifest.python.sys;

    res.push(CreateNode::Executable {
        path: sys.executable.clone(),
    });

    let lib_dynload_path = get_lib_dynload_loc(sys);
    res.extend(get_create_nodes_payload_recursive(
        &lib_dynload_path,
        &sys.version,
        &random_string(),
        CreateNodeToMake::ExecPrefix,
        &manifest.skip,
    )?);

    let stdlib_path = get_stdlib_loc(sys);
    res.extend(get_create_nodes_payload_recursive(
        &stdlib_path,
        &sys.version,
        &random_string(),
        CreateNodeToMake::Prefix,
        &manifest.skip,
    )?);

    // do site-packages in the end
    let site_pkgs = get_site_pkgs_without_prefixes(&sys.path, &lib_dynload_path, &stdlib_path);
    let site_pkgs = only_top_level_site_pkgs(&site_pkgs, &lib_dynload_path, &stdlib_path);
    let site_pkg_by_alias = create_site_pkgs_alias(&site_pkgs);

    for site_pkg in &site_pkgs {
        if !site_pkg.exists() {
            // its valid for a pythonpath to not exist
            info!(
                "site-packages {} does not exist, ignoring",
                site_pkg.display()
            );
            continue;
        }
        info!(
            "gathering files from site-packages, site-package={} alias={:?}",
            site_pkg.display(),
            site_pkg_by_alias.get(site_pkg)
        );
        res.extend(get_create_nodes_payload_recursive(
            site_pkg,
            &sys.version,
            site_pkg_by_alias.get(site_pkg).expect(&format!("fatal: could not find alias for site-packages, site_pkg={:?} site-packages-map={:?}", site_pkg, site_pkg_by_alias)),
            CreateNodeToMake::SitePkg,
            &manifest.skip,
        )?);
    }
    for p in &manifest.loads {
        if should_skip(&p.path, &manifest.skip) {
            continue;
        }
        res.push(CreateNode::BinaryInLdPath {
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
        res.push(CreateNode::Binary {
            path: p.path.clone(),
        });
    }
    let py_path_comps = get_python_path_mapping(
        &site_pkg_by_alias,
        &stdlib_path,
        &lib_dynload_path,
        &sys.path,
    );
    Ok((res, py_path_comps))
}

fn create_site_pkgs_alias(site_pkgs: &Vec<PathBuf>) -> HashMap<PathBuf, String> {
    let mut site_pkg_aliases = std::collections::HashMap::new();
    for site_pkg in site_pkgs {
        site_pkg_aliases.insert(site_pkg.clone(), random_string());
    }
    site_pkg_aliases
}

enum CreateNodeToMake {
    SitePkg,
    Prefix,
    ExecPrefix,
}

fn get_create_nodes_payload_recursive(
    directory: &PathBuf,
    version: &Version,
    alias: &String,
    to_make: CreateNodeToMake,
    skip: &Skip,
) -> Result<Vec<CreateNode>> {
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
        let create_node = match to_make {
            CreateNodeToMake::Prefix => CreateNode::PrefixPkg {
                original_prefix: directory.clone(),
                alias: alias.clone(),
                version: version.clone(),
                path: p.clone(),
            },
            CreateNodeToMake::ExecPrefix => CreateNode::ExecPrefixPkg {
                original_prefix: directory.clone(),
                alias: alias.clone(),
                version: version.clone(),
                path: p.clone(),
            },
            CreateNodeToMake::SitePkg => CreateNode::SitePkg {
                site_pkg_path: directory.clone(),
                alias: alias.clone(),
                version: version.clone(),
                path: p.clone(),
            },
        };
        nodes.push(create_node);
    }
    Ok(nodes)
}

fn get_lib_dynload_loc(sys: &Sys) -> PathBuf {
    sys.exec_prefix
        .join(&sys.platlibdir)
        .join(sys.version.get_python_version())
        .join("lib-dynload")
}

fn get_stdlib_loc(sys: &Sys) -> PathBuf {
    sys.prefix
        .join(&sys.platlibdir)
        .join(sys.version.get_python_version())
}

fn get_site_pkgs_without_prefixes(
    site_pkgs: &Vec<PathBuf>,
    lib_dynload: &PathBuf,
    stdlib: &PathBuf,
) -> Vec<PathBuf> {
    site_pkgs
        .iter()
        .filter(|p| **p != *lib_dynload)
        .filter(|p| **p != *stdlib)
        .map(|p| p.clone())
        .collect()
}

fn only_top_level_site_pkgs(
    sys_path: &Vec<PathBuf>,
    lib_dynload: &PathBuf,
    stdlib: &PathBuf,
) -> Vec<PathBuf> {
    let mut all_paths_to_check: Vec<&PathBuf> = sys_path.iter().collect();
    all_paths_to_check.push(lib_dynload);
    all_paths_to_check.push(stdlib);

    sys_path
        .iter()
        .filter(|p| {
            let should_keep = !is_sub_path_of_other_pkgs(p, &all_paths_to_check);
            if !should_keep {
                info!("package {} is a nested site-package, ignoring", p.display());
            }
            should_keep
        })
        .map(|p| p.clone())
        .collect()
}

fn is_sub_path_of_other_pkgs(p: &PathBuf, sys_path: &Vec<&PathBuf>) -> bool {
    for other in sys_path {
        if *p != **other && p.starts_with(other) {
            return true;
        }
    }
    return false;
}

fn random_string() -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
    let mut rng = rng();

    (0..10)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
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
