use std::{collections::HashMap, fmt::Display, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use bimap::BiHashMap;
use petgraph::{Direction::Incoming, Graph, algo::toposort, graph::NodeIndex, visit::EdgeRef};

use crate::{
    manifest::Env,
    node::{Node, Pkg, deps::Deps},
    paths::get_dyld_library_path,
};

#[derive(Debug)]
pub struct FileGraph {
    executable_path: PathBuf,
    cwd: PathBuf,
    inner: Graph<(), ()>,
    idx_by_path: BiHashMap<NodeIndex, PathBuf>,
    path_by_node: HashMap<PathBuf, Node>,
    env: Env,
}

impl Display for FileGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for node_idx in self.inner.node_indices() {
            let path = self
                .idx_by_path
                .get_by_left(&node_idx)
                .expect("corrupted graph state");
            write!(f, "{} -> ", path.display())?;

            let mut neighbors = Vec::new();
            for neighbor_idx in self.inner.neighbors(node_idx) {
                let neighbor = self
                    .idx_by_path
                    .get_by_left(&neighbor_idx)
                    .expect("corrupted graph state");
                neighbors.push(format!("{}", neighbor.display()));
            }
            writeln!(f, "[{}]", neighbors.join(", "))?;
        }
        Ok(())
    }
}

impl FileGraph {
    pub fn new(executable_path: PathBuf, cwd: PathBuf, env: Env) -> Self {
        Self {
            inner: Graph::new(),
            idx_by_path: BiHashMap::new(),
            path_by_node: HashMap::new(),
            executable_path,
            cwd,
            env,
        }
    }

    pub fn len(&self) -> usize {
        self.inner.node_count()
    }

    /// simply add a node to the graph, this is a plain operation
    /// No dependency analysis or edge making is done
    /// use `add_tree` for that
    pub fn add_node(&mut self, node: Node) -> NodeIndex {
        let idx = self.idx_by_path.get_by_right(&node.path);
        match idx {
            Some(idx) => *idx,
            None => {
                let idx = self.inner.add_node(());
                self.raw_add_node(idx, node);
                idx
            }
        }
    }

    fn raw_add_node(&mut self, idx: NodeIndex, node: Node) {
        self.idx_by_path.insert(idx, node.path.clone());
        self.path_by_node.insert(node.path.clone(), node);
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = &Node> {
        self.path_by_node.values()
    }

    /// Take a node, and recursively add its dependencies to the graph
    /// An edge is created from the dependency to the node
    /// If the node is already present in the graph, this node is not inserted
    /// And the currently present node is used for all functions
    /// Else we insert the node
    pub fn add_tree(
        &mut self,
        node: Node,
        known_libs: &HashMap<String, PathBuf>,
    ) -> Result<NodeIndex> {
        let path = node.path.clone();
        let idx = self.add_node(node);
        let node = self
            .path_by_node
            .get(&path)
            .with_context(|| {
                anyhow!(
                    "fatal: expected node to be present for path={}",
                    path.display()
                )
            })
            .unwrap();

        let deps = node.deps.find()?;

        for p in deps {
            let parent_idx = match self.idx_by_path.get_by_right(&p) {
                Some(idx) => {
                    *idx
                },
                None => {
                    // no parent found, recursive create it and add its tree
                    let pkg = Pkg::from_path(&p);
                    let deps = Deps::from_path(
                        &p,
                        &self.executable_path,
                        &self.cwd,
                        &get_dyld_library_path(&self.env),
                        &known_libs,
                    )?;
                    let parent_node = Node::new(p.clone(), pkg, deps)?;
                    let parent_idx = self
                        .add_tree(parent_node, known_libs)
                        .context(anyhow!("file: {}", p.display()))?;
                    parent_idx
                },
            };
            self.inner.add_edge(parent_idx, idx, ());
        }

        Ok(idx)
    }

    pub fn toposort(&self) -> Result<impl Iterator<Item = Node>> {
        let node_indices = toposort(&self.inner, None)
            .map_err(|e| anyhow!("{:#?}", e))
            .context("dependency analysis failed")?;

        Ok(node_indices.into_iter().map(|idx| {
            self.idx_by_path
                .get_by_left(&idx)
                .expect(&format!("fatal: failure in topological sort, found an index which we could not find in our registry: index={:?}", idx))
                .clone()
        }).map(|path| {
            self.path_by_node
                .get(&path)
                .expect(&format!("fatal: failure in topological sort, found a path which we could not find in our registry: path={:?}", path))
                .clone()
        }))
    }

    pub fn get_node_dependencies(&self, node: &Node) -> Vec<Node> {
        // given a node, return all the dependencies of the node
        self.idx_by_path
            .get_by_right(&node.path)
            .map(|idx| {
                self.inner
                    .edges_directed(*idx, Incoming)
                    .map(|e| self.get_node_by_index_or_panic(e.source()))
                    .collect::<Vec<Node>>()
            })
            .unwrap_or(vec![])
    }

    fn get_node_by_index_or_panic(&self, idx: NodeIndex) -> Node {
        let path = self.idx_by_path.get_by_left(&idx).expect(&format!(
            "corrupted graph state: could not find path for idx in edge, idx={:?}",
            idx
        ));
        self.path_by_node
            .get(path)
            .expect(&format!(
                "corrupted graph state: could not find node for path in edge, path={:}",
                path.display()
            ))
            .clone()
    }
}

#[cfg(test)]
mod test {

