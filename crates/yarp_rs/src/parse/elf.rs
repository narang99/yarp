use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;
use lief::elf::Binary;


use crate::node::deps::core::Elf;

pub fn parse_and_search(binary: Binary, cwd: &PathBuf, known_libs: &HashMap<String, PathBuf>) {

}

// fn parse(
//     binary: Binary,
//     executable_path: &PathBuf,
//     cwd: &PathBuf,
//     ld_library_path: &Vec<PathBuf>,
//     known_libs: &HashMap<String, PathBuf>,
// ) -> Result<Elf> {
//     let ctx = SharedLibCtx {
//         executable_path,
//         cwd,
//         dyld_library_path,
//     };
//     _parse(macho_path, &ctx, known_libs)
//         .with_context(|| anyhow!("failed in parsing macho={} context={:?}", macho_path, ctx))
// }