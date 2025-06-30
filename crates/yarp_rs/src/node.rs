use anyhow::Result;
use std::{fmt::Display, path::PathBuf, rc::Rc};

/// `Kind` is used to differentiate between values which require separate handling
/// `Python` kind denotes a python executable, only one of its kind can be present
/// `SharedLibrary` kind denotes a shared library, equality checking is done using only name
/// `PyFile` is unique for every `src_path`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Kind {
    Python,
    SharedLibrary { lib_path: PathBuf },
    PyFile { src_path: PathBuf },
}

/// every file that is tracked by yarp has to implement this trait
/// it denotes a single file whose dependencies are moved into dist
/// along with copying/patching the file itself to the dist folder
pub trait DistFile: std::fmt::Debug {
    /// return all the dependencies for your file
    /// These would be used in creating the graph of files to be moved to dist
    /// dependencies are put into symlink farm basically
    fn deps(&self) -> Result<Vec<DistNode>>;

    /// return the path of the file
    fn file_path(&self) -> &PathBuf;

    // move this file to reals directory (can be a noop)
    // you will get the root directory to the logical unit that you are part of
    // the final file content needs to go (if it goes anywhere)
    // `to_reals` would already have been called on your dependencies
    // make sure you make any symlinks or whatever
    // the file should be functioning after it has been moved
    // fn to_reals(&self, unit_root: PathBuf);

    // many files can have fixed destinations in the dist
    // `python` executable goes to `./python/bin/python`
    // `lib-dynload` similarly has the same behavior
    // site-packages so's will symlink to reals at some location, etc.
    // fn to_destination(&self, dist_root: PathBuf);
}

/// a single node in the graph
/// this needs to be hashable as we use this as a core unit of our graph operations
/// Each node is a unit which has dependencies, and is placed in the final dist
/// differentiating nodes is done by their `kind`
/// Nodes can contain other information in their `inner` dyn box, this information is simply
/// Whatever we need implement the box's trait
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Node {
    pub kind: Kind,
}

impl Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Node{{")?;
        match &self.kind {
            Kind::Python => write!(f, "Python"),
            Kind::SharedLibrary { lib_path: name } => {
                write!(f, "SharedLibrary({})", name.display())
            }
            Kind::PyFile { src_path } => write!(f, "PyFile({})", src_path.display()),
        }?;
        write!(f, "}}")
    }
}

/// not used for indexing, but kept in graph
/// `dist_file` is used for polymorphism over `DistFile`
#[derive(Debug, Clone)]
pub struct DistNode {
    pub node: Rc<Node>,
    pub dist_file: Rc<dyn DistFile>,
}

impl Display for DistNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.node)
    }
}
