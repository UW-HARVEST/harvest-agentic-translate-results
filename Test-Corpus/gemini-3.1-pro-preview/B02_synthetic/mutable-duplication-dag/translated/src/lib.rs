use std::cell::RefCell;
use std::rc::Rc;

pub const MAX_CITY_NAME: usize = 64;
pub const MAX_EDGES: usize = 10;
pub const MAX_NODES: usize = 100;

#[derive(Clone)]
pub struct Edge {
    pub destination: Rc<RefCell<Node>>,
    pub distance: i32,
}

pub struct Node {
    pub city_name: String,
    pub ref_count: i32,
    pub edges: Vec<Edge>,
}

pub struct Graph {
    pub nodes: Vec<Rc<RefCell<Node>>>,
}

pub fn create_graph() -> Option<Graph> {
    Some(Graph { nodes: Vec::new() })
}

pub fn add_node(graph: &mut Graph, city_name: &str) -> Option<Rc<RefCell<Node>>> {
    if graph.nodes.len() >= MAX_NODES {
        eprintln!("Error: Graph is full (max {} nodes)", MAX_NODES);
        return None;
    }
    for node in &graph.nodes {
        if node.borrow().city_name == city_name {
            eprintln!("Error: Node '{}' already exists", city_name);
            return None;
        }
    }
    let node = Rc::new(RefCell::new(Node {
        city_name: city_name.to_string(),
        ref_count: 1,
        edges: Vec::new(),
    }));
    graph.nodes.push(Rc::clone(&node));
    Some(node)
}

pub fn add_edge(from: &Rc<RefCell<Node>>, to: &Rc<RefCell<Node>>, distance: i32) -> i32 {
    let mut from_borrow = from.borrow_mut();
    if from_borrow.edges.len() >= MAX_EDGES {
        eprintln!("Error: Node '{}' has maximum edges", from_borrow.city_name);
        return -1;
    }
    if distance < 0 {
        eprintln!("Error: Negative distance not allowed");
        return -1;
    }
    for edge in &from_borrow.edges {
        if Rc::ptr_eq(&edge.destination, to) {
            eprintln!("Error: Edge already exists");
            return -1;
        }
    }
    from_borrow.edges.push(Edge {
        destination: Rc::clone(to),
        distance,
    });
    0
}

pub fn delete_node(node: &Rc<RefCell<Node>>) {
    let mut n = node.borrow_mut();
    n.ref_count -= 1;
}

fn increment_refs_recursive(node: &Rc<RefCell<Node>>, visited: &mut Vec<Rc<RefCell<Node>>>) {
    for v in visited.iter() {
        if Rc::ptr_eq(v, node) {
            return;
        }
    }
    if visited.len() < MAX_NODES {
        visited.push(Rc::clone(node));
    }
    node.borrow_mut().ref_count += 1;
    let edges = node.borrow().edges.clone();
    for edge in edges {
        increment_refs_recursive(&edge.destination, visited);
    }
}

pub fn shallow_copy(start: &Rc<RefCell<Node>>) -> Option<Rc<RefCell<Node>>> {
    let mut visited = Vec::new();
    increment_refs_recursive(start, &mut visited);
    Some(Rc::clone(start))
}

pub fn find_shortest_path(
    start: &Rc<RefCell<Node>>,
    end: &Rc<RefCell<Node>>,
) -> Option<Vec<Rc<RefCell<Node>>>> {
    #[derive(Clone)]
    struct DijkstraNode {
        node: Rc<RefCell<Node>>,
        distance: i32,
        previous: Option<Rc<RefCell<Node>>>,
        visited: bool,
    }

    let mut state: Vec<DijkstraNode> = Vec::new();
    state.push(DijkstraNode {
        node: Rc::clone(start),
        distance: 0,
        previous: None,
        visited: false,
    });

    let mut current = Some(Rc::clone(start));

    while let Some(curr) = current {
        let current_idx = state.iter().position(|s| Rc::ptr_eq(&s.node, &curr)).unwrap();
        state[current_idx].visited = true;

        if Rc::ptr_eq(&curr, end) {
            break;
        }

        let edges = curr.borrow().edges.clone();
        for edge in edges {
            let neighbor = &edge.destination;
            let new_distance = state[current_idx].distance.saturating_add(edge.distance);

            let neighbor_idx = state.iter().position(|s| Rc::ptr_eq(&s.node, neighbor));
            
            let n_idx = if let Some(idx) = neighbor_idx {
                idx
            } else if state.len() < MAX_NODES {
                let idx = state.len();
                state.push(DijkstraNode {
                    node: Rc::clone(neighbor),
                    distance: i32::MAX,
                    previous: None,
                    visited: false,
                });
                idx
            } else {
                continue;
            };

            if new_distance < state[n_idx].distance {
                state[n_idx].distance = new_distance;
                state[n_idx].previous = Some(Rc::clone(&curr));
            }
        }

        let mut min_distance = i32::MAX;
        let mut next_current = None;
        for s in &state {
            if !s.visited && s.distance < min_distance {
                min_distance = s.distance;
                next_current = Some(Rc::clone(&s.node));
            }
        }
        current = next_current;
    }

    let end_idx = state.iter().position(|s| Rc::ptr_eq(&s.node, end));
    if let Some(idx) = end_idx {
        if state[idx].distance == i32::MAX {
            eprintln!("No path found");
            return None;
        }
    } else {
        eprintln!("No path found");
        return None;
    }

    let mut path = Vec::new();
    let mut current_node = Some(Rc::clone(end));

    while let Some(curr) = current_node {
        path.push(Rc::clone(&curr));
        let current_state_idx = state.iter().position(|s| Rc::ptr_eq(&s.node, &curr)).unwrap();
        current_node = state[current_state_idx].previous.clone();
    }

    path.reverse();
    Some(path)
}

pub fn get_node_by_name(graph: &Graph, city_name: &str) -> Option<Rc<RefCell<Node>>> {
    for node in &graph.nodes {
        if node.borrow().city_name == city_name {
            return Some(Rc::clone(node));
        }
    }
    None
}

pub fn print_node(node: &Rc<RefCell<Node>>) {
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

pub fn print_graph(graph: &Graph) {
    println!("Graph with {} nodes:", graph.nodes.len());
    for node in &graph.nodes {
        print_node(node);
    }
}

pub fn free_graph(graph: Graph) {
    for node in &graph.nodes {
        delete_node(node);
    }
}
