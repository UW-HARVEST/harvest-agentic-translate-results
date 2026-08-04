use std::io::{self, BufRead, Write};

const MAX_CITY_NAME: usize = 64;
const MAX_EDGES: usize = 10;
const MAX_NODES: usize = 100;

struct Edge {
    destination: usize, // index into graph nodes
    distance: i32,
}

struct Node {
    city_name: String,
    ref_count: i32,
    edges: Vec<Edge>,
}

struct Graph {
    nodes: Vec<Node>,
}

fn create_graph() -> Option<Graph> {
    Some(Graph { nodes: Vec::new() })
}

fn add_node(graph: &mut Graph, city_name: &str) -> Option<usize> {
    if city_name.is_empty() {
        eprintln!("Error: NULL parameter in add_node");
        return None;
    }
    if graph.nodes.len() >= MAX_NODES {
        eprintln!("Error: Graph is full (max {} nodes)", MAX_NODES);
        return None;
    }
    for i in 0..graph.nodes.len() {
        if graph.nodes[i].city_name == city_name {
            eprintln!("Error: Node '{}' already exists", city_name);
            return None;
        }
    }
    let mut name = String::from(city_name);
    name.truncate(MAX_CITY_NAME - 1);
    let idx = graph.nodes.len();
    graph.nodes.push(Node {
        city_name: name,
        ref_count: 1,
        edges: Vec::new(),
    });
    Some(idx)
}

fn add_edge(graph: &mut Graph, from: usize, to: usize, distance: i32) -> i32 {
    if graph.nodes[from].edges.len() >= MAX_EDGES {
        eprintln!("Error: Node '{}' has maximum edges", graph.nodes[from].city_name);
        return -1;
    }
    if distance < 0 {
        eprintln!("Error: Negative distance not allowed");
        return -1;
    }
    for e in &graph.nodes[from].edges {
        if e.destination == to {
            eprintln!("Error: Edge already exists");
            return -1;
        }
    }
    graph.nodes[from].edges.push(Edge { destination: to, distance });
    0
}

fn delete_node(graph: &mut Graph, idx: usize) {
    graph.nodes[idx].ref_count -= 1;
    // Note: C code frees when ref_count == 0, but we don't actually remove
    // from graph (matching C behavior where the pointer stays in graph->nodes)
}

fn increment_refs_recursive(graph: &mut Graph, node_idx: usize, visited: &mut Vec<usize>) {
    for i in 0..visited.len() {
        if visited[i] == node_idx {
            return;
        }
    }
    if visited.len() < MAX_NODES {
        visited.push(node_idx);
    }
    graph.nodes[node_idx].ref_count += 1;
    let edge_count = graph.nodes[node_idx].edges.len();
    for i in 0..edge_count {
        let dest = graph.nodes[node_idx].edges[i].destination;
        increment_refs_recursive(graph, dest, visited);
    }
}

fn shallow_copy(graph: &mut Graph, start: usize) -> Option<usize> {
    let mut visited = Vec::new();
    increment_refs_recursive(graph, start, &mut visited);
    Some(start)
}

struct DijkstraNode {
    node: usize,
    distance: i32,
    previous: Option<usize>, // index into state array
    visited: bool,
}

