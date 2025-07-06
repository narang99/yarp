mod core;
mod elf;
mod macho;
mod search;

use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
pub use core::{BinaryParseError, Elf, Macho, Binary};
use log::warn;
use std::collections::HashMap;
use std::path::PathBuf;

use elf::parse as parse_linux;
use macho::parse as parse_macho;
// pub use macho::get_deps_from_macho;



pub fn parse_and_search(
    path: &PathBuf,
    executable_path: &PathBuf,
    cwd: &PathBuf,
    env: &HashMap<String, String>,
    known_libs: &HashMap<String, PathBuf>,
    extra_rpaths: &Vec<PathBuf>,
) -> Result<(Binary, Vec<PathBuf>)> {
    let mut file =
        std::fs::File::open(path).context(anyhow!("Can't open the file={}", path.display()))?;
    let (binary, rpaths) = match lief::Binary::from(&mut file) {
        Some(lief::Binary::ELF(elf)) => {
            let (elf, rpaths) = parse_linux(elf, path, cwd, env, extra_rpaths)?;
            (Binary::Elf(elf), rpaths)
        }
        Some(lief::Binary::MachO(macho)) => {
            let macho = parse_macho(macho, path, executable_path, cwd, env, known_libs)?;
            (Binary::Macho(macho), Vec::new())
        }
        Some(lief::Binary::PE(_)) => {
            warn!(
                "windows PE object files are not supported: {}",
                path.display()
            );
            return Err(Error::new(BinaryParseError::UnsupportedArchitecture));
        }
        None => {
            return Err(Error::new(BinaryParseError::NotBinary));
        }
    };

    Ok((binary, rpaths))
}
