use std::io::{self, BufWriter, Read, Write};

const MAX_INPUT: usize = 256;
const MAX_CITY_NAME: usize = 64;
const MAX_EDGES: usize = 10;
const MAX_NODES: usize = 100;

#[derive(Clone, Copy)]
struct Edge {
    destination: usize,
    distance: i32,
}

struct Node {
    city_name: Vec<u8>,
    ref_count: i32,
    edges: Vec<Edge>,
}

struct Graph {
    nodes: Vec<Node>,
}

impl Graph {
    fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    fn add_node(&mut self, city_name: &[u8], err: &mut impl Write) -> Option<usize> {
        if self.nodes.len() >= MAX_NODES {
            let _ = writeln!(err, "Error: Graph is full (max {MAX_NODES} nodes)");
            return None;
        }

        if self
            .nodes
            .iter()
            .any(|node| node.city_name.as_slice() == city_name)
        {
            let _ = write!(err, "Error: Node '");
            let _ = err.write_all(city_name);
            let _ = writeln!(err, "' already exists");
            return None;
        }

        let node = Node {
            city_name: city_name[..city_name.len().min(MAX_CITY_NAME - 1)].to_vec(),
            ref_count: 1,
            edges: Vec::new(),
        };
        self.nodes.push(node);
        Some(self.nodes.len() - 1)
    }

    fn get_node_by_name(&self, city_name: &[u8]) -> Option<usize> {
        self.nodes
            .iter()
            .position(|node| node.city_name.as_slice() == city_name)
    }

    fn add_edge(&mut self, from: usize, to: usize, distance: i32, err: &mut impl Write) -> bool {
        if self.nodes[from].edges.len() >= MAX_EDGES {
            let _ = write!(err, "Error: Node '");
            let _ = err.write_all(&self.nodes[from].city_name);
            let _ = writeln!(err, "' has maximum edges");
            return false;
        }

        if distance < 0 {
            let _ = writeln!(err, "Error: Negative distance not allowed");
            return false;
        }

        if self.nodes[from]
            .edges
            .iter()
            .any(|edge| edge.destination == to)
        {
            let _ = writeln!(err, "Error: Edge already exists");
            return false;
        }

        self.nodes[from].edges.push(Edge {
            destination: to,
            distance,
        });
        true
    }

    fn delete_node(&mut self, node: usize) {
        self.nodes[node].ref_count = self.nodes[node].ref_count.wrapping_sub(1);
    }

    fn shallow_copy(&mut self, start: usize) -> usize {
        let mut visited = [false; MAX_NODES];
        self.increment_refs_recursive(start, &mut visited);
        start
    }

    fn increment_refs_recursive(&mut self, node: usize, visited: &mut [bool; MAX_NODES]) {
        if visited[node] {
            return;
        }

        visited[node] = true;
        self.nodes[node].ref_count = self.nodes[node].ref_count.wrapping_add(1);

        let edges = self.nodes[node].edges.clone();
        for edge in edges {
            self.increment_refs_recursive(edge.destination, visited);
        }
    }

    fn find_shortest_path(&self, start: usize, end: usize) -> Option<Vec<usize>> {
        struct State {
            node: usize,
            distance: i32,
            previous: Option<usize>,
            visited: bool,
        }

        let mut state = Vec::with_capacity(MAX_NODES);
        state.push(State {
            node: start,
            distance: 0,
            previous: None,
            visited: false,
        });

        let mut current = Some(start);
        while let Some(current_node) = current {
            let Some(current_idx) = state.iter().position(|item| item.node == current_node) else {
                break;
            };

            state[current_idx].visited = true;
            if current_node == end {
                break;
            }

            let current_distance = state[current_idx].distance;
            for edge in &self.nodes[current_node].edges {
                let new_distance = current_distance.wrapping_add(edge.distance);
                let mut neighbor_idx = state.iter().position(|item| item.node == edge.destination);

                if neighbor_idx.is_none() && state.len() < MAX_NODES {
                    neighbor_idx = Some(state.len());
                    state.push(State {
                        node: edge.destination,
                        distance: i32::MAX,
                        previous: None,
                        visited: false,
                    });
                }

                if let Some(index) = neighbor_idx {
                    if new_distance < state[index].distance {
                        state[index].distance = new_distance;
                        state[index].previous = Some(current_node);
                    }
                }
            }

            let mut min_distance = i32::MAX;
            current = None;
            for item in &state {
                if !item.visited && item.distance < min_distance {
                    min_distance = item.distance;
                    current = Some(item.node);
                }
            }
        }

        let end_idx = state.iter().position(|item| item.node == end)?;
        if state[end_idx].distance == i32::MAX {
            return None;
        }

        let mut path = Vec::new();
        let mut current_node = Some(end);
        while let Some(node) = current_node {
            path.push(node);
            let Some(index) = state.iter().position(|item| item.node == node) else {
                break;
            };
            current_node = state[index].previous;
        }
        path.reverse();
        Some(path)
    }

