use std::io::{self, Read, Write};

const MAX_INPUT_DATA: usize = 255;
const MAX_CITY_NAME_DATA: usize = 63;
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

        if self.nodes.iter().any(|node| node.city_name == city_name) {
            let _ = err.write_all(b"Error: Node '");
            let _ = err.write_all(city_name);
            let _ = err.write_all(b"' already exists\n");
            return None;
        }

        let stored_len = city_name.len().min(MAX_CITY_NAME_DATA);
        self.nodes.push(Node {
            city_name: city_name[..stored_len].to_vec(),
            ref_count: 1,
            edges: Vec::new(),
        });
        Some(self.nodes.len() - 1)
    }

    fn get_node_by_name(&self, city_name: &[u8]) -> Option<usize> {
        self.nodes
            .iter()
            .position(|node| node.city_name == city_name)
    }

    fn add_edge(&mut self, from: usize, to: usize, distance: i32, err: &mut impl Write) -> bool {
        if self.nodes[from].edges.len() >= MAX_EDGES {
            let _ = err.write_all(b"Error: Node '");
            let _ = err.write_all(&self.nodes[from].city_name);
            let _ = err.write_all(b"' has maximum edges\n");
            return false;
        }

        if distance < 0 {
            let _ = err.write_all(b"Error: Negative distance not allowed\n");
            return false;
        }

        if self.nodes[from]
            .edges
            .iter()
            .any(|edge| edge.destination == to)
        {
            let _ = err.write_all(b"Error: Edge already exists\n");
            return false;
        }

        self.nodes[from].edges.push(Edge {
            destination: to,
            distance,
        });
        true
    }

    fn shallow_copy(&mut self, start: usize) -> usize {
        let mut visited = [false; MAX_NODES];
        self.increment_refs_recursive(start, &mut visited);
        start
    }

    fn increment_refs_recursive(&mut self, node_index: usize, visited: &mut [bool; MAX_NODES]) {
        if visited[node_index] {
            return;
        }

        visited[node_index] = true;
        self.nodes[node_index].ref_count = self.nodes[node_index].ref_count.wrapping_add(1);
        let destinations: Vec<usize> = self.nodes[node_index]
            .edges
            .iter()
            .map(|edge| edge.destination)
            .collect();

        for destination in destinations {
            self.increment_refs_recursive(destination, visited);
        }
    }

    fn delete_node(&mut self, node_index: usize) {
        self.nodes[node_index].ref_count = self.nodes[node_index].ref_count.wrapping_sub(1);
    }

    fn find_shortest_path(
        &self,
        start: usize,
        end: usize,
        err: &mut impl Write,
    ) -> Option<Vec<usize>> {
        struct DijkstraNode {
            node: usize,
            distance: i32,
            previous: Option<usize>,
            visited: bool,
        }

        let mut state = Vec::with_capacity(MAX_NODES);
        state.push(DijkstraNode {
            node: start,
            distance: 0,
            previous: None,
            visited: false,
        });

        let mut current = Some(start);
        while let Some(current_node) = current {
            let Some(current_index) = state.iter().position(|item| item.node == current_node)
            else {
                break;
            };

            state[current_index].visited = true;
            if current_node == end {
                break;
            }

            for edge in &self.nodes[current_node].edges {
                let new_distance = state[current_index].distance.wrapping_add(edge.distance);
                let mut neighbor_index =
                    state.iter().position(|item| item.node == edge.destination);

                if neighbor_index.is_none() && state.len() < MAX_NODES {
                    state.push(DijkstraNode {
                        node: edge.destination,
                        distance: i32::MAX,
                        previous: None,
                        visited: false,
                    });
                    neighbor_index = Some(state.len() - 1);
                }

                if let Some(index) = neighbor_index {
                    if new_distance < state[index].distance {
                        state[index].distance = new_distance;
                        state[index].previous = Some(current_node);
                    }
                }
            }

            let mut minimum_distance = i32::MAX;
            current = None;
            for item in &state {
                if !item.visited && item.distance < minimum_distance {
                    minimum_distance = item.distance;
                    current = Some(item.node);
                }
            }
        }

        let Some(end_index) = state.iter().position(|item| item.node == end) else {
            let _ = err.write_all(b"No path found\n");
            return None;
        };
        if state[end_index].distance == i32::MAX {
            let _ = err.write_all(b"No path found\n");
            return None;
        }

        let mut path = Vec::with_capacity(MAX_NODES);
        let mut current_node = Some(end);
        while let Some(node_index) = current_node {
            path.push(node_index);
            let Some(state_index) = state.iter().position(|item| item.node == node_index) else {
                break;
            };
            current_node = state[state_index].previous;
        }
        path.reverse();
        Some(path)
    }

    fn print_node(&self, node_index: usize, out: &mut impl Write) {
        let node = &self.nodes[node_index];
        let _ = out.write_all(b"City: ");
        let _ = out.write_all(&node.city_name);
        let _ = writeln!(out, " (ref_count: {})", node.ref_count);
        let _ = out.write_all(b"  Edges:\n");
        for edge in &node.edges {
            let _ = out.write_all(b"    -> ");
            let _ = out.write_all(&self.nodes[edge.destination].city_name);
            let _ = writeln!(out, " (distance: {})", edge.distance);
        }
    }

    fn print_graph(&self, out: &mut impl Write) {
        let _ = writeln!(out, "Graph with {} nodes:", self.nodes.len());
        for index in 0..self.nodes.len() {
            self.print_node(index, out);
        }
    }
}

