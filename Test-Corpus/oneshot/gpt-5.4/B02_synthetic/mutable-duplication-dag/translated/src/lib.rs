use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::rc::Rc;

pub const MAX_CITY_NAME: usize = 64;
pub const MAX_EDGES: usize = 10;
pub const MAX_NODES: usize = 100;

pub type node_t = Rc<RefCell<Node>>;
pub type graph_t = Graph;

#[derive(Clone)]
pub struct edge_t {
    pub destination: node_t,
    pub distance: i32,
}

pub struct Node {
    pub city_name: String,
    pub ref_count: i32,
    pub edges: Vec<edge_t>,
}

pub struct Graph {
    pub nodes: Vec<node_t>,
}

pub fn create_graph() -> Option<graph_t> {
    Some(Graph { nodes: Vec::new() })
}

pub fn add_node(graph: &mut graph_t, city_name: &str) -> Option<node_t> {
    if graph.nodes.len() >= MAX_NODES {
        eprintln!("Error: Graph is full (max {} nodes)", MAX_NODES);
        return None;
    }

    if graph
        .nodes
        .iter()
        .any(|n| n.borrow().city_name == city_name)
    {
        eprintln!("Error: Node '{}' already exists", city_name);
        return None;
    }

    let truncated: String = city_name.chars().take(MAX_CITY_NAME - 1).collect();
    let node = Rc::new(RefCell::new(Node {
        city_name: truncated,
        ref_count: 1,
        edges: Vec::new(),
    }));
    graph.nodes.push(node.clone());
    Some(node)
}

pub fn add_edge(from: &node_t, to: &node_t, distance: i32) -> i32 {
    if distance < 0 {
        eprintln!("Error: Negative distance not allowed");
        return -1;
    }

    let mut from_borrow = from.borrow_mut();
    if from_borrow.edges.len() >= MAX_EDGES {
        eprintln!("Error: Node '{}' has maximum edges", from_borrow.city_name);
        return -1;
    }

    if from_borrow
        .edges
        .iter()
        .any(|e| Rc::ptr_eq(&e.destination, to))
    {
        eprintln!("Error: Edge already exists");
        return -1;
    }

    from_borrow.edges.push(edge_t {
        destination: to.clone(),
        distance,
    });
    0
}

pub fn delete_node(node: &node_t) {
    let mut n = node.borrow_mut();
    n.ref_count -= 1;
}

fn increment_refs_recursive(node: &node_t, visited: &mut HashSet<usize>) {
    let ptr = Rc::as_ptr(node) as usize;
    if visited.contains(&ptr) {
        return;
    }
    visited.insert(ptr);

    let edges = {
        let mut n = node.borrow_mut();
        n.ref_count += 1;
        n.edges.clone()
    };

    for edge in edges {
        increment_refs_recursive(&edge.destination, visited);
    }
}

pub fn shallow_copy(start: &node_t) -> Option<node_t> {
    let mut visited = HashSet::new();
    increment_refs_recursive(start, &mut visited);
    Some(start.clone())
}

#[derive(Copy, Clone, Eq, PartialEq)]
struct State {
    cost: i32,
    position: usize,
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .cmp(&self.cost)
            .then_with(|| self.position.cmp(&other.position))
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn find_shortest_path(start: &node_t, end: &node_t, path_length: &mut i32) -> Option<Vec<node_t>> {
    let start_ptr = Rc::as_ptr(start) as usize;
    let end_ptr = Rc::as_ptr(end) as usize;

    let mut dist: HashMap<usize, i32> = HashMap::new();
    let mut prev: HashMap<usize, usize> = HashMap::new();
    let mut nodes: HashMap<usize, node_t> = HashMap::new();
    let mut order: Vec<usize> = Vec::new();
    let mut heap = BinaryHeap::new();

    dist.insert(start_ptr, 0);
    nodes.insert(start_ptr, start.clone());
    order.push(start_ptr);
    heap.push(State {
        cost: 0,
        position: start_ptr,
    });

    while let Some(State { cost, position }) = heap.pop() {
        let Some(&best) = dist.get(&position) else {
            continue;
        };
        if cost > best {
            continue;
        }
        if position == end_ptr {
            break;
        }

        let Some(current_node) = nodes.get(&position).cloned() else {
            continue;
        };
        let edges = current_node.borrow().edges.clone();

        for edge in edges {
            let next_ptr = Rc::as_ptr(&edge.destination) as usize;
            let next_cost = cost.saturating_add(edge.distance);

            if !nodes.contains_key(&next_ptr) {
                nodes.insert(next_ptr, edge.destination.clone());
                order.push(next_ptr);
            }

            let is_shorter = match dist.get(&next_ptr) {
                Some(&existing) => next_cost < existing,
                None => true,
            };

            if is_shorter {
                dist.insert(next_ptr, next_cost);
                prev.insert(next_ptr, position);
                heap.push(State {
                    cost: next_cost,
                    position: next_ptr,
                });
            }
        }

        if order.len() >= MAX_NODES {
            break;
        }
    }

    if !dist.contains_key(&end_ptr) {
        eprintln!("No path found");
        *path_length = 0;
        return None;
    }

    let mut path_ptrs = Vec::new();
    let mut current = end_ptr;
    path_ptrs.push(current);
    while current != start_ptr {
        let Some(&p) = prev.get(&current) else {
            break;
        };
        current = p;
        path_ptrs.push(current);
    }
    path_ptrs.reverse();

    let mut result = Vec::new();
    for ptr in path_ptrs {
        if let Some(node) = nodes.get(&ptr) {
            result.push(node.clone());
        }
    }

    *path_length = result.len() as i32;
    Some(result)
}

pub fn free_graph(graph: &mut graph_t) {
    for node in &graph.nodes {
        delete_node(node);
    }
    graph.nodes.clear();
}

pub fn get_node_by_name(graph: &graph_t, city_name: &str) -> Option<node_t> {
    graph
        .nodes
        .iter()
        .find(|n| n.borrow().city_name == city_name)
        .cloned()
}

pub fn print_node(node: &node_t) {
    let n = node.borrow();
    println!("City: {} (ref_count: {})", n.city_name, n.ref_count);
    println!("  Edges:");
    for edge in &n.edges {
        println!(
            "    -> {} (distance: {})",
            edge.destination.borrow().city_name,
            edge.distance
        );
    }
}

pub fn print_graph(graph: &graph_t) {
    println!("Graph with {} nodes:", graph.nodes.len());
    for node in &graph.nodes {
        print_node(node);
    }
}
