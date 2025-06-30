// given a macho to parse
// we return the rpaths if they exist
// loader-path would be simply the current path
// we also want executable-path as an input

use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use lief::macho::{
    Commands,
    commands::{Command, LoadCommandTypes},
};
use log::info;

struct PathResolverCtx<'a> {
    rpaths: Vec<PathBuf>,
    loader_path: &'a PathBuf,
    shared_lib_ctx: &'a SharedLibCtx<'a>,
}

#[derive(Debug)]
struct SharedLibCtx<'a> {
    executable_path: &'a PathBuf,
    cwd: &'a PathBuf,
}

struct Macho {
    // all load commands, along with the resolved path of the dependency
    load_cmds: HashMap<String, PathBuf>,

    // all rpaths, along with resolved rpath
    rpaths: HashMap<String, PathBuf>,

    // the current id of the dylib
    id_dylib: Option<String>,

    // path to the lib
    path: PathBuf,
}

pub fn get_deps(
    macho_path: &PathBuf,
    executable_path: &PathBuf,
    cwd: &PathBuf,
) -> Result<Vec<PathBuf>> {
    let string_path = macho_path.to_str().ok_or(anyhow!(
        "path {} could not be converted to string, `yarp` does not support these paths",
        macho_path.display()
    ))?;
    let ctx = SharedLibCtx {
        executable_path,
        cwd,
    };
    let macho = parse(&string_path, &ctx)?;
    Ok(macho.load_cmds.into_iter().map(|(_, path)| path).collect())
}

/// parse a macho file and get its dependencies
/// Parsing logic depends on three kinds of paths
/// First is an actual path, denoted by Path/PathBuf
/// Second is a string path that needs resolution
fn parse(macho_path: &str, ctx: &SharedLibCtx) -> Result<Macho> {
    _parse(macho_path, ctx)
        .with_context(|| anyhow!("failed in parsing macho={} context={:?}", macho_path, ctx))
}

fn _parse(macho_path: &str, ctx: &SharedLibCtx) -> Result<Macho> {
    let buf = PathBuf::from(macho_path);
    if !buf.exists() {
        bail!(
            "failed in resolving dependency, it does not exist, path={}",
            macho_path
        );
    }
    let fat = lief::macho::parse(macho_path).unwrap();
    if fat.iter().count() > 1 {
        panic!(
            "found a FAT Macho binary, `yarp` currently does not support this binary type, path={}",
            macho_path
        )
    }
    let macho = fat.iter().next().expect(&format!(
        "expected fat binary to have one macho file, got 0, path={}",
        macho_path
    ));
    let loader_path = buf
        .parent()
        .ok_or(anyhow!(
            "fatal: could not find directory of path={}",
            buf.display()
        ))?
        .to_path_buf();

    let rpaths = get_rpaths(&macho, ctx.executable_path, ctx.cwd, &loader_path)
        .context(anyhow!("failed in parsing rpath"))?;
    let resolver_ctx = PathResolverCtx {
        loader_path: &loader_path,
        rpaths: rpaths.iter().map(|(_, rpath)| rpath.clone()).collect(),
        shared_lib_ctx: ctx,
    };
    let (id_dylib, load_cmds) = get_load_commands(&macho, &macho_path, &resolver_ctx)
        .context(anyhow!("failed in parsing load commands"))?;
    Ok(Macho {
        load_cmds,
        rpaths,
        id_dylib,
        path: buf,
    })
}

fn get_rpaths(
    macho: &lief::macho::Binary,
    executable_path: &PathBuf,
    cwd: &PathBuf,
    loader_path: &PathBuf,
) -> Result<HashMap<String, PathBuf>> {
    let mut rpaths = HashMap::new();
    for cmd in macho.commands() {
        match cmd {
            Commands::RPath(rpath) => {
                let val = rpath.path();
                let p = resolve_rpath(&val, executable_path, cwd, loader_path)
                    .context(anyhow!("failed in resolving rpath={}", val))?;
                if let Some(inner) = p {
                    rpaths.insert(val, inner);
                }
            }
            _ => {}
        };
    }
    Ok(rpaths)
}