    fn print_node(&self, node: usize, out: &mut impl Write) {
        let _ = write!(out, "City: ");
        let _ = out.write_all(&self.nodes[node].city_name);
        let _ = writeln!(out, " (ref_count: {})", self.nodes[node].ref_count);
        let _ = writeln!(out, "  Edges:");
        for edge in &self.nodes[node].edges {
            let _ = write!(out, "    -> ");
            let _ = out.write_all(&self.nodes[edge.destination].city_name);
            let _ = writeln!(out, " (distance: {})", edge.distance);
        }
    }

    fn print_graph(&self, out: &mut impl Write) {
        let _ = writeln!(out, "Graph with {} nodes:", self.nodes.len());
        for node in 0..self.nodes.len() {
            self.print_node(node, out);
        }
    }

    fn free_graph(&mut self) {
        for node in 0..self.nodes.len() {
            self.delete_node(node);
        }
    }
}

fn fgets(input: &mut impl Read) -> Option<Vec<u8>> {
    let mut buffer = Vec::with_capacity(MAX_INPUT);
    let mut byte = [0_u8; 1];

    while buffer.len() < MAX_INPUT - 1 {
        match input.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buffer.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => return None,
        }
    }

    if buffer.is_empty() {
        None
    } else {
        Some(buffer)
    }
}

fn c_string(bytes: &[u8]) -> &[u8] {
    let end = bytes
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(bytes.len());
    &bytes[..end]
}

fn city_input(bytes: &[u8]) -> &[u8] {
    let bytes = c_string(bytes);
    let end = bytes
        .iter()
        .position(|&byte| byte == b'\n')
        .unwrap_or(bytes.len());
    &bytes[..end]
}

