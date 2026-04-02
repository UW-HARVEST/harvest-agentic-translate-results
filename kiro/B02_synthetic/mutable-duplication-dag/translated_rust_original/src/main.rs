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

// --- Library functions ---

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
    for node in &graph.nodes {
        if node.city_name == city_name {
            eprint!("Error: Node '{}' already exists\n", city_name);
            return None;
        }
    }
    let mut name = city_name.to_string();
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
        eprint!(
            "Error: Node '{}' has maximum edges\n",
            graph.nodes[from].city_name
        );
        return -1;
    }
    if distance < 0 {
        eprint!("Error: Negative distance not allowed\n");
        return -1;
    }
    for e in &graph.nodes[from].edges {
        if e.destination == to {
            eprint!("Error: Edge already exists\n");
            return -1;
        }
    }
    graph.nodes[from].edges.push(Edge {
        destination: to,
        distance,
    });
    0
}

fn delete_node(graph: &mut Graph, idx: usize) {
    graph.nodes[idx].ref_count -= 1;
    // In C this frees the node when ref_count == 0.
    // We keep the node in the vec (no actual free) to match the C behavior
    // where the graph still holds a dangling pointer.
}

fn increment_refs_recursive(graph: &mut Graph, node_idx: usize, visited: &mut Vec<usize>) {
    if visited.contains(&node_idx) {
        return;
    }
    if visited.len() < MAX_NODES {
        visited.push(node_idx);
    }
    graph.nodes[node_idx].ref_count += 1;
    let destinations: Vec<usize> = graph.nodes[node_idx]
        .edges
        .iter()
        .map(|e| e.destination)
        .collect();
    for dest in destinations {
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
    previous: Option<usize>, // index into state vec
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
        let current_idx = state.iter().position(|s| s.node == cur);
        let current_idx = match current_idx {
            Some(i) => i,
            None => break,
        };

        state[current_idx].visited = true;

        if cur == end {
            break;
        }

        let cur_dist = state[current_idx].distance;
        let edges: Vec<(usize, i32)> = graph.nodes[cur]
            .edges
            .iter()
            .map(|e| (e.destination, e.distance))
            .collect();

        for (neighbor, edge_dist) in edges {
            let new_distance = cur_dist + edge_dist;
            let neighbor_idx = state.iter().position(|s| s.node == neighbor);

            let neighbor_idx = match neighbor_idx {
                Some(i) => i,
                None => {
                    if state.len() < MAX_NODES {
                        let idx = state.len();
                        state.push(DijkstraNode {
                            node: neighbor,
                            distance: i32::MAX,
                            previous: None,
                            visited: false,
                        });
                        idx
                    } else {
                        continue;
                    }
                }
            };

            if new_distance < state[neighbor_idx].distance {
                state[neighbor_idx].distance = new_distance;
                state[neighbor_idx].previous = Some(current_idx);
            }
        }

        let mut min_distance = i32::MAX;
        current = None;
        for s in &state {
            if !s.visited && s.distance < min_distance {
                min_distance = s.distance;
                current = Some(s.node);
            }
        }
    }

    let end_idx = state.iter().position(|s| s.node == end);
    let end_idx = match end_idx {
        Some(i) => i,
        None => {
            eprint!("No path found\n");
            return None;
        }
    };

    if state[end_idx].distance == i32::MAX {
        eprint!("No path found\n");
        return None;
    }

    // Reconstruct path
    let mut path = Vec::new();
    let mut cur_state_idx = Some(end_idx);
    while let Some(idx) = cur_state_idx {
        path.push(state[idx].node);
        cur_state_idx = state[idx].previous;
    }
    path.reverse();
    Some(path)
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
        print!(
            "    -> {} (distance: {})\n",
            graph.nodes[e.destination].city_name, e.distance
        );
    }
}

fn print_graph(graph: &Graph) {
    print!("Graph with {} nodes:\n", graph.nodes.len());
    for i in 0..graph.nodes.len() {
        print_node(graph, i);
    }
}

fn free_graph(graph: &mut Graph) {
    for i in 0..graph.nodes.len() {
        graph.nodes[i].ref_count -= 1;
    }
}

// --- Main ---

fn print_menu() {
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
    io::stdout().flush().unwrap();
}

