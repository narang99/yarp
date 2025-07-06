mod elf;
mod macho;
mod core;


pub use core::{BinaryParseError, Macho, Elf};
// TODO: remove this
pub use macho::get_deps_from_macho;
pub use macho::parse as parse_macho;



pub fn parse_and_search() {

}