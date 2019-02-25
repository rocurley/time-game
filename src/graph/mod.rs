use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

pub struct Node<'a, N: 'a, E: 'a> {
    pub val: N,
    graph: &'a Graph<N, E>,
}
impl<'a, N, E> Node<'a, N, E>
where
    N: Hash,
    N: Eq,
    N: Copy,
    E: Hash,
    E: Eq,
    E: Copy,
{
    pub fn neighbors(&'a self) -> Box<Iterator<Item = Node<'a, N, E>> + 'a> {
        let edges = self.graph.nodes.get(&self.val).expect("Invalid node state");
        Box::new(edges.iter().map(move |edge| {
            Node {
                val: self
                    .graph
                    .edges
                    .get(edge)
                    .expect("Invalid graph state")
                    .clone(),
                graph: self.graph,
            }
        }))
    }
    pub fn connected_to(&self, target: N) -> bool {
        self.connected_to_helper(target, &mut HashSet::new())
    }
    fn connected_to_helper(&self, target: N, seen: &mut HashSet<N>) -> bool {
        if self.val == target {
            return true;
        }
        if seen.contains(&self.val) {
            return false;
        }
        seen.insert(self.val);
        for n in self.neighbors() {
            if n.connected_to_helper(target, seen) {
                return true;
            }
        }
        false
    }
}

#[derive(Clone)]
pub struct Graph<N, E> {
    pub nodes: HashMap<N, Vec<E>>,
    pub edges: HashMap<E, N>,
}

impl<'a, N, E> Graph<N, E>
where
    N: Hash,
    N: Eq,
    N: Copy,
    E: Hash,
    E: Eq,
    E: Copy,
{
    pub fn new() -> Self {
        Graph {
            nodes: HashMap::new(),
            edges: HashMap::new(),
        }
    }
    pub fn insert_node(&mut self, val: N, incoming: Vec<(N, E)>, outgoing: Vec<(N, E)>) {
        for &(n, e) in incoming.iter() {
            self.nodes
                .get_mut(&n)
                .expect("invalid incoming node")
                .push(e);
            match self.edges.entry(e) {
                Entry::Occupied(_) => panic!("Edge already exists"),
                Entry::Vacant(entry) => entry.insert(val.clone()),
            };
        }
        let mut outgoing_edges = Vec::new();
        for &(n, e) in outgoing.iter() {
            outgoing_edges.push(e);
            match self.edges.entry(e) {
                Entry::Occupied(_) => panic!("Edge already exists"),
                Entry::Vacant(entry) => entry.insert(n.clone()),
            };
        }
        self.nodes.insert(val, outgoing_edges);
    }
    pub fn get_node(&'a self, val: N) -> Node<'a, N, E> {
        Node { val, graph: self }
    }
}