fn read_line(stdin: &io::Stdin, buf: &mut String) -> bool {
    buf.clear();
    match stdin.lock().read_line(buf) {
        Ok(0) => false,
        Ok(_) => true,
        Err(_) => false,
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
    let mut input = String::new();

    print!("City Route Management System\n");
    print!("Commands are read from stdin\n");

    loop {
        print_menu();

        if !read_line(&stdin, &mut input) {
            break;
        }

        let choice: i32 = match input.trim().parse() {
            Ok(v) => v,
            Err(_) => {
                print!("Invalid input\n");
                io::stdout().flush().unwrap();
                continue;
            }
        };

        match choice {
            1 => {
                // Add city
                print!("Enter city name: ");
                io::stdout().flush().unwrap();
                if !read_line(&stdin, &mut input) {
                    break;
                }
                let name = input.trim_end_matches('\n').to_string();
                if add_node(&mut graph, &name).is_some() {
                    print!("Added city: {}\n", name);
                } else {
                    print!("Failed to add city\n");
                }
                io::stdout().flush().unwrap();
            }
            2 => {
                // Add route
                print!("Enter from city: ");
                io::stdout().flush().unwrap();
                let mut from_city = String::new();
                if !read_line(&stdin, &mut from_city) {
                    break;
                }
                let from_city = from_city.trim_end_matches('\n').to_string();

                print!("Enter to city: ");
                io::stdout().flush().unwrap();
                let mut to_city = String::new();
                if !read_line(&stdin, &mut to_city) {
                    break;
                }
                let to_city = to_city.trim_end_matches('\n').to_string();

                print!("Enter distance: ");
                io::stdout().flush().unwrap();
                if !read_line(&stdin, &mut input) {
                    break;
                }
                let distance: i32 = match input.trim().parse() {
                    Ok(v) => v,
                    Err(_) => {
                        print!("Invalid distance\n");
                        io::stdout().flush().unwrap();
                        continue;
                    }
                };

                let from_idx = get_node_by_name(&graph, &from_city);
                let to_idx = get_node_by_name(&graph, &to_city);

                let from_idx = match from_idx {
                    Some(i) => i,
                    None => {
                        print!("City '{}' not found\n", from_city);
                        io::stdout().flush().unwrap();
                        continue;
                    }
                };
                let to_idx = match to_idx {
                    Some(i) => i,
                    None => {
                        print!("City '{}' not found\n", to_city);
                        io::stdout().flush().unwrap();
                        continue;
                    }
                };

                if add_edge(&mut graph, from_idx, to_idx, distance) == 0 {
                    print!(
                        "Added route: {} -> {} (distance: {})\n",
                        from_city, to_city, distance
                    );
                } else {
                    print!("Failed to add route\n");
                }
                io::stdout().flush().unwrap();
            }
            3 => {
                // Show all cities
                print_graph(&graph);
                io::stdout().flush().unwrap();
            }
            4 => {
                // Show city details
                print!("Enter city name: ");
                io::stdout().flush().unwrap();
                if !read_line(&stdin, &mut input) {
                    break;
                }
                let name = input.trim_end_matches('\n').to_string();
                match get_node_by_name(&graph, &name) {
                    Some(idx) => print_node(&graph, idx),
                    None => print!("City '{}' not found\n", name),
                }
                io::stdout().flush().unwrap();
            }
            5 => {
                // Find shortest path
                print!("Enter start city: ");
                io::stdout().flush().unwrap();
                let mut start_city = String::new();
                if !read_line(&stdin, &mut start_city) {
                    break;
                }
                let start_city = start_city.trim_end_matches('\n').to_string();

                print!("Enter end city: ");
                io::stdout().flush().unwrap();
                let mut end_city = String::new();
                if !read_line(&stdin, &mut end_city) {
                    break;
                }
                let end_city = end_city.trim_end_matches('\n').to_string();

                let start_idx = match get_node_by_name(&graph, &start_city) {
                    Some(i) => i,
                    None => {
                        print!("City '{}' not found\n", start_city);
                        io::stdout().flush().unwrap();
                        continue;
                    }
                };
                let end_idx = match get_node_by_name(&graph, &end_city) {
                    Some(i) => i,
                    None => {
                        print!("City '{}' not found\n", end_city);
                        io::stdout().flush().unwrap();
                        continue;
                    }
                };

                match find_shortest_path(&graph, start_idx, end_idx) {
                    Some(path) => {
                        print!(
                            "Shortest path from {} to {}:\n",
                            start_city, end_city
                        );
                        for (i, &node_idx) in path.iter().enumerate() {
                            print!(
                                "  {}. {}\n",
                                i + 1,
                                graph.nodes[node_idx].city_name
                            );
                        }
                    }
                    None => {
                        print!("No path found\n");
                    }
                }
                io::stdout().flush().unwrap();
            }
            6 => {
                // Make shallow copy
                print!("Enter start city for shallow copy: ");
                io::stdout().flush().unwrap();
                if !read_line(&stdin, &mut input) {
                    break;
                }
                let name = input.trim_end_matches('\n').to_string();
                let node_idx = match get_node_by_name(&graph, &name) {
                    Some(i) => i,
                    None => {
                        print!("City '{}' not found\n", name);
                        io::stdout().flush().unwrap();
                        continue;
                    }
                };

                match shallow_copy(&mut graph, node_idx) {
                    Some(copy_idx) => {
                        print!("Created shallow copy starting from {}\n", name);
                        print!("Reference counts incremented for all reachable nodes\n");
                        print_node(&graph, copy_idx);
                    }
                    None => {
                        print!("Failed to create shallow copy\n");
                    }
                }
                io::stdout().flush().unwrap();
            }
            7 => {
                // Delete node
                print!("Enter city name to delete: ");
                io::stdout().flush().unwrap();
                if !read_line(&stdin, &mut input) {
                    break;
                }
                let name = input.trim_end_matches('\n').to_string();
                let node_idx = match get_node_by_name(&graph, &name) {
                    Some(i) => i,
                    None => {
                        print!("City '{}' not found\n", name);
                        io::stdout().flush().unwrap();
                        continue;
                    }
                };

                print!("Current ref count: {}\n", graph.nodes[node_idx].ref_count);
                delete_node(&mut graph, node_idx);
                print!("Decremented reference count for {}\n", name);
                print!("Note: Node will be freed when ref count reaches 0\n");
                io::stdout().flush().unwrap();
            }
            8 => {
                // Exit
                print!("Freeing graph and exiting...\n");
                io::stdout().flush().unwrap();
                free_graph(&mut graph);
                std::process::exit(0);
            }
            _ => {
                print!("Invalid choice\n");
                io::stdout().flush().unwrap();
            }
        }
    }

    free_graph(&mut graph);
}
