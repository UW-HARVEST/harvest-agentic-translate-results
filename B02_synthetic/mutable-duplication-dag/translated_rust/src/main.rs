use std::io::{self, BufRead, Write};

const MAX_CITY_NAME: usize = 64;
const MAX_EDGES: usize = 10;
const MAX_NODES: usize = 100;

struct Edge {
    destination: usize, // index into graph.nodes
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
        eprint!("Error: NULL parameter in add_node\n");
        return None;
    }

    if graph.nodes.len() >= MAX_NODES {
        eprint!("Error: Graph is full (max {} nodes)\n", MAX_NODES);
        return None;
    }

    // Check if node already exists
    for node in &graph.nodes {
        if node.city_name == city_name {
            eprint!("Error: Node '{}' already exists\n", city_name);
            return None;
        }
    }

    let mut name = city_name.to_string();
    name.truncate(MAX_CITY_NAME - 1);

    let node = Node {
        city_name: name,
        ref_count: 1,
        edges: Vec::new(),
    };
    let idx = graph.nodes.len();
    graph.nodes.push(node);
    Some(idx)
}

fn add_edge(graph: &mut Graph, from: usize, to: usize, distance: i32) -> i32 {
    if graph.nodes[from].edges.len() >= MAX_EDGES {
        eprint!("Error: Node '{}' has maximum edges\n", graph.nodes[from].city_name);
        return -1;
    }

    if distance < 0 {
        eprint!("Error: Negative distance not allowed\n");
        return -1;
    }

    // Check for duplicate edge
    for e in &graph.nodes[from].edges {
        if e.destination == to {
            eprint!("Error: Edge already exists\n");
            return -1;
        }
    }

    graph.nodes[from].edges.push(Edge { destination: to, distance });
    0
}

fn delete_node(graph: &mut Graph, idx: usize) {
    graph.nodes[idx].ref_count -= 1;
    // In C this frees when ref_count == 0; we just decrement.
}

fn increment_refs_recursive(graph: &mut Graph, node_idx: usize, visited: &mut Vec<usize>) {
    if visited.contains(&node_idx) {
        return;
    }
    if visited.len() < MAX_NODES {
        visited.push(node_idx);
    }
    graph.nodes[node_idx].ref_count += 1;

    let edge_dests: Vec<usize> = graph.nodes[node_idx].edges.iter().map(|e| e.destination).collect();
    for dest in edge_dests {
        increment_refs_recursive(graph, dest, visited);
    }
}

fn shallow_copy(graph: &mut Graph, start: usize) -> usize {
    let mut visited = Vec::new();
    increment_refs_recursive(graph, start, &mut visited);
    start
}

fn get_node_by_name(graph: &Graph, city_name: &str) -> Option<usize> {
    for (i, node) in graph.nodes.iter().enumerate() {
        if node.city_name == city_name {
            return Some(i);
        }
    }
    None
}

fn print_node(graph: &Graph, idx: usize) {
    let node = &graph.nodes[idx];
    print!("City: {} (ref_count: {})\n", node.city_name, node.ref_count);
    print!("  Edges:\n");
    for e in &node.edges {
        print!("    -> {} (distance: {})\n",
               graph.nodes[e.destination].city_name, e.distance);
    }
}

fn print_graph(graph: &Graph) {
    print!("Graph with {} nodes:\n", graph.nodes.len());
    for i in 0..graph.nodes.len() {
        print_node(graph, i);
    }
}