fn scan_decimal_i32(bytes: &[u8]) -> Option<i32> {
    let bytes = c_string(bytes);
    let mut index = 0;
    while index < bytes.len() && matches!(bytes[index], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    {
        index += 1;
    }

    let negative = match bytes.get(index) {
        Some(b'-') => {
            index += 1;
            true
        }
        Some(b'+') => {
            index += 1;
            false
        }
        _ => false,
    };

    let start = index;
    let limit = if negative {
        (i64::MAX as u64) + 1
    } else {
        i64::MAX as u64
    };
    let mut value = 0_u64;
    while let Some(&digit @ b'0'..=b'9') = bytes.get(index) {
        value = value
            .saturating_mul(10)
            .saturating_add(u64::from(digit - b'0'))
            .min(limit);
        index += 1;
    }
    if index == start {
        return None;
    }

    if negative {
        Some(0_u64.wrapping_sub(value) as i32)
    } else {
        Some(value as i32)
    }
}

fn print_menu(out: &mut impl Write) {
    let _ = writeln!(out, "\n=== DAG City Route Manager ===");
    let _ = writeln!(out, "1. Add city (node)");
    let _ = writeln!(out, "2. Add route (edge)");
    let _ = writeln!(out, "3. Show all cities");
    let _ = writeln!(out, "4. Show city details");
    let _ = writeln!(out, "5. Find shortest path");
    let _ = writeln!(out, "6. Make shallow copy of subsection");
    let _ = writeln!(out, "7. Delete node");
    let _ = writeln!(out, "8. Exit");
    let _ = write!(out, "Choice: ");
}

fn write_quoted_not_found(out: &mut impl Write, city: &[u8]) {
    let _ = write!(out, "City '");
    let _ = out.write_all(city);
    let _ = writeln!(out, "' not found");
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let stderr = io::stderr();
    let mut input_stream = stdin.lock();
    let mut out = BufWriter::new(stdout.lock());
    let mut err = stderr.lock();
    let mut graph = Graph::new();

    let _ = writeln!(out, "City Route Management System");
    let _ = writeln!(out, "Commands are read from stdin");

    loop {
        print_menu(&mut out);

        let Some(input) = fgets(&mut input_stream) else {
            break;
        };
        let Some(choice) = scan_decimal_i32(&input) else {
            let _ = writeln!(out, "Invalid input");
            continue;
        };

        match choice {
            1 => {
                let _ = write!(out, "Enter city name: ");
                let Some(input) = fgets(&mut input_stream) else {
                    continue;
                };
                let city = city_input(&input);

                if graph.add_node(city, &mut err).is_some() {
                    let _ = write!(out, "Added city: ");
                    let _ = out.write_all(city);
                    let _ = writeln!(out);
                } else {
                    let _ = writeln!(out, "Failed to add city");
                }
            }
            2 => {
                let _ = write!(out, "Enter from city: ");
                let Some(from_input) = fgets(&mut input_stream) else {
                    continue;
                };
                let from_city = city_input(&from_input);

                let _ = write!(out, "Enter to city: ");
                let Some(to_input) = fgets(&mut input_stream) else {
                    continue;
                };
                let to_city = city_input(&to_input);

                let _ = write!(out, "Enter distance: ");
                let Some(distance_input) = fgets(&mut input_stream) else {
                    continue;
                };
                let Some(distance) = scan_decimal_i32(&distance_input) else {
                    let _ = writeln!(out, "Invalid distance");
                    continue;
                };

                let from = graph.get_node_by_name(from_city);
                let to = graph.get_node_by_name(to_city);

                let Some(from) = from else {
                    write_quoted_not_found(&mut out, from_city);
                    continue;
                };
                let Some(to) = to else {
                    write_quoted_not_found(&mut out, to_city);
                    continue;
                };

                if graph.add_edge(from, to, distance, &mut err) {
                    let _ = write!(out, "Added route: ");
                    let _ = out.write_all(from_city);
                    let _ = write!(out, " -> ");
                    let _ = out.write_all(to_city);
                    let _ = writeln!(out, " (distance: {distance})");
                } else {
                    let _ = writeln!(out, "Failed to add route");
                }
            }
            3 => graph.print_graph(&mut out),
            4 => {
                let _ = write!(out, "Enter city name: ");
                let Some(input) = fgets(&mut input_stream) else {
                    continue;
                };
                let city = city_input(&input);

                if let Some(node) = graph.get_node_by_name(city) {
                    graph.print_node(node, &mut out);
                } else {
                    write_quoted_not_found(&mut out, city);
                }
            }
            5 => {
                let _ = write!(out, "Enter start city: ");
                let Some(start_input) = fgets(&mut input_stream) else {
                    continue;
                };
                let start_city = city_input(&start_input);

                let _ = write!(out, "Enter end city: ");
                let Some(end_input) = fgets(&mut input_stream) else {
                    continue;
                };
                let end_city = city_input(&end_input);

                let start = graph.get_node_by_name(start_city);
                let end = graph.get_node_by_name(end_city);

                let Some(start) = start else {
                    write_quoted_not_found(&mut out, start_city);
                    continue;
                };
                let Some(end) = end else {
                    write_quoted_not_found(&mut out, end_city);
                    continue;
                };

                if let Some(path) = graph.find_shortest_path(start, end) {
                    let _ = write!(out, "Shortest path from ");
                    let _ = out.write_all(start_city);
                    let _ = write!(out, " to ");
                    let _ = out.write_all(end_city);
                    let _ = writeln!(out, ":");
                    for (index, node) in path.iter().enumerate() {
                        let _ = write!(out, "  {}. ", index + 1);
                        let _ = out.write_all(&graph.nodes[*node].city_name);
                        let _ = writeln!(out);
                    }
                } else {
                    let _ = writeln!(err, "No path found");
                    let _ = writeln!(out, "No path found");
                }
            }
            6 => {
                let _ = write!(out, "Enter start city for shallow copy: ");
                let Some(input) = fgets(&mut input_stream) else {
                    continue;
                };
                let city = city_input(&input);

                let Some(node) = graph.get_node_by_name(city) else {
                    write_quoted_not_found(&mut out, city);
                    continue;
                };

                let copy = graph.shallow_copy(node);
                let _ = write!(out, "Created shallow copy starting from ");
                let _ = out.write_all(city);
                let _ = writeln!(out);
                let _ = writeln!(out, "Reference counts incremented for all reachable nodes");
                graph.print_node(copy, &mut out);
            }
            7 => {
                let _ = write!(out, "Enter city name to delete: ");
                let Some(input) = fgets(&mut input_stream) else {
                    continue;
                };
                let city = city_input(&input);

                let Some(node) = graph.get_node_by_name(city) else {
                    write_quoted_not_found(&mut out, city);
                    continue;
                };

                let _ = writeln!(out, "Current ref count: {}", graph.nodes[node].ref_count);
                graph.delete_node(node);
                let _ = write!(out, "Decremented reference count for ");
                let _ = out.write_all(city);
                let _ = writeln!(out);
                let _ = writeln!(out, "Note: Node will be freed when ref count reaches 0");
            }
            8 => {
                let _ = writeln!(out, "Freeing graph and exiting...");
                graph.free_graph();
                return;
            }
            _ => {
                let _ = writeln!(out, "Invalid choice");
            }
        }
    }

    graph.free_graph();
}
