// use core::unimplemented;
// // given a yarp manifest, gather all the nodes that we can discover
use std::{collections::HashMap, path::PathBuf};

use anyhow::{Result, anyhow, bail};
use log::info;
use pathdiff::diff_paths;
use rand::{Rng, rng};
use walkdir::WalkDir;

mod site_pkgs_comp;
pub use site_pkgs_comp::PythonPathComponent;

use crate::{
    gather::site_pkgs_comp::get_python_path_mapping,
    graph::FileGraph,
    manifest::{Env, Sys, Version, YarpManifest},
    node::{Node, Pkg, PkgSitePackages, PrefixPackages, deps::Deps},
    pkg::paths::is_shared_library,
};

pub fn build_graph_from_manifest(
    manifest: &Box<YarpManifest>,
    cwd: &PathBuf,
) -> Result<(FileGraph, Vec<PythonPathComponent>)> {
    let (nodes, path_components) = get_python_universe(manifest, cwd)?;
    let g = build_graph(
        nodes,
        manifest.python.sys.executable.clone(),
        cwd.clone(),
        manifest.env.clone(),
    )?;
    Ok((g, path_components))
}

fn build_graph(
    nodes: Vec<Node>,
    executable_path: PathBuf,
    cwd: PathBuf,
    env: Env,
) -> Result<FileGraph> {
    info!(
        "building graph, number of nodes={} executable_path={} cwd={}",
        nodes.len(),
        executable_path.display(),
        cwd.display()
    );
    let mut g = FileGraph::new(executable_path, cwd, env);
    for node in &nodes {
        g.add_node(node.clone());
    }
    for node in nodes {
        g.add_tree(node)?;
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
) -> Result<(Vec<Node>, Vec<PythonPathComponent>)> {
    // given a manifest, we have exec_prefix, prefix and site-packages
    // go through all recursively and get corresponding nodes for each
    let mut nodes = Vec::new();
    let sys = &manifest.python.sys;

    nodes.push(get_executable_node(
        &sys.executable,
        cwd,
        &manifest.env.dyld_library_path,
    )?);

    // the order of nodes collection is important
    // exec prefix and prefix can be inside site-packages
    // in this case, we want to put nodes with exec-prefix destination identity before site-pkgs identity
    // graph only cares about the path of the node
    // any duplicate paths in site-packages which repeat from prefix would be ignored
    // i might need to revert this decision
    // and use some other sort of variable for node hashing other than path too (some enum maybe?)

    // exec_prefix can be inside prefix, do that first
    let lib_dynload_path = get_lib_dynload_loc(sys);
    info!(
        "gathering files from exec_prefix, exec_prefix={}, lib-dynload={}",
        sys.exec_prefix.display(),
        lib_dynload_path.display()
    );
    nodes.extend(get_nodes_recursive(
        &lib_dynload_path,
        &sys.version,
        &sys.executable,
        cwd,
        &manifest.env.dyld_library_path,
        &random_string(),
        get_exec_prefix_pkg,
    )?);

    // prefix now
    let stdlib_path = get_stdlib_loc(sys);
    info!(
        "gathering files from stdlib, prefix={} stdlib={}",
        sys.prefix.display(),
        stdlib_path.display()
    );
    nodes.extend(get_nodes_recursive(
        &stdlib_path,
        &sys.version,
        &sys.executable,
        cwd,
        &manifest.env.dyld_library_path,
        &random_string(),
        get_prefix_pkg,
    )?);

    // do site-packages in the end
    let site_pkgs = get_site_pkgs_without_prefixes(&sys.path, &lib_dynload_path, &stdlib_path);
    let site_pkgs = only_top_level_site_pkgs(&site_pkgs, &lib_dynload_path, &stdlib_path);
    let site_pkg_by_alias = create_site_pkgs_alias(&site_pkgs);

    info!(
        "gathering files from site-packages: {:?} original={:?}",
        site_pkgs, sys.path
    );
    let mut site_pkg_aliases = std::collections::HashMap::new();
    for site_pkg in &site_pkgs {
        site_pkg_aliases.insert(site_pkg, random_string());
    }
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
        nodes.extend(get_nodes_recursive(
            site_pkg,
            &sys.version,
            &sys.executable,
            cwd,
            &manifest.env.dyld_library_path,
            site_pkg_by_alias.get(site_pkg).expect(&format!("fatal: could not find alias for site-packages, site_pkg={:?} site-packages-map={:?}", site_pkg, site_pkg_by_alias)),
            get_site_packages_pkg,
        )?);
    }

    nodes.extend(get_loaded_nodes_from_manifest(
        manifest,
        &sys.executable,
        cwd,
        &manifest.env.dyld_library_path,
    )?);

    let py_path_comps = get_python_path_mapping(
        &site_pkg_by_alias,
        &stdlib_path,
        &lib_dynload_path,
        &sys.path,
    );
    Ok((nodes, py_path_comps))
}

