use std::{fmt::Display, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use bimap::BiHashMap;
use petgraph::{Graph, algo::toposort, graph::NodeIndex};

use crate::node::Node;

#[derive(Debug)]
pub struct FileGraph {
    executable_path: PathBuf,
    cwd: PathBuf,
    inner: Graph<(), ()>,
    idx_by_node: BiHashMap<NodeIndex, Node>,
}

impl Display for FileGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for node_idx in self.inner.node_indices() {
            let node = self
                .idx_by_node
                .get_by_left(&node_idx)
                .expect("corrupted graph state");
            write!(f, "{} -> ", node)?;

            let mut neighbors = Vec::new();
            for neighbor_idx in self.inner.neighbors(node_idx) {
                let neighbor = self
                    .idx_by_node
                    .get_by_left(&neighbor_idx)
                    .expect("corrupted graph state");
                neighbors.push(format!("{}", neighbor));
            }
            writeln!(f, "[{}]", neighbors.join(", "))?;
        }
        Ok(())
    }
}

impl FileGraph {
    pub fn new(executable_path: PathBuf, cwd: PathBuf) -> Self {
        Self {
            inner: Graph::new(),
            idx_by_node: BiHashMap::new(),
            executable_path,
            cwd,
        }
    }

    /// simply add a node to the graph, this is a plain operation
    /// No dependency analysis or edge making is done
    /// use `add_tree` for that
    pub fn add_node(&mut self, node: Node) -> NodeIndex {
        let idx = self.idx_by_node.get_by_right(&node);
        match idx {
            Some(idx) => *idx,
            None => {
                let idx = self.inner.add_node(());
                self.idx_by_node.insert(idx, node);
                idx
            }
        }
    }

    /// Take a node, and recursively add its dependencies to the graph
    /// An edge is created from the dependency to the node
    /// If the node is already present in the graph, this node is not inserted
    /// And the currently present node is used for all functions
    /// Else we insert the node
    pub fn add_tree(&mut self, node: Node) -> Result<NodeIndex> {
        let idx = match self.idx_by_node.get_by_right(&node) {
            Some(idx) => *idx,
            None => {
                let idx = self.inner.add_node(());
                self.idx_by_node.insert(idx, node.clone());
                idx
            }
        };
        let node = self.idx_by_node.get_by_left(&idx).expect(&format!(
            "fatal: expected node to be present for idx={:?}",
            idx
        ));

        let deps = node
            .deps
            .find(&node.path, &self.executable_path, &self.cwd)?;
        println!("adding deps for {:?}, deps={:?}", node, deps);

        for p in deps {
            let parent_node = Node::new(p.clone());
            let parent_idx = self
                .add_tree(parent_node)
                .context(anyhow!("file: {}", p.display()))?;
            self.inner.add_edge(parent_idx, idx, ());
        }

        Ok(idx)
    }

    pub fn toposort(&self) -> Result<impl Iterator<Item = Node>> {
        let node_indices = toposort(&self.inner, None)
            .map_err(|e| anyhow!("{:#?}", e))
            .context("dependency analysis failed")?;

        Ok(node_indices.into_iter().map(|idx| {
            // clones are safe due to Rc
            self.idx_by_node
                .get_by_left(&idx)
                .expect(&format!("fatal: failure in topological sort, found an index which we could not find in our registry: index={:?}", idx))
                .clone()
        }))
    }
}

#[cfg(test)]
mod test {

    use super::*;
    
    use std::{path::PathBuf, str::FromStr};

    fn get_graph() -> FileGraph {
        let executable_path = PathBuf::from_str("/python").unwrap();
        let cwd = PathBuf::from_str(".").unwrap();
        FileGraph::new(executable_path, cwd)
    }

    #[test]
    fn test_add_node_single() {
        let mut graph = get_graph();
        let node = Node::mock(PathBuf::from_str("/python").unwrap(), vec![]);
        let idx = graph.add_tree(node).unwrap();
        assert_eq!(graph.inner.node_count(), 1);
        assert!(graph.idx_by_node.contains_left(&idx));
    }

    #[test]
    fn test_add_node_with_dependencies() {
        let mut graph = get_graph();

        let lib_test = Node::mock(PathBuf::from_str("/libtest").unwrap(), vec![]);
        let py_node = Node::mock(
            PathBuf::from_str("/python").unwrap(),
            vec![PathBuf::from_str("/libtest").unwrap()],
        );

        let idx = graph.add_tree(py_node.clone()).unwrap();
        assert_eq!(graph.inner.node_count(), 2);
        assert_eq!(graph.inner.edge_count(), 1);
        assert!(graph.idx_by_node.contains_right(&lib_test));
        assert!(graph.idx_by_node.contains_right(&py_node));
    }

    #[test]
    fn test_add_duplicate_node() {
        let mut graph = get_graph();

        let node = Node::mock(PathBuf::from_str("/python").unwrap(), vec![]);

        graph.add_tree(node.clone()).unwrap();
        assert_eq!(graph.inner.node_count(), 1);

        graph.add_tree(node.clone()).unwrap();
        assert_eq!(graph.inner.node_count(), 1); // Should not add duplicate
    }

    #[test]
    fn test_add_node_complex_dependencies() {
        println!("*************start complex test**********************");
        let mut graph = get_graph();

        let dep2_path = PathBuf::from_str("/path/to/dep2.py").unwrap();
        let dep2 = Node::mock(dep2_path.clone(), vec![]);

        let dep1_path = PathBuf::from_str("libdep1").unwrap();
        let dep1 = Node::mock(dep1_path.clone(), vec![dep2_path.clone()]);

        let dep3_path = PathBuf::from_str("libdep3").unwrap();
        let dep3 = Node::mock(dep3_path.clone(), vec![dep2_path.clone()]);

        let main_path = PathBuf::from_str("/python").unwrap();
        let main = Node::mock(main_path, vec![dep1_path, dep3_path]);

        graph.add_node(main.clone());
        graph.add_node(dep1.clone());
        graph.add_node(dep2.clone());
        graph.add_node(dep3.clone());

        let result = graph.add_tree(main.clone());
        println!("*************end complex adding**********************");
        assert!(result.is_ok());
        assert_eq!(graph.inner.node_count(), 4);
        assert_eq!(graph.inner.edge_count(), 4);
        assert!(graph.idx_by_node.contains_right(&main));
        assert!(graph.idx_by_node.contains_right(&dep1));
        assert!(graph.idx_by_node.contains_right(&dep2));

        // Test topological sort
        let nodes: Vec<Node> = graph.toposort().unwrap().collect();
        assert_eq!(nodes.len(), 4);

        assert_before(&nodes, &dep2, &dep1);
        assert_before(&nodes, &dep2, &dep3);
        assert_before(&nodes, &dep1, &main);
        assert_before(&nodes, &dep3, &main);
    }

    fn assert_before(vec: &Vec<Node>, first: &Node, second: &Node) {
        let first_pos = vec
            .iter()
            .position(|x| *x == *first)
            .expect("first element not found in vec");
        let second_pos = vec
            .iter()
            .position(|x| *x == *second)
            .expect("second element not found in vec");

        assert!(
            first_pos < second_pos,
            "Expected first element to come before second element in vec, first={:?} second={:?}",
            first,
            second
        );
    }
}