struct Fgets<R> {
    inner: R,
}

impl<R: Read> Fgets<R> {
    fn new(inner: R) -> Self {
        Self { inner }
    }

    fn read(&mut self) -> Option<Vec<u8>> {
        let mut input = Vec::with_capacity(MAX_INPUT_DATA);
        let mut byte = [0_u8; 1];

        while input.len() < MAX_INPUT_DATA {
            match self.inner.read(&mut byte) {
                Ok(0) => break,
                Ok(_) => {
                    input.push(byte[0]);
                    if byte[0] == b'\n' {
                        break;
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => return None,
            }
        }

        if input.is_empty() {
            None
        } else {
            Some(input)
        }
    }
}

fn c_string(input: &[u8]) -> &[u8] {
    match input.iter().position(|byte| *byte == 0) {
        Some(end) => &input[..end],
        None => input,
    }
}

fn chomp_newline(input: &[u8]) -> Vec<u8> {
    let visible = c_string(input);
    match visible.iter().position(|byte| *byte == b'\n') {
        Some(end) => visible[..end].to_vec(),
        None => visible.to_vec(),
    }
}

fn parse_decimal_int(input: &[u8]) -> Option<i32> {
    let input = c_string(input);
    let mut index = 0;
    while index < input.len() && matches!(input[index], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    {
        index += 1;
    }

    let negative = if input.get(index) == Some(&b'-') {
        index += 1;
        true
    } else {
        if input.get(index) == Some(&b'+') {
            index += 1;
        }
        false
    };

    if !input.get(index).is_some_and(u8::is_ascii_digit) {
        return None;
    }

    let limit = if negative {
        (i64::MAX as u64) + 1
    } else {
        i64::MAX as u64
    };
    let mut magnitude = 0_u64;
    while let Some(byte) = input.get(index) {
        if !byte.is_ascii_digit() {
            break;
        }
        magnitude = magnitude
            .saturating_mul(10)
            .saturating_add((byte - b'0') as u64)
            .min(limit);
        index += 1;
    }

    let value = if negative {
        if magnitude == (i64::MAX as u64) + 1 {
            i64::MIN
        } else {
            -(magnitude as i64)
        }
    } else {
        magnitude as i64
    };
    Some(value as i32)
}

fn print_menu(out: &mut impl Write) {
    let _ = out.write_all(
        b"\n=== DAG City Route Manager ===\n\
1. Add city (node)\n\
2. Add route (edge)\n\
3. Show all cities\n\
4. Show city details\n\
5. Find shortest path\n\
6. Make shallow copy of subsection\n\
7. Delete node\n\
8. Exit\n\
Choice: ",
    );
}

fn write_city_not_found(out: &mut impl Write, city_name: &[u8]) {
    let _ = out.write_all(b"City '");
    let _ = out.write_all(city_name);
    let _ = out.write_all(b"' not found\n");
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let stderr = io::stderr();
    let mut input_reader = Fgets::new(stdin.lock());
    let mut out = stdout.lock();
    let mut err = stderr.lock();
    let mut graph = Graph::new();

    let _ = out.write_all(b"City Route Management System\n");
    let _ = out.write_all(b"Commands are read from stdin\n");

    loop {
        print_menu(&mut out);
        let _ = out.flush();

        let Some(input) = input_reader.read() else {
            break;
        };
        let Some(choice) = parse_decimal_int(&input) else {
            let _ = out.write_all(b"Invalid input\n");
            continue;
        };

        match choice {
            1 => {
                let _ = out.write_all(b"Enter city name: ");
                let _ = out.flush();
                let Some(input) = input_reader.read() else {
                    continue;
                };
                let city_name = chomp_newline(&input);

                if graph.add_node(&city_name, &mut err).is_some() {
                    let _ = out.write_all(b"Added city: ");
                    let _ = out.write_all(&city_name);
                    let _ = out.write_all(b"\n");
                } else {
                    let _ = out.write_all(b"Failed to add city\n");
                }
            }
            2 => {
                let _ = out.write_all(b"Enter from city: ");
                let _ = out.flush();
                let Some(from_input) = input_reader.read() else {
                    continue;
                };
                let from_city = chomp_newline(&from_input);

                let _ = out.write_all(b"Enter to city: ");
                let _ = out.flush();
                let Some(to_input) = input_reader.read() else {
                    continue;
                };
                let to_city = chomp_newline(&to_input);

                let _ = out.write_all(b"Enter distance: ");
                let _ = out.flush();
                let Some(distance_input) = input_reader.read() else {
                    continue;
                };
                let Some(distance) = parse_decimal_int(&distance_input) else {
                    let _ = out.write_all(b"Invalid distance\n");
                    continue;
                };

                let from = graph.get_node_by_name(&from_city);
                let to = graph.get_node_by_name(&to_city);
                let Some(from) = from else {
                    write_city_not_found(&mut out, &from_city);
                    continue;
                };
                let Some(to) = to else {
                    write_city_not_found(&mut out, &to_city);
                    continue;
                };

                if graph.add_edge(from, to, distance, &mut err) {
                    let _ = out.write_all(b"Added route: ");
                    let _ = out.write_all(&from_city);
                    let _ = out.write_all(b" -> ");
                    let _ = out.write_all(&to_city);
                    let _ = writeln!(out, " (distance: {distance})");
                } else {
                    let _ = out.write_all(b"Failed to add route\n");
                }
            }
            3 => graph.print_graph(&mut out),
            4 => {
                let _ = out.write_all(b"Enter city name: ");
                let _ = out.flush();
                let Some(input) = input_reader.read() else {
                    continue;
                };
                let city_name = chomp_newline(&input);

                if let Some(node) = graph.get_node_by_name(&city_name) {
                    graph.print_node(node, &mut out);
                } else {
                    write_city_not_found(&mut out, &city_name);
                }
            }
            5 => {
                let _ = out.write_all(b"Enter start city: ");
                let _ = out.flush();
                let Some(start_input) = input_reader.read() else {
                    continue;
                };
                let start_city = chomp_newline(&start_input);

                let _ = out.write_all(b"Enter end city: ");
                let _ = out.flush();
                let Some(end_input) = input_reader.read() else {
                    continue;
                };
                let end_city = chomp_newline(&end_input);

                let start = graph.get_node_by_name(&start_city);
                let end = graph.get_node_by_name(&end_city);
                let Some(start) = start else {
                    write_city_not_found(&mut out, &start_city);
                    continue;
                };
                let Some(end) = end else {
                    write_city_not_found(&mut out, &end_city);
                    continue;
                };

                if let Some(path) = graph.find_shortest_path(start, end, &mut err) {
                    let _ = out.write_all(b"Shortest path from ");
                    let _ = out.write_all(&start_city);
                    let _ = out.write_all(b" to ");
                    let _ = out.write_all(&end_city);
                    let _ = out.write_all(b":\n");
                    for (index, node) in path.iter().enumerate() {
                        let _ = write!(out, "  {}. ", index + 1);
                        let _ = out.write_all(&graph.nodes[*node].city_name);
                        let _ = out.write_all(b"\n");
                    }
                } else {
                    let _ = out.write_all(b"No path found\n");
                }
            }
            6 => {
                let _ = out.write_all(b"Enter start city for shallow copy: ");
                let _ = out.flush();
                let Some(input) = input_reader.read() else {
                    continue;
                };
                let city_name = chomp_newline(&input);

                let Some(node) = graph.get_node_by_name(&city_name) else {
                    write_city_not_found(&mut out, &city_name);
                    continue;
                };

                let copy = graph.shallow_copy(node);
                let _ = out.write_all(b"Created shallow copy starting from ");
                let _ = out.write_all(&city_name);
                let _ = out.write_all(b"\n");
                let _ = out.write_all(b"Reference counts incremented for all reachable nodes\n");
                graph.print_node(copy, &mut out);
            }
            7 => {
                let _ = out.write_all(b"Enter city name to delete: ");
                let _ = out.flush();
                let Some(input) = input_reader.read() else {
                    continue;
                };
                let city_name = chomp_newline(&input);

                let Some(node) = graph.get_node_by_name(&city_name) else {
                    write_city_not_found(&mut out, &city_name);
                    continue;
                };

                let _ = writeln!(out, "Current ref count: {}", graph.nodes[node].ref_count);
                graph.delete_node(node);
                let _ = out.write_all(b"Decremented reference count for ");
                let _ = out.write_all(&city_name);
                let _ = out.write_all(b"\n");
                let _ = out.write_all(b"Note: Node will be freed when ref count reaches 0\n");
            }
            8 => {
                let _ = out.write_all(b"Freeing graph and exiting...\n");
                let _ = out.flush();
                return;
            }
            _ => {
                let _ = out.write_all(b"Invalid choice\n");
            }
        }
    }

    let _ = out.flush();
}
