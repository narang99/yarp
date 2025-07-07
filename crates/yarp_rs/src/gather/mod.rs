// use core::unimplemented;
// // given a yarp manifest, gather all the nodes that we can discover
use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Error, Result, anyhow, bail};
use log::{info, warn};
use walkdir::WalkDir;

pub use crate::factory::NodeFactory;
pub use crate::site_pkgs::PythonPathComponent;

use crate::{
    factory::Factory,
    graph::FileGraph,
    manifest::{LoadKind, YarpManifest},
    node::{Node, deps::Deps},
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
        manifest.skip.prefixes.clone(),
    );
    let g = build_graph(manifest, cwd, &factory, &site_pkgs)?;

    Ok((g, site_pkgs.comps))
}

fn build_graph(
    manifest: &YarpManifest,
    cwd: &PathBuf,
    factory: &NodeFactory,
    site_pkgs: &SitePkgs,
) -> Result<FileGraph<NodeFactory>> {
    let executable_path = &manifest.python.sys.executable;
    let known_libs = HashMap::new();
    let env = &manifest.env;
    let mut g = FileGraph::new(
        executable_path.clone(),
        cwd.clone(),
        env.clone(),
        factory.clone(),
    );
    info!("Build graph: pass 1, begin");

    // first add the py executable and its whole tree, should not fail
    info!(
        "adding python executable, path={}",
        executable_path.display()
    );
    // TODO: add the executable's DT_RPATH to extra search paths for all libraries to load
    g.add_tree(factory.make_py_executable(executable_path)?, &known_libs, true)?;

    // now add all loads, in the correct order, again, should not fail
    for l in &manifest.loads {
        info!(
            "adding load detected in manifest, path={}",
            l.path.display()
        );
        match l.kind {
            LoadKind::Dlopen => factory
                .make_with_symlinks(&l.path, &l.symlinks, &known_libs)
                .and_then(|n| add_to_graph_if_some(&mut g, n, &known_libs, true))?,
            LoadKind::Extension => factory
                .make(&l.path, &known_libs)
                .and_then(|n| add_to_graph_if_some(&mut g, n, &known_libs, true))?,
        };
    }

    let mut failures = Vec::new();
    // add exec prefix, can fail
    info!("adding stdlib, path={}", site_pkgs.lib_dynload.display());
    add_nodes_recursive(
        &mut g,
        &mut failures,
        &site_pkgs.lib_dynload,
        &factory,
        &known_libs,
        true,
    )?;

    // add prefix, can fail
    info!("adding stdlib, path={}", site_pkgs.stdlib.display());
    add_nodes_recursive(
        &mut g,
        &mut failures,
        &site_pkgs.stdlib,
        &factory,
        &known_libs,
        true,
    )?;

    // now all site-packages, can fail
    for (pkg, _) in &site_pkgs.site_pkg_by_alias {
        info!("adding site-package: path={}", pkg.display());
        if pkg.exists() {
            // site-packages addition would replace
            add_nodes_recursive(&mut g, &mut failures, pkg, &factory, &known_libs, true)?;
        } else {
            info!(
                "site packages at path={} does not exist, skipping",
                pkg.display()
            );
        }
    }

    add_failures(&mut g, failures, &factory)?;

    Ok(g)
}

fn add_failures(
    g: &mut FileGraph<NodeFactory>,
    failures: Vec<PathBuf>,
    factory: &NodeFactory,
) -> Result<()> {
    // in each cycle, go through all the failures
    // add them to the graph
    // if any of them fail, keep them in the new failures vector
    // if failures do not decrease in a loop, then break and error out with all failures
    // else go to the next one with the new set of failures
    // if the failures are 0, break out
    // keep updating the known_libs values too
    // updating known_libs is a costly operation (it converts all paths stored in the graph to known_libs in every iteration)
    // TODO: fix known libs speed, mostly we should be able to fix it if the graph returns all the added nodes in add_tree instead of just the index
    let mut prev_failures: Vec<(PathBuf, Error)> = failures
        .iter()
        .map(|p| (p.clone(), anyhow!("unknown error")))
        .collect();
    let mut known_libs = get_libs_from_graph(g);

    let mut i = 0;

    let mut prev_len = prev_failures.len();
    while prev_len > 0 {
        i = i + 1;
        info!("adding failed nodes: Pass {}, length={}", i, prev_len);

        let mut new_failures = Vec::new();

        // failures addition does not recursively replace stuff in the graph
        for (p, _) in prev_failures {
            let res = factory
                .make(&p, &known_libs)
                .and_then(|n| add_to_graph_if_some(g, n, &known_libs, false));
            if let Err(e) = res {
                new_failures.push((p, e));
            }
        }

        if new_failures.len() >= prev_len {
            let errors: Vec<String> = new_failures
                .into_iter()
                .map(|(_, e)| format!("{:#}", e))
                .collect();
            bail!(
                "fatal failure in gather, could not find the dependencies of libraries. Errors:\n{}",
                errors.join("\n\n")
            );
        }

        prev_failures = new_failures;
        prev_len = prev_failures.len();

        known_libs = get_libs_from_graph(g);
    }
    Ok(())
}

fn add_nodes_recursive(
    g: &mut FileGraph<NodeFactory>,
    failures: &mut Vec<PathBuf>,
    directory: &PathBuf,
    factory: &NodeFactory,
    known_libs: &HashMap<String, PathBuf>,
    replace: bool,
) -> Result<()> {
    if !directory.exists() {
        bail!(
            "fatal: tried finding nodes recursively for directory={}, but it does not exist",
            directory.display()
        );
    }
    let paths = get_paths_recursive_from_dir(directory)?;
    let mut i = 0;
    let total = paths.len();

    for p in paths {
        let res = factory
            .make(&p, known_libs)
            .and_then(|n| add_to_graph_if_some(g, n, known_libs, replace));
        if let Err(_) = res {
            failures.push(p);
        }
        i += 1;
        if i % (total / 10) == 0 {
            info!("graph: pass 1: {}/{} nodes", i, total);
        }
    }
    Ok(())
}

fn add_to_graph_if_some(
    g: &mut FileGraph<NodeFactory>,
    maybe_node: Option<Node>,
    known_libs: &HashMap<String, PathBuf>,
    replace: bool,
) -> Result<()> {
    match maybe_node {
        Some(node) => {
            g.add_tree(node, known_libs, replace)?;
            Ok(())
        }
        None => Ok(()),
    }
}

fn get_libs_from_graph(g: &FileGraph<NodeFactory>) -> HashMap<String, PathBuf> {
    let mut known_libs = HashMap::new();
    for n in g.iter_nodes() {
        n.path
            .file_name()
            .and_then(|file_name| file_name.to_str())
            .map(|f| f.to_string())
            .map(|f| known_libs.insert(f, n.path.clone()));
    }
    known_libs
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