fn find_shortest_path(graph: &Graph, start: usize, end: usize) -> Option<Vec<usize>> {
    struct DState {
        node: usize,
        distance: i32,
        previous: Option<usize>, // index into state vec
        visited: bool,
    }

    let mut state: Vec<DState> = Vec::new();
    state.push(DState { node: start, distance: 0, previous: None, visited: false });

    let mut current: Option<usize> = Some(start);

    while let Some(cur_node) = current {
        // Find current node in state
        let current_idx = state.iter().position(|s| s.node == cur_node);
        let current_idx = match current_idx {
            Some(i) => i,
            None => break,
        };

        state[current_idx].visited = true;

        if cur_node == end {
            break;
        }

        let cur_dist = state[current_idx].distance;

        // Collect neighbor info
        let neighbors: Vec<(usize, i32)> = graph.nodes[cur_node]
            .edges.iter()
            .map(|e| (e.destination, e.distance))
            .collect();

        for (neighbor, edge_dist) in neighbors {
            let new_distance = cur_dist + edge_dist;

            let neighbor_idx = state.iter().position(|s| s.node == neighbor);

            let ni = if let Some(ni) = neighbor_idx {
                ni
            } else if state.len() < MAX_NODES {
                let ni = state.len();
                state.push(DState {
                    node: neighbor,
                    distance: i32::MAX,
                    previous: None,
                    visited: false,
                });
                ni
            } else {
                continue;
            };

            if new_distance < state[ni].distance {
                state[ni].distance = new_distance;
                state[ni].previous = Some(current_idx);
            }
        }

        // Find next unvisited with min distance
        let mut min_dist = i32::MAX;
        current = None;
        for s in &state {
            if !s.visited && s.distance < min_dist {
                min_dist = s.distance;
                current = Some(s.node);
            }
        }
    }

    // Find end in state
    let end_idx = state.iter().position(|s| s.node == end);
    let end_idx = match end_idx {
        Some(i) if state[i].distance != i32::MAX => i,
        _ => {
            eprint!("No path found\n");
            return None;
        }
    };

    // Reconstruct path
    let mut path_indices = Vec::new();
    let mut ci = Some(end_idx);
    while let Some(i) = ci {
        path_indices.push(state[i].node);
        ci = state[i].previous;
    }
    path_indices.reverse();
    Some(path_indices)
}

fn read_line(_stdin: &io::Stdin, lock: &mut io::StdinLock) -> Option<String> {
    let mut buf = String::new();
    match lock.read_line(&mut buf) {
        Ok(0) => None,
        Ok(_) => Some(buf),
        Err(_) => None,
    }
}

