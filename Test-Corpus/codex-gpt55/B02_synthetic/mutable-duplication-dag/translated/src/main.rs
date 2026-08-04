use std::io::{self, Read, Write};

const MAX_INPUT: usize = 256;
const MAX_CITY_NAME: usize = 64;
const MAX_EDGES: usize = 10;
const MAX_NODES: usize = 100;

#[derive(Clone)]
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

struct FgetsReader<R: Read> {
    reader: R,
    eof: bool,
}

impl<R: Read> FgetsReader<R> {
    fn new(reader: R) -> Self {
        Self { reader, eof: false }
    }

    fn fgets(&mut self, size: usize) -> io::Result<Option<Vec<u8>>> {
        if self.eof || size == 0 {
            return Ok(None);
        }

        let mut out = Vec::new();
        let limit = size.saturating_sub(1);
        while out.len() < limit {
            let mut byte = [0u8; 1];
            let read = self.reader.read(&mut byte)?;
            if read == 0 {
                self.eof = true;
                break;
            }
            let b = byte[0];
            out.push(b);
            if b == b'\n' {
                break;
            }
        }

        if out.is_empty() {
            Ok(None)
        } else {
            Ok(Some(out))
        }
    }
}

#[derive(Clone)]
struct DijkstraNode {
    node: usize,
    distance: i32,
    previous: Option<usize>,
    visited: bool,
}

fn c_string_line(mut input: Vec<u8>) -> Vec<u8> {
    let end = input
        .iter()
        .position(|&b| b == b'\n' || b == 0)
        .unwrap_or(input.len());
    input.truncate(end);
    input
}