    use super::*;

    use std::{path::PathBuf, str::FromStr};

    fn get_graph() -> FileGraph {
        let executable_path = PathBuf::from_str("/python").unwrap();
        let cwd = PathBuf::from_str(".").unwrap();
        FileGraph::new(executable_path, cwd, HashMap::new())
    }

    #[test]
    fn test_add_node_single() {
        let mut graph = get_graph();
        let node = Node::mock(PathBuf::from_str("/python").unwrap(), vec![]).unwrap();
        let idx = graph.add_tree(node, &HashMap::new()).unwrap();
        assert_eq!(graph.inner.node_count(), 1);
        assert!(graph.idx_by_path.contains_left(&idx));
    }

    #[test]
    fn test_add_node_with_dependencies() {
        let mut graph = get_graph();

        let lib_test = Node::mock(PathBuf::from_str("/libtest").unwrap(), vec![]).unwrap();
        let py_node = Node::mock(
            PathBuf::from_str("/python").unwrap(),
            vec![PathBuf::from_str("/libtest").unwrap()],
        )
        .unwrap();

        graph.add_tree(py_node.clone(), &HashMap::new()).unwrap();
        assert_eq!(graph.inner.node_count(), 2);
        assert_eq!(graph.inner.edge_count(), 1);
        assert!(graph.idx_by_path.contains_right(&lib_test.path));
        assert!(graph.idx_by_path.contains_right(&py_node.path));
    }

    #[test]
    fn test_add_duplicate_node() {
        let mut graph = get_graph();

        let node = Node::mock(PathBuf::from_str("/python").unwrap(), vec![]).unwrap();

        graph.add_tree(node.clone(), &HashMap::new()).unwrap();
        assert_eq!(graph.inner.node_count(), 1);

        graph.add_tree(node.clone(), &HashMap::new()).unwrap();
        assert_eq!(graph.inner.node_count(), 1); // Should not add duplicate
    }

    #[test]
    fn test_add_node_complex_dependencies() {
        println!("*************start complex test**********************");
        let mut graph = get_graph();

        let dep2_path = PathBuf::from_str("/path/to/dep2.py").unwrap();
        let dep2 = Node::mock(dep2_path.clone(), vec![]).unwrap();

        let dep1_path = PathBuf::from_str("libdep1").unwrap();
        let dep1 = Node::mock(dep1_path.clone(), vec![dep2_path.clone()]).unwrap();

        let dep3_path = PathBuf::from_str("libdep3").unwrap();
        let dep3 = Node::mock(dep3_path.clone(), vec![dep2_path.clone()]).unwrap();

        let main_path = PathBuf::from_str("/python").unwrap();
        let main = Node::mock(main_path, vec![dep1_path, dep3_path]).unwrap();

        graph.add_node(main.clone());
        graph.add_node(dep1.clone());
        graph.add_node(dep2.clone());
        graph.add_node(dep3.clone());

        let result = graph.add_tree(main.clone(), &HashMap::new());
        println!("*************end complex adding**********************");
        assert!(result.is_ok());
        assert_eq!(graph.inner.node_count(), 4);
        assert_eq!(graph.inner.edge_count(), 4);
        assert!(graph.idx_by_path.contains_right(&main.path));
        assert!(graph.idx_by_path.contains_right(&dep1.path));
        assert!(graph.idx_by_path.contains_right(&dep2.path));

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
