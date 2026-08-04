use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

pub const MAX_CITY_NAME: usize = 64;
pub const MAX_EDGES: usize = 10;
pub const MAX_NODES: usize = 100;

#[derive(Debug, Clone)]
pub struct Edge {
    pub destination: Arc<RwLock<Node>>,
    pub distance: i32,
}

#[derive(Debug)]
pub struct Node {
    pub city_name: String,
    pub edges: Vec<Edge>,
}

pub struct Graph {
    pub nodes: HashMap<String, Arc<RwLock<Node>>>,
}

pub fn create_graph() -> Option<Graph> {
    Some(Graph {
        nodes: HashMap::new(),
    })
}

pub fn add_node(graph: &mut Graph, city_name: &str) -> Option<Arc<RwLock<Node>>> {
    if graph.nodes.len() >= MAX_NODES {
        eprintln!("Error: Graph is full (max {} nodes)", MAX_NODES);
        return None;
    }
    
    if graph.nodes.contains_key(city_name) {
        eprintln!("Error: Node '{}' already exists", city_name);
        return None;
    }
    
    let node = Arc::new(RwLock::new(Node {
        city_name: city_name.to_string(),
        edges: Vec::new(),
    }));
    
    graph.nodes.insert(city_name.to_string(), Arc::clone(&node));
    
    Some(node)
}

pub fn add_edge(from: &Arc<RwLock<Node>>, to: &Arc<RwLock<Node>>, distance: i32) -> i32 {
    if distance < 0 {
        eprintln!("Error: Negative distance not allowed");
        return -1;
    }
    
    let mut from_guard = from.write().unwrap();
    
    if from_guard.edges.len() >= MAX_EDGES {
        eprintln!("Error: Node '{}' has maximum edges", from_guard.city_name);
        return -1;
    }
    
    for edge in &from_guard.edges {
        if Arc::ptr_eq(&edge.destination, to) {
            eprintln!("Error: Edge already exists");
            return -1;
        }
    }
    
    from_guard.edges.push(Edge {
        destination: Arc::clone(to),
        distance,
    });
    
    0
}

pub fn delete_node(node: &Arc<RwLock<Node>>) {
    let _ = node;
}

fn increment_refs_recursive(
    node: &Arc<RwLock<Node>>,
    visited: &mut HashSet<String>,
) {
    let guard = node.read().unwrap();
    if visited.contains(&guard.city_name) {
        return;
    }
    visited.insert(guard.city_name.clone());
    
    for edge in &guard.edges {
        increment_refs_recursive(&edge.destination, visited);
    }
}

pub fn shallow_copy(start: &Arc<RwLock<Node>>) -> Option<Arc<RwLock<Node>>> {
    let mut visited = HashSet::new();
    increment_refs_recursive(start, &mut visited);
    Some(Arc::clone(start))
}

#[derive(Clone)]
struct DijkstraState {
    node: Arc<RwLock<Node>>,
    distance: i32,
    previous: Option<Arc<RwLock<Node>>>,
    visited: bool,
}

pub fn find_shortest_path(
    start: &Arc<RwLock<Node>>,
    end: &Arc<RwLock<Node>>,
) -> Option<Vec<Arc<RwLock<Node>>>> {
    let mut states: HashMap<String, DijkstraState> = HashMap::new();
    
    let start_name = start.read().unwrap().city_name.clone();
    states.insert(
        start_name.clone(),
        DijkstraState {
            node: Arc::clone(start),
            distance: 0,
            previous: None,
            visited: false,
        },
    );
    
    let mut current: Option<Arc<RwLock<Node>>> = Some(Arc::clone(start));
    
    while let Some(curr) = current {
        let curr_name = curr.read().unwrap().city_name.clone();
        
        if let Some(state) = states.get_mut(&curr_name) {
            state.visited = true;
        }
        
        if Arc::ptr_eq(&curr, end) {
            break;
        }
        
        let curr_guard = curr.read().unwrap();
        let curr_distance = states.get(&curr_name).unwrap().distance;
        
        for edge in &curr_guard.edges {
            let neighbor_name = edge.destination.read().unwrap().city_name.clone();
            let new_distance = curr_distance + edge.distance;
            
            if let Some(neighbor_state) = states.get_mut(&neighbor_name) {
                if new_distance < neighbor_state.distance {
                    neighbor_state.distance = new_distance;
                    neighbor_state.previous = Some(Arc::clone(&curr));
                }
            } else {
                states.insert(
                    neighbor_name,
                    DijkstraState {
                        node: Arc::clone(&edge.destination),
                        distance: new_distance,
                        previous: Some(Arc::clone(&curr)),
                        visited: false,
                    },
                );
            }
        }
        
        let mut min_distance = i32::MAX;
        current = None;
        
        for state in states.values() {
            if !state.visited && state.distance < min_distance {
                min_distance = state.distance;
                current = Some(Arc::clone(&state.node));
            }
        }
    }
    
    let end_name = end.read().unwrap().city_name.clone();
    let end_state = states.get(&end_name)?;
    
    if end_state.distance == i32::MAX {
        eprintln!("No path found");
        return None;
    }
    
    let mut path: Vec<Arc<RwLock<Node>>> = Vec::new();
    let mut current_node: Option<Arc<RwLock<Node>>> = Some(Arc::clone(end));
    
    while let Some(node) = current_node {
        path.push(Arc::clone(&node));
        let node_name = node.read().unwrap().city_name.clone();
        current_node = states.get(&node_name).and_then(|s| s.previous.clone());
    }
    
    path.reverse();
    Some(path)
}

pub fn get_node_by_name(graph: &Graph, city_name: &str) -> Option<Arc<RwLock<Node>>> {
    graph.nodes.get(city_name).map(|n| Arc::clone(n))
}

pub fn print_node(node: &Arc<RwLock<Node>>) {
    let guard = node.read().unwrap();
    println!("City: {}", guard.city_name);
    println!("  Edges:");
    for edge in &guard.edges {
        let dest_guard = edge.destination.read().unwrap();
        println!("    -> {} (distance: {})", dest_guard.city_name, edge.distance);
    }
}

pub fn print_graph(graph: &Graph) {
    println!("Graph with {} nodes:", graph.nodes.len());
    for node in graph.nodes.values() {
        print_node(node);
    }
}

pub fn free_graph(graph: Graph) {
    let _ = graph;
}