fn parse_c_int(input: &[u8]) -> Option<i32> {
    let end = input.iter().position(|&b| b == 0).unwrap_or(input.len());
    let mut i = 0;
    while i < end && matches!(input[i], b' ' | 0x0c | b'\n' | b'\r' | b'\t' | 0x0b) {
        i += 1;
    }

    let mut sign = 1i128;
    if i < end && (input[i] == b'+' || input[i] == b'-') {
        if input[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }

    if i >= end || !input[i].is_ascii_digit() {
        return None;
    }

    let mut value = 0i128;
    while i < end && input[i].is_ascii_digit() {
        value = value
            .saturating_mul(10)
            .saturating_add((input[i] - b'0') as i128);
        i += 1;
    }

    Some((value.saturating_mul(sign)) as i32)
}

fn print_menu(stdout: &mut dyn Write) -> io::Result<()> {
    stdout.write_all(b"\n=== DAG City Route Manager ===\n")?;
    stdout.write_all(b"1. Add city (node)\n")?;
    stdout.write_all(b"2. Add route (edge)\n")?;
    stdout.write_all(b"3. Show all cities\n")?;
    stdout.write_all(b"4. Show city details\n")?;
    stdout.write_all(b"5. Find shortest path\n")?;
    stdout.write_all(b"6. Make shallow copy of subsection\n")?;
    stdout.write_all(b"7. Delete node\n")?;
    stdout.write_all(b"8. Exit\n")?;
    stdout.write_all(b"Choice: ")
}

fn create_graph() -> Graph {
    Graph { nodes: Vec::new() }
}

fn add_node(
    graph: &mut Graph,
    city_name: &[u8],
    stderr: &mut dyn Write,
) -> io::Result<Option<usize>> {
    if graph.nodes.len() >= MAX_NODES {
        writeln!(stderr, "Error: Graph is full (max {} nodes)", MAX_NODES)?;
        return Ok(None);
    }

    for node in &graph.nodes {
        if node.city_name == city_name {
            stderr.write_all(b"Error: Node '")?;
            stderr.write_all(city_name)?;
            stderr.write_all(b"' already exists\n")?;
            return Ok(None);
        }
    }

    let mut stored = city_name.to_vec();
    if stored.len() > MAX_CITY_NAME - 1 {
        stored.truncate(MAX_CITY_NAME - 1);
    }

    let node = Node {
        city_name: stored,
        ref_count: 1,
        edges: Vec::new(),
    };
    graph.nodes.push(node);
    Ok(Some(graph.nodes.len() - 1))
}

fn get_node_by_name(graph: &Graph, city_name: &[u8]) -> Option<usize> {
    graph
        .nodes
        .iter()
        .position(|node| node.city_name == city_name)
}

fn add_edge(
    graph: &mut Graph,
    from: usize,
    to: usize,
    distance: i32,
    stderr: &mut dyn Write,
) -> io::Result<i32> {
    if graph.nodes[from].edges.len() >= MAX_EDGES {
        stderr.write_all(b"Error: Node '")?;
        stderr.write_all(&graph.nodes[from].city_name)?;
        stderr.write_all(b"' has maximum edges\n")?;
        return Ok(-1);
    }

    if distance < 0 {
        stderr.write_all(b"Error: Negative distance not allowed\n")?;
        return Ok(-1);
    }

    if graph.nodes[from]
        .edges
        .iter()
        .any(|edge| edge.destination == to)
    {
        stderr.write_all(b"Error: Edge already exists\n")?;
        return Ok(-1);
    }

    graph.nodes[from].edges.push(Edge {
        destination: to,
        distance,
    });
    Ok(0)
}

fn delete_node(graph: &mut Graph, node: usize) {
    graph.nodes[node].ref_count -= 1;
}

fn increment_refs_recursive(graph: &mut Graph, node: usize, visited: &mut Vec<usize>) {
    if visited.contains(&node) {
        return;
    }
    if visited.len() < MAX_NODES {
        visited.push(node);
    }

    graph.nodes[node].ref_count += 1;
    let edges = graph.nodes[node].edges.clone();
    for edge in edges {
        increment_refs_recursive(graph, edge.destination, visited);
    }
}

fn shallow_copy(graph: &mut Graph, start: usize) -> usize {
    let mut visited = Vec::new();
    increment_refs_recursive(graph, start, &mut visited);
    start
}

fn find_shortest_path(
    graph: &Graph,
    start: usize,
    end: usize,
    stderr: &mut dyn Write,
) -> io::Result<Option<Vec<usize>>> {
    let mut state = Vec::<DijkstraNode>::new();
    state.push(DijkstraNode {
        node: start,
        distance: 0,
        previous: None,
        visited: false,
    });

    let mut current = Some(start);
    while let Some(current_node) = current {
        let Some(current_idx) = state.iter().position(|entry| entry.node == current_node) else {
            break;
        };

        state[current_idx].visited = true;

        if current_node == end {
            break;
        }

        for edge in &graph.nodes[current_node].edges {
            let neighbor = edge.destination;
            let new_distance = state[current_idx].distance.wrapping_add(edge.distance);

            let mut neighbor_idx = state.iter().position(|entry| entry.node == neighbor);
            if neighbor_idx.is_none() && state.len() < MAX_NODES {
                state.push(DijkstraNode {
                    node: neighbor,
                    distance: i32::MAX,
                    previous: None,
                    visited: false,
                });
                neighbor_idx = Some(state.len() - 1);
            }

            if let Some(idx) = neighbor_idx {
                if new_distance < state[idx].distance {
                    state[idx].distance = new_distance;
                    state[idx].previous = Some(current_node);
                }
            }
        }

        let mut min_distance = i32::MAX;
        current = None;
        for entry in &state {
            if !entry.visited && entry.distance < min_distance {
                min_distance = entry.distance;
                current = Some(entry.node);
            }
        }
    }

    let end_idx = state.iter().position(|entry| entry.node == end);
    if end_idx.is_none() || state[end_idx.unwrap()].distance == i32::MAX {
        stderr.write_all(b"No path found\n")?;
        return Ok(None);
    }

    let mut path = Vec::new();
    let mut current_node = Some(end);
    while let Some(node) = current_node {
        path.push(node);
        let current_state_idx = state.iter().position(|entry| entry.node == node);
        let Some(idx) = current_state_idx else {
            break;
        };
        current_node = state[idx].previous;
    }

    path.reverse();
    Ok(Some(path))
}

fn print_node(graph: &Graph, node: usize, stdout: &mut dyn Write) -> io::Result<()> {
    stdout.write_all(b"City: ")?;
    stdout.write_all(&graph.nodes[node].city_name)?;
    writeln!(stdout, " (ref_count: {})", graph.nodes[node].ref_count)?;
    stdout.write_all(b"  Edges:\n")?;
    for edge in &graph.nodes[node].edges {
        stdout.write_all(b"    -> ")?;
        stdout.write_all(&graph.nodes[edge.destination].city_name)?;
        writeln!(stdout, " (distance: {})", edge.distance)?;
    }
    Ok(())
}

fn print_graph(graph: &Graph, stdout: &mut dyn Write) -> io::Result<()> {
    writeln!(stdout, "Graph with {} nodes:", graph.nodes.len())?;
    for i in 0..graph.nodes.len() {
        print_node(graph, i, stdout)?;
    }
    Ok(())
}

fn free_graph(graph: &mut Graph) {
    for i in 0..graph.nodes.len() {
        delete_node(graph, i);
    }
}

fn city_not_found(stdout: &mut dyn Write, city: &[u8]) -> io::Result<()> {
    stdout.write_all(b"City '")?;
    stdout.write_all(city)?;
    stdout.write_all(b"' not found\n")
}

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut reader = FgetsReader::new(stdin.lock());

    let mut stdout = io::BufWriter::new(io::stdout());
    let mut stderr = io::BufWriter::new(io::stderr());
    let mut graph = create_graph();

    stdout.write_all(b"City Route Management System\n")?;
    stdout.write_all(b"Commands are read from stdin\n")?;

    loop {
        print_menu(&mut stdout)?;

        let Some(input) = reader.fgets(MAX_INPUT)? else {
            break;
        };

        let Some(choice) = parse_c_int(&input) else {
            stdout.write_all(b"Invalid input\n")?;
            continue;
        };

        match choice {
            1 => {
                stdout.write_all(b"Enter city name: ")?;
                let Some(input) = reader.fgets(MAX_INPUT)? else {
                    continue;
                };
                let city = c_string_line(input);
                let node = add_node(&mut graph, &city, &mut stderr)?;
                if node.is_some() {
                    stdout.write_all(b"Added city: ")?;
                    stdout.write_all(&city)?;
                    stdout.write_all(b"\n")?;
                } else {
                    stdout.write_all(b"Failed to add city\n")?;
                }
            }
            2 => {
                stdout.write_all(b"Enter from city: ")?;
                let Some(from_input) = reader.fgets(MAX_INPUT)? else {
                    continue;
                };
                let from_city = c_string_line(from_input);

                stdout.write_all(b"Enter to city: ")?;
                let Some(to_input) = reader.fgets(MAX_INPUT)? else {
                    continue;
                };
                let to_city = c_string_line(to_input);

                stdout.write_all(b"Enter distance: ")?;
                let Some(distance_input) = reader.fgets(MAX_INPUT)? else {
                    continue;
                };
                let Some(distance) = parse_c_int(&distance_input) else {
                    stdout.write_all(b"Invalid distance\n")?;
                    continue;
                };

                let from = get_node_by_name(&graph, &from_city);
                let to = get_node_by_name(&graph, &to_city);

                let Some(from_idx) = from else {
                    city_not_found(&mut stdout, &from_city)?;
                    continue;
                };
                let Some(to_idx) = to else {
                    city_not_found(&mut stdout, &to_city)?;
                    continue;
                };

                if add_edge(&mut graph, from_idx, to_idx, distance, &mut stderr)? == 0 {
                    stdout.write_all(b"Added route: ")?;
                    stdout.write_all(&from_city)?;
                    stdout.write_all(b" -> ")?;
                    stdout.write_all(&to_city)?;
                    writeln!(stdout, " (distance: {})", distance)?;
                } else {
                    stdout.write_all(b"Failed to add route\n")?;
                }
            }
            3 => {
                print_graph(&graph, &mut stdout)?;
            }
            4 => {
                stdout.write_all(b"Enter city name: ")?;
                let Some(input) = reader.fgets(MAX_INPUT)? else {
                    continue;
                };
                let city = c_string_line(input);
                if let Some(node) = get_node_by_name(&graph, &city) {
                    print_node(&graph, node, &mut stdout)?;
                } else {
                    city_not_found(&mut stdout, &city)?;
                }
            }
            5 => {
                stdout.write_all(b"Enter start city: ")?;
                let Some(start_input) = reader.fgets(MAX_INPUT)? else {
                    continue;
                };
                let start_city = c_string_line(start_input);

                stdout.write_all(b"Enter end city: ")?;
                let Some(end_input) = reader.fgets(MAX_INPUT)? else {
                    continue;
                };
                let end_city = c_string_line(end_input);

                let start = get_node_by_name(&graph, &start_city);
                let end = get_node_by_name(&graph, &end_city);

                let Some(start_idx) = start else {
                    city_not_found(&mut stdout, &start_city)?;
                    continue;
                };
                let Some(end_idx) = end else {
                    city_not_found(&mut stdout, &end_city)?;
                    continue;
                };

                if let Some(path) = find_shortest_path(&graph, start_idx, end_idx, &mut stderr)? {
                    stdout.write_all(b"Shortest path from ")?;
                    stdout.write_all(&start_city)?;
                    stdout.write_all(b" to ")?;
                    stdout.write_all(&end_city)?;
                    stdout.write_all(b":\n")?;
                    for (i, node) in path.iter().enumerate() {
                        write!(stdout, "  {}. ", i + 1)?;
                        stdout.write_all(&graph.nodes[*node].city_name)?;
                        stdout.write_all(b"\n")?;
                    }
                } else {
                    stdout.write_all(b"No path found\n")?;
                }
            }
            6 => {
                stdout.write_all(b"Enter start city for shallow copy: ")?;
                let Some(input) = reader.fgets(MAX_INPUT)? else {
                    continue;
                };
                let city = c_string_line(input);
                let Some(node) = get_node_by_name(&graph, &city) else {
                    city_not_found(&mut stdout, &city)?;
                    continue;
                };

                let copy = shallow_copy(&mut graph, node);
                stdout.write_all(b"Created shallow copy starting from ")?;
                stdout.write_all(&city)?;
                stdout.write_all(b"\n")?;
                stdout.write_all(b"Reference counts incremented for all reachable nodes\n")?;
                print_node(&graph, copy, &mut stdout)?;
            }
            7 => {
                stdout.write_all(b"Enter city name to delete: ")?;
                let Some(input) = reader.fgets(MAX_INPUT)? else {
                    continue;
                };
                let city = c_string_line(input);
                let Some(node) = get_node_by_name(&graph, &city) else {
                    city_not_found(&mut stdout, &city)?;
                    continue;
                };

                writeln!(stdout, "Current ref count: {}", graph.nodes[node].ref_count)?;
                delete_node(&mut graph, node);
                stdout.write_all(b"Decremented reference count for ")?;
                stdout.write_all(&city)?;
                stdout.write_all(b"\n")?;
                stdout.write_all(b"Note: Node will be freed when ref count reaches 0\n")?;
            }
            8 => {
                stdout.write_all(b"Freeing graph and exiting...\n")?;
                free_graph(&mut graph);
                return Ok(());
            }
            _ => {
                stdout.write_all(b"Invalid choice\n")?;
            }
        }
    }

    free_graph(&mut graph);
    Ok(())
}
