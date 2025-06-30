use std::{
    fmt::Display,
    hash::{Hash, Hasher},
    path::PathBuf,
};

use crate::node::deps::Deps;

pub mod deps;

#[derive(Debug, Clone)]
pub struct Node {
    pub path: PathBuf,

    pub deps: Deps,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Eq for Node {}

impl Hash for Node {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.path.hash(state);
    }
}

impl Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Node{{")?;
        write!(f, "{}", self.path.display())?;
        write!(f, "}}")
    }
}

impl Node {
    pub fn new(path: PathBuf) -> Node {
        let deps = Deps::from_path(&path);
        Node { path, deps }
    }

    #[cfg(test)]
    pub fn mock(path: PathBuf, deps: Vec<PathBuf>) -> Node {
        Node {path, deps: Deps::mock(deps)}
    }
}
