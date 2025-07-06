
use crate::parse::Macho;

#[derive(Debug, Clone)]
pub enum Binary {
    Macho(Macho),
}

#[derive(Debug, Clone)]
pub enum Deps {
    Plain,
    Binary(Binary),

    #[cfg(test)]
    Mock {
        paths: Vec<PathBuf>,
    },
}