fn get_load_commands(
    macho: &lief::macho::Binary,
    macho_path: &str,
    ctx: &PathResolverCtx,
) -> Result<(Option<String>, HashMap<String, PathBuf>)> {
    let mut id_dylib = None;
    let mut load_cmds = HashMap::new();
    for cmd in macho.commands() {
        match cmd {
            Commands::Dylib(dylib) => match dylib.command_type() {
                LoadCommandTypes::IdDylib => id_dylib = Some(dylib.name()),
                LoadCommandTypes::LoadDylib => {
                    let val = dylib.name();
                    if is_load_cmd_path_sys_lib(&val) {
                        info!(
                            "skipping system library {} in macho parsing, dependency of {}",
                            val, macho_path
                        );
                        continue;
                    }
                    let p = resolve_load_cmd_path(&val, ctx)
                        .with_context(|| format!("failed in resolving load command={}", val))?;
                    match p {
                        Some(p) => {
                            load_cmds.insert(val, p);
                        }
                        None => {
                            bail!("could not find dependency for load_cmd={}", val);
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        };
    }
    Ok((id_dylib, load_cmds))
}

fn is_load_cmd_path_sys_lib(load_cmd_path: &str) -> bool {
    load_cmd_path.starts_with("/usr/lib/")
        || load_cmd_path.starts_with("/System/Library/Frameworks/")
}

fn resolve_rpath(
    load_cmd_rpath: &str,
    executable_path: &PathBuf,
    cwd: &PathBuf,
    loader_path: &PathBuf,
) -> Result<Option<PathBuf>> {
    if load_cmd_rpath.starts_with("@rpath/") {
        bail!(
            "could not resolve RPath={}, it is invalid for RPaths to start with `@rpath/`, most likely the shared library is corrupted. Raise an issue if you think this is a bug",
            load_cmd_rpath,
        );
    }
    let ctx = &PathResolverCtx {
        rpaths: Vec::new(),
        loader_path: loader_path,
        shared_lib_ctx: &SharedLibCtx {
            executable_path: executable_path,
            cwd: cwd,
        },
    };
    resolve_load_cmd_path(load_cmd_rpath, ctx)
}

/// given a load command, this function would try to resolve it
/// if it does not find any path associated with the load command, we send an `Ok(None)`
/// on every other kind of error, we send Err
fn resolve_load_cmd_path(load_cmd_path: &str, ctx: &PathResolverCtx) -> Result<Option<PathBuf>> {
    if load_cmd_path.starts_with("@rpath/") {
        let p = load_cmd_path.strip_prefix("@rpath/").expect(&format!(
            "fatal: load_cmd_path={} should start with @rpath/",
            load_cmd_path
        ));
        for rpath in &ctx.rpaths {
            let resolved = rpath.join(p);
            if resolved.exists() {
                return Ok(Some(resolved));
            }
        }
        return Ok(None);
    } else if load_cmd_path.starts_with("@loader_path/") {
        let p = load_cmd_path.strip_prefix("@loader_path/").expect(&format!(
            "fatal: load_cmd_path={} should start with @loader_path/",
            load_cmd_path
        ));
        let resolved = ctx.loader_path.join(p);
        if resolved.exists() {
            return Ok(Some(resolved));
        } else {
            return Ok(None);
        }
    } else if load_cmd_path.starts_with("@executable_path/") {
        let p = load_cmd_path
            .strip_prefix("@executable_path/")
            .expect(&format!(
                "fatal: load_cmd_path={} should start with @executable_path/",
                load_cmd_path
            ));
        let resolved = ctx.shared_lib_ctx.executable_path.join(p);
        if resolved.exists() {
            return Ok(Some(resolved));
        } else {
            return Ok(None);
        }
    } else if load_cmd_path.starts_with("/") {
        let resolved = PathBuf::from(load_cmd_path);
        if resolved.exists() {
            return Ok(Some(resolved));
        } else {
            return Ok(None);
        }
    } else {
        let resolved = ctx.shared_lib_ctx.cwd.join(load_cmd_path);
        if resolved.exists() {
            return Ok(Some(resolved));
        } else {
            return Ok(None);
        }
    }
}