fn get_loaded_nodes_from_manifest(
    manifest: &YarpManifest,
    executable_path: &PathBuf,
    cwd: &PathBuf,
    dyld_library_path: &Vec<PathBuf>,
) -> Result<Vec<Node>> {
    let mut res = Vec::new();
    for p in &manifest.loads {
        res.push(Node::new(
            p.path.clone(),
            Pkg::BinaryInLDPath,
            Deps::new_binary(&p.path, executable_path, cwd, dyld_library_path)?,
        ))
    }
    for p in &manifest.modules.extensions {
        res.push(Node::new(
            p.path.clone(),
            Pkg::BinaryInLDPath,
            Deps::new_binary(&p.path, executable_path, cwd, dyld_library_path)?,
        ))
    }

    Ok(res)
}

fn create_site_pkgs_alias(site_pkgs: &Vec<PathBuf>) -> HashMap<PathBuf, String> {
    let mut site_pkg_aliases = std::collections::HashMap::new();
    for site_pkg in site_pkgs {
        site_pkg_aliases.insert(site_pkg.clone(), random_string());
    }
    site_pkg_aliases
}

fn get_executable_node(
    executable_path: &PathBuf,
    cwd: &PathBuf,
    dyld_library_path: &Vec<PathBuf>,
) -> Result<Node> {
    let p = executable_path.clone();
    Ok(Node::new(
        p,
        Pkg::Executable,
        Deps::new_binary(executable_path, executable_path, cwd, dyld_library_path)?,
    ))
}

fn get_nodes_recursive<F>(
    directory: &PathBuf,
    version: &Version,
    executable_path: &PathBuf,
    cwd: &PathBuf,
    dyld_library_path: &Vec<PathBuf>,
    alias: &String,
    callback: F,
) -> Result<Vec<Node>>
where
    F: Fn(&PathBuf, &PathBuf, &str, &Version) -> Result<Pkg>,
{
    if !directory.exists() {
        bail!(
            "fatal: tried finding nodes recursively for directory={}, but it does not exist",
            directory.display()
        );
    }
    let paths = get_paths_recursive_from_dir(directory)?;
    let mut nodes = Vec::with_capacity(paths.len());
    for p in paths {
        let pkg = callback(&p, directory, alias, version)?;
        let deps = Deps::from_path(&p, executable_path, cwd, dyld_library_path)?;
        nodes.push(Node::new(p, pkg, deps));
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

fn get_exec_prefix_pkg(
    path: &PathBuf,
    original_prefix: &PathBuf,
    _alias: &str,
    version: &Version,
) -> Result<Pkg> {
    let rel_path = diff_paths(&path, &original_prefix).ok_or_else(|| {
        anyhow!(
            "failed in finding relative path of file inside prefix file={} prefix={}",
            path.display(),
            original_prefix.display()
        )
    })?;
    let prefix_pkg = PrefixPackages {
        original_prefix: original_prefix.clone(),
        version: version.clone(),
        rel_path,
    };
    if is_shared_library(path) {
        Ok(Pkg::ExecPrefixBinary(prefix_pkg))
    } else {
        Ok(Pkg::ExecPrefixPlain(prefix_pkg))
    }
}

fn get_prefix_pkg(
    path: &PathBuf,
    original_prefix: &PathBuf,
    _alias: &str,
    version: &Version,
) -> Result<Pkg> {
    let rel_path = diff_paths(&path, &original_prefix).ok_or_else(|| {
        anyhow!(
            "failed in finding relative path of file inside prefix file={} prefix={}",
            path.display(),
            original_prefix.display()
        )
    })?;
    let prefix_pkg = PrefixPackages {
        original_prefix: original_prefix.clone(),
        version: version.clone(),
        rel_path,
    };
    if is_shared_library(path) {
        Ok(Pkg::PrefixBinary(prefix_pkg))
    } else {
        Ok(Pkg::PrefixPlain(prefix_pkg))
    }
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

fn get_site_packages_pkg(
    path: &PathBuf,
    site_pkg_path: &PathBuf,
    alias: &str,
    _version: &Version,
) -> Result<Pkg> {
    let rel_path = diff_paths(&path, &site_pkg_path).ok_or_else(|| {
        anyhow!(
            "failed in finding relative path of file inside site-packages, file={} site-packages={}",
            path.display(),
            site_pkg_path.display()
        )
    })?;
    let res = PkgSitePackages {
        site_packages: site_pkg_path.clone(),
        alias: alias.to_string(),
        rel_path: rel_path,
    };
    if is_shared_library(path) {
        Ok(Pkg::SitePackagesBinary(res))
    } else {
        Ok(Pkg::SitePackagesPlain(res))
    }
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