fn find_shortest_path(graph: &Graph, start: usize, end: usize) -> Option<Vec<usize>> {
    let mut state: Vec<DijkstraNode> = Vec::new();
    state.push(DijkstraNode {
        node: start,
        distance: 0,
        previous: None,
        visited: false,
    });

    let mut current = Some(start);

    while let Some(cur) = current {
        let mut current_idx = None;
        for i in 0..state.len() {
            if state[i].node == cur {
                current_idx = Some(i);
                break;
            }
        }
        let current_idx = match current_idx {
            Some(i) => i,
            None => break,
        };

        state[current_idx].visited = true;

        if cur == end {
            break;
        }

        let cur_dist = state[current_idx].distance;
        let edge_count = graph.nodes[cur].edges.len();
        for i in 0..edge_count {
            let neighbor = graph.nodes[cur].edges[i].destination;
            let new_distance = cur_dist + graph.nodes[cur].edges[i].distance;

            let mut neighbor_idx = None;
            for j in 0..state.len() {
                if state[j].node == neighbor {
                    neighbor_idx = Some(j);
                    break;
                }
            }

            if neighbor_idx.is_none() && state.len() < MAX_NODES {
                let idx = state.len();
                state.push(DijkstraNode {
                    node: neighbor,
                    distance: i32::MAX,
                    previous: None,
                    visited: false,
                });
                neighbor_idx = Some(idx);
            }

            if let Some(ni) = neighbor_idx {
                if new_distance < state[ni].distance {
                    state[ni].distance = new_distance;
                    // Store the node index (not state index) as previous, matching C behavior
                    state[ni].previous = Some(cur);
                }
            }
        }

        let mut min_distance = i32::MAX;
        current = None;
        for i in 0..state.len() {
            if !state[i].visited && state[i].distance < min_distance {
                min_distance = state[i].distance;
                current = Some(state[i].node);
            }
        }
    }

    let mut end_idx = None;
    for i in 0..state.len() {
        if state[i].node == end {
            end_idx = Some(i);
            break;
        }
    }

    match end_idx {
        None => {
            eprintln!("No path found");
            return None;
        }
        Some(ei) => {
            if state[ei].distance == i32::MAX {
                eprintln!("No path found");
                return None;
            }
        }
    }

    // Reconstruct path - previous stores node indices
    let mut path = Vec::new();
    let mut current_node: Option<usize> = Some(end);

    while let Some(cn) = current_node {
        path.push(cn);
        // Find this node in state to get its previous
        let mut found = false;
        for i in 0..state.len() {
            if state[i].node == cn {
                current_node = state[i].previous;
                found = true;
                break;
            }
        }
        if !found {
            break;
        }
    }

    path.reverse();
    Some(path)
}

fn get_node_by_name(graph: &Graph, city_name: &str) -> Option<usize> {
    for i in 0..graph.nodes.len() {
        if graph.nodes[i].city_name == city_name {
            return Some(i);
        }
    }
    None
}

fn print_node(graph: &Graph, idx: usize) {
    let node = &graph.nodes[idx];
    println!("City: {} (ref_count: {})", node.city_name, node.ref_count);
    println!("  Edges:");
    for e in &node.edges {
        println!("    -> {} (distance: {})", graph.nodes[e.destination].city_name, e.distance);
    }
}

fn print_graph(graph: &Graph) {
    println!("Graph with {} nodes:", graph.nodes.len());
    for i in 0..graph.nodes.len() {
        print_node(graph, i);
    }
}

fn print_menu() {
    println!();
    println!("=== DAG City Route Manager ===");
    println!("1. Add city (node)");
    println!("2. Add route (edge)");
    println!("3. Show all cities");
    println!("4. Show city details");
    println!("5. Find shortest path");
    println!("6. Make shallow copy of subsection");
    println!("7. Delete node");
    println!("8. Exit");
    print!("Choice: ");
    let _ = io::stdout().flush();
}

fn read_line(stdin: &io::Stdin) -> Option<String> {
    let mut line = String::new();
    match stdin.lock().read_line(&mut line) {
        Ok(0) => None,
        Ok(_) => Some(line),
        Err(_) => None,
    }
}

