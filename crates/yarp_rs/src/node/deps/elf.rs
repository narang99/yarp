use std::{collections::HashMap, path::PathBuf};

use anyhow::{anyhow, Context, Result};

use crate::node::deps::core::Elf;

/// parse a macho file and get its dependencies
/// Parsing logic depends on three kinds of paths
/// First is an actual path, denoted by Path/PathBuf
/// Second is a string path that needs resolution
pub fn parse(
    elf_path: &PathBuf,
    executable_path: &PathBuf,
    cwd: &PathBuf,
    known_libs: &HashMap<String, PathBuf>,
    extra_search_paths: &Vec<PathBuf>,
) -> Result<Elf> {
    _parse(
        elf_path,
        executable_path,
        cwd,
        known_libs,
        extra_search_paths,
    )
    .with_context(|| anyhow!("failed in parsing macho={} context={:?}", elf_path.display(), executable_path.display()))
}

fn _parse(
    elf_path: &PathBuf,
    executable_path: &PathBuf,
    cwd: &PathBuf,
    known_libs: &HashMap<String, PathBuf>,
    extra_search_paths: &Vec<PathBuf>,
) -> Result<Elf> {
    Err(anyhow!("damn"))
}
