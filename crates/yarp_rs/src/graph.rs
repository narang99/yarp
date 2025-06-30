use std::{collections::HashMap, fmt::Display, rc::Rc};

use anyhow::{Context, Result, anyhow};
use petgraph::{Graph, algo::toposort, graph::NodeIndex};

use crate::node::{DistNode, Node};

#[derive(Debug)]
pub struct FileGraph {
    inner: Graph<(), ()>,
    idx_by_dist_node: HashMap<NodeIndex, DistNode>,
    node_by_idx: HashMap<Rc<Node>, NodeIndex>,
}

impl Display for FileGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for node_idx in self.inner.node_indices() {
            let node = self
                .idx_by_dist_node
                .get(&node_idx)
                .expect("corrupted graph state");
            write!(f, "{} -> ", node)?;

            let mut neighbors = Vec::new();
            for neighbor_idx in self.inner.neighbors(node_idx) {
                let neighbor = self
                    .idx_by_dist_node
                    .get(&neighbor_idx)
                    .expect("corrupted graph state");
                neighbors.push(format!("{}", neighbor));
            }
            writeln!(f, "[{}]", neighbors.join(", "))?;
        }
        Ok(())
    }
}

impl FileGraph {
    pub fn new() -> Self {
        Self {
            inner: Graph::new(),
            idx_by_dist_node: HashMap::new(),
            node_by_idx: HashMap::new(),
        }
    }

    fn add_edge(&mut self, src: &Node, dest: &Node) {
        let s = self.node_by_idx.get(src).expect(&format!(
            "fatal error: expected the node {:?} to be in graph",
            src
        ));
        let d = self.node_by_idx.get(dest).expect(&format!(
            "fatal error: expected the node {:?} to be in graph",
            dest
        ));

        self.inner.add_edge(*s, *d, ());
    }

    pub fn add_node(&mut self, f: DistNode) -> Result<()> {
        if self.node_by_idx.contains_key(&f.node) {
            return Ok(());
        }
        // we get the deps first because we move the node inside the graph later
        let our_path = f.dist_file.file_path().clone();
        let deps = f.dist_file.deps()?;

        let idx = self.inner.add_node(());
        let our_node = Rc::clone(&f.node);
        self.node_by_idx.insert(Rc::clone(&f.node), idx);
        self.idx_by_dist_node.insert(idx, f);

        for d in deps {
            let parent_node = Rc::clone(&d.node);
            self.add_node(d)
                .context(anyhow!("file: {}", our_path.to_string_lossy()))?;
            self.add_edge(&parent_node, &our_node);
        }

        Ok(())
    }

    pub fn toposort(&self) -> Result<impl Iterator<Item = DistNode>> {
        let node_indices = toposort(&self.inner, None)
            .map_err(|e| anyhow!("{:#?}", e))
            .context("dependency analysis failed")?;

        Ok(node_indices.into_iter().map(|idx| {
            // clones are safe due to Rc
            self.idx_by_dist_node
                .get(&idx)
                .expect(&format!("fatal: failure in topological sort, found an index which we could not find in our registry: index={:?}", idx))
                .clone()
        }))
    }
}

#[cfg(test)]
mod test {
    use crate::node::{DistFile, Kind};

    use super::*;
    use anyhow::Result;
    use std::{path::PathBuf, str::FromStr};

    #[derive(Debug)]
    struct MockDistFile {
        deps: Vec<DistNode>,
        mock_path: PathBuf,
    }

    impl MockDistFile {
        fn new(deps: Vec<DistNode>) -> Self {
            Self {
                deps,
                mock_path: PathBuf::from("hello"),
            }
        }
    }

    impl DistFile for MockDistFile {
        fn deps(&self) -> Result<Vec<DistNode>> {
            Ok(self.deps.clone())
        }

        fn file_path(&self) -> &PathBuf {
            &self.mock_path
        }
        // fn to_reals(&self, _unit_root: PathBuf) {
        //     // Mock implementation - no-op
        // }

        // fn to_destination(&self, _dist_root: PathBuf) {
        //     // Mock implementation - no-op
        // }
    }

    #[test]
    fn test_add_node_single() {
        let mut graph = FileGraph::new();

        let node = Rc::new(Node { kind: Kind::Python });

        let dist_node = DistNode {
            node: Rc::clone(&node),
            dist_file: Rc::new(MockDistFile::new(vec![])),
        };

        let result = graph.add_node(dist_node);
        assert!(result.is_ok());
        assert_eq!(graph.inner.node_count(), 1);
        assert!(graph.node_by_idx.contains_key(&node));
    }