fn main() {
    let mut graph = match create_graph() {
        Some(g) => g,
        None => {
            eprintln!("Failed to create graph");
            std::process::exit(1);
        }
    };

    let stdin = io::stdin();

    println!("City Route Management System");
    println!("Commands are read from stdin");

    loop {
        print_menu();

        let input = match read_line(&stdin) {
            Some(s) => s,
            None => break,
        };

        let choice: i32 = match input.trim().parse() {
            Ok(v) => v,
            Err(_) => {
                println!("Invalid input");
                continue;
            }
        };

        match choice {
            1 => {
                print!("Enter city name: ");
                let _ = io::stdout().flush();
                let input = match read_line(&stdin) {
                    Some(s) => s,
                    None => break,
                };
                let name = input.trim_end_matches('\n');
                if add_node(&mut graph, name).is_some() {
                    println!("Added city: {}", name);
                } else {
                    println!("Failed to add city");
                }
            }
            2 => {
                print!("Enter from city: ");
                let _ = io::stdout().flush();
                let from_input = match read_line(&stdin) {
                    Some(s) => s,
                    None => break,
                };
                let from_city = from_input.trim_end_matches('\n');

                print!("Enter to city: ");
                let _ = io::stdout().flush();
                let to_input = match read_line(&stdin) {
                    Some(s) => s,
                    None => break,
                };
                let to_city = to_input.trim_end_matches('\n');

                print!("Enter distance: ");
                let _ = io::stdout().flush();
                let dist_input = match read_line(&stdin) {
                    Some(s) => s,
                    None => break,
                };
                let distance: i32 = match dist_input.trim().parse() {
                    Ok(v) => v,
                    Err(_) => {
                        println!("Invalid distance");
                        continue;
                    }
                };

                let from_city_owned = from_city.to_string();
                let to_city_owned = to_city.to_string();

                let from_idx = get_node_by_name(&graph, &from_city_owned);
                let to_idx = get_node_by_name(&graph, &to_city_owned);

                if from_idx.is_none() {
                    println!("City '{}' not found", from_city_owned);
                    continue;
                }
                if to_idx.is_none() {
                    println!("City '{}' not found", to_city_owned);
                    continue;
                }

                let from_idx = from_idx.unwrap();
                let to_idx = to_idx.unwrap();

                if add_edge(&mut graph, from_idx, to_idx, distance) == 0 {
                    println!("Added route: {} -> {} (distance: {})", from_city_owned, to_city_owned, distance);
                } else {
                    println!("Failed to add route");
                }
            }
            3 => {
                print_graph(&graph);
            }
            4 => {
                print!("Enter city name: ");
                let _ = io::stdout().flush();
                let input = match read_line(&stdin) {
                    Some(s) => s,
                    None => break,
                };
                let name = input.trim_end_matches('\n');
                match get_node_by_name(&graph, name) {
                    Some(idx) => print_node(&graph, idx),
                    None => println!("City '{}' not found", name),
                }
            }
            5 => {
                print!("Enter start city: ");
                let _ = io::stdout().flush();
                let start_input = match read_line(&stdin) {
                    Some(s) => s,
                    None => break,
                };
                let start_city = start_input.trim_end_matches('\n');

                print!("Enter end city: ");
                let _ = io::stdout().flush();
                let end_input = match read_line(&stdin) {
                    Some(s) => s,
                    None => break,
                };
                let end_city = end_input.trim_end_matches('\n');

                let start_city_owned = start_city.to_string();
                let end_city_owned = end_city.to_string();

                let start_idx = get_node_by_name(&graph, &start_city_owned);
                let end_idx = get_node_by_name(&graph, &end_city_owned);

                if start_idx.is_none() {
                    println!("City '{}' not found", start_city_owned);
                    continue;
                }
                if end_idx.is_none() {
                    println!("City '{}' not found", end_city_owned);
                    continue;
                }

                let start_idx = start_idx.unwrap();
                let end_idx = end_idx.unwrap();

                match find_shortest_path(&graph, start_idx, end_idx) {
                    Some(path) => {
                        println!("Shortest path from {} to {}:", start_city_owned, end_city_owned);
                        for i in 0..path.len() {
                            println!("  {}. {}", i + 1, graph.nodes[path[i]].city_name);
                        }
                    }
                    None => {
                        println!("No path found");
                    }
                }
            }
            6 => {
                print!("Enter start city for shallow copy: ");
                let _ = io::stdout().flush();
                let input = match read_line(&stdin) {
                    Some(s) => s,
                    None => break,
                };
                let name = input.trim_end_matches('\n').to_string();

                match get_node_by_name(&graph, &name) {
                    None => {
                        println!("City '{}' not found", name);
                    }
                    Some(idx) => {
                        match shallow_copy(&mut graph, idx) {
                            Some(copy_idx) => {
                                println!("Created shallow copy starting from {}", name);
                                println!("Reference counts incremented for all reachable nodes");
                                print_node(&graph, copy_idx);
                            }
                            None => {
                                println!("Failed to create shallow copy");
                            }
                        }
                    }
                }
            }
            7 => {
                print!("Enter city name to delete: ");
                let _ = io::stdout().flush();
                let input = match read_line(&stdin) {
                    Some(s) => s,
                    None => break,
                };
                let name = input.trim_end_matches('\n').to_string();

                match get_node_by_name(&graph, &name) {
                    None => {
                        println!("City '{}' not found", name);
                    }
                    Some(idx) => {
                        println!("Current ref count: {}", graph.nodes[idx].ref_count);
                        delete_node(&mut graph, idx);
                        println!("Decremented reference count for {}", name);
                        println!("Note: Node will be freed when ref count reaches 0");
                    }
                }
            }
            8 => {
                println!("Freeing graph and exiting...");
                return;
            }
            _ => {
                println!("Invalid choice");
            }
        }
    }
}