fn main() {
    let mut graph = match create_graph() {
        Some(g) => g,
        None => {
            eprint!("Failed to create graph\n");
            std::process::exit(1);
        }
    };

    let stdin = io::stdin();
    let mut lock = stdin.lock();

    print!("City Route Management System\n");
    print!("Commands are read from stdin\n");

    loop {
        print!("\n=== DAG City Route Manager ===\n");
        print!("1. Add city (node)\n");
        print!("2. Add route (edge)\n");
        print!("3. Show all cities\n");
        print!("4. Show city details\n");
        print!("5. Find shortest path\n");
        print!("6. Make shallow copy of subsection\n");
        print!("7. Delete node\n");
        print!("8. Exit\n");
        print!("Choice: ");
        let _ = io::stdout().flush();

        let line = match read_line(&stdin, &mut lock) {
            Some(l) => l,
            None => break,
        };

        let choice: i32 = match line.trim().parse() {
            Ok(v) => v,
            Err(_) => {
                print!("Invalid input\n");
                continue;
            }
        };

        match choice {
            1 => {
                print!("Enter city name: ");
                let _ = io::stdout().flush();
                let line = match read_line(&stdin, &mut lock) {
                    Some(l) => l,
                    None => break,
                };
                let name = line.trim_end_matches('\n').to_string();

                if add_node(&mut graph, &name).is_some() {
                    print!("Added city: {}\n", name);
                } else {
                    print!("Failed to add city\n");
                }
            }
            2 => {
                print!("Enter from city: ");
                let _ = io::stdout().flush();
                let line = match read_line(&stdin, &mut lock) {
                    Some(l) => l,
                    None => break,
                };
                let from_city = line.trim_end_matches('\n').to_string();

                print!("Enter to city: ");
                let _ = io::stdout().flush();
                let line = match read_line(&stdin, &mut lock) {
                    Some(l) => l,
                    None => break,
                };
                let to_city = line.trim_end_matches('\n').to_string();

                print!("Enter distance: ");
                let _ = io::stdout().flush();
                let line = match read_line(&stdin, &mut lock) {
                    Some(l) => l,
                    None => break,
                };
                let distance: i32 = match line.trim().parse() {
                    Ok(v) => v,
                    Err(_) => {
                        print!("Invalid distance\n");
                        continue;
                    }
                };

                let from = get_node_by_name(&graph, &from_city);
                let to = get_node_by_name(&graph, &to_city);

                let from = match from {
                    Some(f) => f,
                    None => {
                        print!("City '{}' not found\n", from_city);
                        continue;
                    }
                };
                let to = match to {
                    Some(t) => t,
                    None => {
                        print!("City '{}' not found\n", to_city);
                        continue;
                    }
                };

                if add_edge(&mut graph, from, to, distance) == 0 {
                    print!("Added route: {} -> {} (distance: {})\n",
                           from_city, to_city, distance);
                } else {
                    print!("Failed to add route\n");
                }
            }
            3 => {
                print_graph(&graph);
            }
            4 => {
                print!("Enter city name: ");
                let _ = io::stdout().flush();
                let line = match read_line(&stdin, &mut lock) {
                    Some(l) => l,
                    None => break,
                };
                let name = line.trim_end_matches('\n').to_string();

                match get_node_by_name(&graph, &name) {
                    Some(idx) => print_node(&graph, idx),
                    None => print!("City '{}' not found\n", name),
                }
            }
            5 => {
                print!("Enter start city: ");
                let _ = io::stdout().flush();
                let line = match read_line(&stdin, &mut lock) {
                    Some(l) => l,
                    None => break,
                };
                let start_city = line.trim_end_matches('\n').to_string();

                print!("Enter end city: ");
                let _ = io::stdout().flush();
                let line = match read_line(&stdin, &mut lock) {
                    Some(l) => l,
                    None => break,
                };
                let end_city = line.trim_end_matches('\n').to_string();

                let start = match get_node_by_name(&graph, &start_city) {
                    Some(s) => s,
                    None => {
                        print!("City '{}' not found\n", start_city);
                        continue;
                    }
                };
                let end = match get_node_by_name(&graph, &end_city) {
                    Some(e) => e,
                    None => {
                        print!("City '{}' not found\n", end_city);
                        continue;
                    }
                };

                match find_shortest_path(&graph, start, end) {
                    Some(path) => {
                        print!("Shortest path from {} to {}:\n", start_city, end_city);
                        for (i, &node_idx) in path.iter().enumerate() {
                            print!("  {}. {}\n", i + 1, graph.nodes[node_idx].city_name);
                        }
                    }
                    None => {
                        print!("No path found\n");
                    }
                }
            }
            6 => {
                print!("Enter start city for shallow copy: ");
                let _ = io::stdout().flush();
                let line = match read_line(&stdin, &mut lock) {
                    Some(l) => l,
                    None => break,
                };
                let name = line.trim_end_matches('\n').to_string();

                let node_idx = match get_node_by_name(&graph, &name) {
                    Some(i) => i,
                    None => {
                        print!("City '{}' not found\n", name);
                        continue;
                    }
                };

                let copy_idx = shallow_copy(&mut graph, node_idx);
                print!("Created shallow copy starting from {}\n", name);
                print!("Reference counts incremented for all reachable nodes\n");
                print_node(&graph, copy_idx);
            }
            7 => {
                print!("Enter city name to delete: ");
                let _ = io::stdout().flush();
                let line = match read_line(&stdin, &mut lock) {
                    Some(l) => l,
                    None => break,
                };
                let name = line.trim_end_matches('\n').to_string();

                let node_idx = match get_node_by_name(&graph, &name) {
                    Some(i) => i,
                    None => {
                        print!("City '{}' not found\n", name);
                        continue;
                    }
                };

                print!("Current ref count: {}\n", graph.nodes[node_idx].ref_count);
                delete_node(&mut graph, node_idx);
                print!("Decremented reference count for {}\n", name);
                print!("Note: Node will be freed when ref count reaches 0\n");
            }
            8 => {
                print!("Freeing graph and exiting...\n");
                return;
            }
            _ => {
                print!("Invalid choice\n");
            }
        }
    }
}