    #[test]
    fn test_add_node_with_dependencies() {
        let mut graph = FileGraph::new();

        let dep_node = Rc::new(Node {
            kind: Kind::SharedLibrary {
                lib_path: PathBuf::from_str("/libtest").unwrap(),
            },
        });

        let dep_dist_node = DistNode {
            node: Rc::clone(&dep_node),
            dist_file: Rc::new(MockDistFile::new(vec![])),
        };

        let main_node = Rc::new(Node { kind: Kind::Python });

        let main_dist_node = DistNode {
            node: Rc::clone(&main_node),
            dist_file: Rc::new(MockDistFile::new(vec![dep_dist_node])),
        };

        let result = graph.add_node(main_dist_node);
        assert!(result.is_ok());
        assert_eq!(graph.inner.node_count(), 2);
        assert_eq!(graph.inner.edge_count(), 1);
        assert!(graph.node_by_idx.contains_key(&main_node));
        assert!(graph.node_by_idx.contains_key(&dep_node));
    }

    #[test]
    fn test_add_duplicate_node() {
        let mut graph = FileGraph::new();

        let node = Rc::new(Node { kind: Kind::Python });

        let dist_node1 = DistNode {
            node: Rc::clone(&node),
            dist_file: Rc::new(MockDistFile::new(vec![])),
        };

        let dist_node2 = DistNode {
            node: Rc::clone(&node),
            dist_file: Rc::new(MockDistFile::new(vec![])),
        };

        let result1 = graph.add_node(dist_node1);
        assert!(result1.is_ok());
        assert_eq!(graph.inner.node_count(), 1);

        let result2 = graph.add_node(dist_node2);
        assert!(result2.is_ok());
        assert_eq!(graph.inner.node_count(), 1); // Should not add duplicate
    }

    #[test]
    fn test_add_node_complex_dependencies() {
        let mut graph = FileGraph::new();

        // Create a chain: main -> dep1 -> dep2
        let dep2_node = Rc::new(Node {
            kind: Kind::PyFile {
                src_path: PathBuf::from_str("/path/to/dep2.py").unwrap(),
            },
        });

        let dep2_dist_node = DistNode {
            node: Rc::clone(&dep2_node),
            dist_file: Rc::new(MockDistFile::new(vec![])),
        };

        let dep1_node = Rc::new(Node {
            kind: Kind::SharedLibrary {
                lib_path: PathBuf::from_str("libdep1").unwrap(),
            },
        });

        let dep1_dist_node = DistNode {
            node: Rc::clone(&dep1_node),
            dist_file: Rc::new(MockDistFile::new(vec![dep2_dist_node.clone()])),
        };

        let dep3_node = Rc::new(Node {
            kind: Kind::SharedLibrary {
                lib_path: PathBuf::from_str("libdep3").unwrap(),
            },
        });

        let dep3_dist_node = DistNode {
            node: Rc::clone(&dep3_node),
            dist_file: Rc::new(MockDistFile::new(vec![dep2_dist_node])),
        };

        let main_node = Rc::new(Node { kind: Kind::Python });

        let main_dist_node = DistNode {
            node: Rc::clone(&main_node),
            dist_file: Rc::new(MockDistFile::new(vec![dep1_dist_node, dep3_dist_node])),
        };

        let result = graph.add_node(main_dist_node);
        assert!(result.is_ok());
        assert_eq!(graph.inner.node_count(), 4);
        assert_eq!(graph.inner.edge_count(), 4);
        assert!(graph.node_by_idx.contains_key(&main_node));
        assert!(graph.node_by_idx.contains_key(&dep1_node));
        assert!(graph.node_by_idx.contains_key(&dep2_node));

        // Test topological sort
        let nodes: Vec<DistNode> = graph.toposort().unwrap().collect();
        assert_eq!(nodes.len(), 4);

        assert_before(&nodes, &dep2_node, &dep1_node);
        assert_before(&nodes, &dep2_node, &dep3_node);
        assert_before(&nodes, &dep1_node, &main_node);
        assert_before(&nodes, &dep3_node, &main_node);
    }

    fn assert_before(vec: &Vec<DistNode>, first: &Node, second: &Node) {
        let first_pos = vec
            .iter()
            .position(|x| *x.node == *first)
            .expect("first element not found in vec");
        let second_pos = vec
            .iter()
            .position(|x| *x.node == *second)
            .expect("second element not found in vec");

        assert!(
            first_pos < second_pos,
            "Expected first element to come before second element in vec, first={:?} second={:?}",
            first,
            second
        );
    }
}
