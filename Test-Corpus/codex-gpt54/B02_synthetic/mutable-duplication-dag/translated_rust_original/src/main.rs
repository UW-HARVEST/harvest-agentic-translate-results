use libc::{c_char, c_int};

const MAX_CITY_NAME: usize = 64;
const MAX_EDGES: usize = 10;
const MAX_NODES: usize = 100;
const MAX_INPUT: usize = 256;

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr().cast::<c_char>()
    };
}

unsafe extern "C" {
    static mut stdin: *mut libc::FILE;
    static mut stderr: *mut libc::FILE;
}

#[derive(Clone, Copy)]
struct Edge {
    destination: usize,
    distance: c_int,
}

impl Default for Edge {
    fn default() -> Self {
        Self {
            destination: usize::MAX,
            distance: 0,
        }
    }
}

struct Node {
    city_name: [c_char; MAX_CITY_NAME],
    ref_count: c_int,
    edges: [Edge; MAX_EDGES],
    edge_count: c_int,
}

impl Node {
    fn new(city_name: *const c_char) -> Self {
        let mut node = Self {
            city_name: [0; MAX_CITY_NAME],
            ref_count: 1,
            edges: [Edge::default(); MAX_EDGES],
            edge_count: 0,
        };

        unsafe {
            libc::strncpy(node.city_name.as_mut_ptr(), city_name, MAX_CITY_NAME - 1);
            node.city_name[MAX_CITY_NAME - 1] = 0;
        }

        node
    }
}

struct Graph {
    nodes: Vec<Node>,
}

#[derive(Clone, Copy)]
struct DijkstraNode {
    node: usize,
    distance: c_int,
    previous: Option<usize>,
    visited: c_int,
}

fn eprintln_failed_create_graph() {
    unsafe {
        libc::fprintf(stderr, cstr!("Failed to create graph\n"));
    }
}

fn create_graph() -> Option<Graph> {
    let mut nodes = Vec::new();
    if nodes.try_reserve_exact(MAX_NODES).is_err() {
        unsafe {
            libc::fprintf(stderr, cstr!("Error: Failed to allocate graph\n"));
        }
        return None;
    }

    Some(Graph { nodes })
}

fn add_node(graph: &mut Graph, city_name: *const c_char) -> Option<usize> {
    if city_name.is_null() {
        unsafe {
            libc::fprintf(stderr, cstr!("Error: NULL parameter in add_node\n"));
        }
        return None;
    }

    if graph.nodes.len() >= MAX_NODES {
        unsafe {
            libc::fprintf(
                stderr,
                cstr!("Error: Graph is full (max %d nodes)\n"),
                MAX_NODES as c_int,
            );
        }
        return None;
    }

    for node in &graph.nodes {
        unsafe {
            if libc::strcmp(node.city_name.as_ptr(), city_name) == 0 {
                libc::fprintf(
                    stderr,
                    cstr!("Error: Node '%s' already exists\n"),
                    city_name,
                );
                return None;
            }
        }
    }

    graph.nodes.push(Node::new(city_name));
    Some(graph.nodes.len() - 1)
}

fn add_edge(graph: &mut Graph, from: usize, to: usize, distance: c_int) -> c_int {
    if from >= graph.nodes.len() || to >= graph.nodes.len() {
        unsafe {
            libc::fprintf(stderr, cstr!("Error: NULL node in add_edge\n"));
        }
        return -1;
    }

    if graph.nodes[from].edge_count as usize >= MAX_EDGES {
        unsafe {
            libc::fprintf(
                stderr,
                cstr!("Error: Node '%s' has maximum edges\n"),
                graph.nodes[from].city_name.as_ptr(),
            );
        }
        return -1;
    }

    if distance < 0 {
        unsafe {
            libc::fprintf(stderr, cstr!("Error: Negative distance not allowed\n"));
        }
        return -1;
    }

    let edge_count = graph.nodes[from].edge_count as usize;
    for i in 0..edge_count {
        if graph.nodes[from].edges[i].destination == to {
            unsafe {
                libc::fprintf(stderr, cstr!("Error: Edge already exists\n"));
            }
            return -1;
        }
    }

    graph.nodes[from].edges[edge_count] = Edge {
        destination: to,
        distance,
    };
    graph.nodes[from].edge_count += 1;

    0
}

fn delete_node(graph: &mut Graph, node_idx: usize) {
    if node_idx >= graph.nodes.len() {
        return;
    }

    graph.nodes[node_idx].ref_count -= 1;
}

fn increment_refs_recursive(
    graph: &mut Graph,
    node_idx: usize,
    visited: &mut [usize; MAX_NODES],
    visited_count: &mut usize,
) {
    for &visited_idx in visited.iter().take(*visited_count) {
        if visited_idx == node_idx {
            return;
        }
    }

    if *visited_count < MAX_NODES {
        visited[*visited_count] = node_idx;
        *visited_count += 1;
    }

    graph.nodes[node_idx].ref_count += 1;

    let edge_count = graph.nodes[node_idx].edge_count as usize;
    let destinations: Vec<usize> = graph.nodes[node_idx]
        .edges
        .iter()
        .take(edge_count)
        .map(|edge| edge.destination)
        .collect();

    for destination in destinations {
        increment_refs_recursive(graph, destination, visited, visited_count);
    }
}

fn shallow_copy(graph: &mut Graph, start: usize) -> Option<usize> {
    if start >= graph.nodes.len() {
        unsafe {
            libc::fprintf(stderr, cstr!("Error: NULL node in shallow_copy\n"));
        }
        return None;
    }

    let mut visited = [usize::MAX; MAX_NODES];
    let mut visited_count = 0usize;
    increment_refs_recursive(graph, start, &mut visited, &mut visited_count);
    Some(start)
}

fn find_shortest_path(graph: &Graph, start: usize, end: usize, path_length: &mut c_int) -> Option<Vec<usize>> {
    if start >= graph.nodes.len() || end >= graph.nodes.len() {
        unsafe {
            libc::fprintf(
                stderr,
                cstr!("Error: NULL parameter in find_shortest_path\n"),
            );
        }
        return None;
    }

    let mut state = [DijkstraNode {
        node: usize::MAX,
        distance: 0,
        previous: None,
        visited: 0,
    }; MAX_NODES];
    let mut state_count = 0usize;

    state[state_count] = DijkstraNode {
        node: start,
        distance: 0,
        previous: None,
        visited: 0,
    };
    state_count += 1;

    let mut current = Some(start);

    while let Some(current_node) = current {
        let mut current_idx: c_int = -1;
        for i in 0..state_count {
            if state[i].node == current_node {
                current_idx = i as c_int;
                break;
            }
        }

        if current_idx == -1 {
            break;
        }

        let current_idx_usize = current_idx as usize;
        state[current_idx_usize].visited = 1;

        if current_node == end {
            break;
        }

        let edge_count = graph.nodes[current_node].edge_count as usize;
        for i in 0..edge_count {
            let neighbor = graph.nodes[current_node].edges[i].destination;
            let new_distance = state[current_idx_usize]
                .distance
                .wrapping_add(graph.nodes[current_node].edges[i].distance);

            let mut neighbor_idx: c_int = -1;
            for j in 0..state_count {
                if state[j].node == neighbor {
                    neighbor_idx = j as c_int;
                    break;
                }
            }

            if neighbor_idx == -1 && state_count < MAX_NODES {
                neighbor_idx = state_count as c_int;
                state[state_count] = DijkstraNode {
                    node: neighbor,
                    distance: c_int::MAX,
                    previous: None,
                    visited: 0,
                };
                state_count += 1;
            }

            if neighbor_idx != -1 {
                let neighbor_idx_usize = neighbor_idx as usize;
                if new_distance < state[neighbor_idx_usize].distance {
                    state[neighbor_idx_usize].distance = new_distance;
                    state[neighbor_idx_usize].previous = Some(current_node);
                }
            }
        }

        let mut min_distance = c_int::MAX;
        current = None;
        for entry in state.iter().take(state_count) {
            if entry.visited == 0 && entry.distance < min_distance {
                min_distance = entry.distance;
                current = Some(entry.node);
            }
        }
    }

    let mut end_idx: c_int = -1;
    for i in 0..state_count {
        if state[i].node == end {
            end_idx = i as c_int;
            break;
        }
    }

    if end_idx == -1 || state[end_idx as usize].distance == c_int::MAX {
        unsafe {
            libc::fprintf(stderr, cstr!("No path found\n"));
        }
        *path_length = 0;
        return None;
    }

    let mut path = [usize::MAX; MAX_NODES];
    let mut count = 0usize;
    let mut current_node = Some(end);

    while let Some(node_idx) = current_node {
        path[count] = node_idx;
        count += 1;

        let mut current_state_idx: c_int = -1;
        for i in 0..state_count {
            if state[i].node == node_idx {
                current_state_idx = i as c_int;
                break;
            }
        }

        if current_state_idx == -1 {
            break;
        }

        current_node = state[current_state_idx as usize].previous;
    }

    let mut result = Vec::new();
    if result.try_reserve_exact(count).is_err() {
        unsafe {
            libc::fprintf(stderr, cstr!("Error: Failed to allocate path\n"));
        }
        *path_length = 0;
        return None;
    }

    for i in 0..count {
        result.push(path[count - 1 - i]);
    }

    *path_length = count as c_int;
    Some(result)
}

fn get_node_by_name(graph: &Graph, city_name: *const c_char) -> Option<usize> {
    if city_name.is_null() {
        return None;
    }

    for (idx, node) in graph.nodes.iter().enumerate() {
        unsafe {
            if libc::strcmp(node.city_name.as_ptr(), city_name) == 0 {
                return Some(idx);
            }
        }
    }

    None
}

fn print_node(graph: &Graph, node_idx: Option<usize>) {
    match node_idx {
        None => unsafe {
            libc::printf(cstr!("NULL node\n"));
        },
        Some(idx) => {
            let node = &graph.nodes[idx];
            unsafe {
                libc::printf(
                    cstr!("City: %s (ref_count: %d)\n"),
                    node.city_name.as_ptr(),
                    node.ref_count,
                );
                libc::printf(cstr!("  Edges:\n"));
                for i in 0..(node.edge_count as usize) {
                    let dest = graph.nodes[node.edges[i].destination].city_name.as_ptr();
                    libc::printf(
                        cstr!("    -> %s (distance: %d)\n"),
                        dest,
                        node.edges[i].distance,
                    );
                }
            }
        }
    }
}

fn print_graph(graph: Option<&Graph>) {
    match graph {
        None => unsafe {
            libc::printf(cstr!("NULL graph\n"));
        },
        Some(graph) => unsafe {
            libc::printf(
                cstr!("Graph with %d nodes:\n"),
                graph.nodes.len() as c_int,
            );
            for idx in 0..graph.nodes.len() {
                print_node(graph, Some(idx));
            }
        },
    }
}

fn free_graph(graph: &mut Graph) {
    for idx in 0..graph.nodes.len() {
        delete_node(graph, idx);
    }
}

fn print_menu() {
    unsafe {
        libc::printf(cstr!("\n=== DAG City Route Manager ===\n"));
        libc::printf(cstr!("1. Add city (node)\n"));
        libc::printf(cstr!("2. Add route (edge)\n"));
        libc::printf(cstr!("3. Show all cities\n"));
        libc::printf(cstr!("4. Show city details\n"));
        libc::printf(cstr!("5. Find shortest path\n"));
        libc::printf(cstr!("6. Make shallow copy of subsection\n"));
        libc::printf(cstr!("7. Delete node\n"));
        libc::printf(cstr!("8. Exit\n"));
        libc::printf(cstr!("Choice: "));
    }
}

fn fgets_into(buffer: &mut [c_char; MAX_INPUT]) -> bool {
    buffer.fill(0);
    unsafe { !libc::fgets(buffer.as_mut_ptr(), MAX_INPUT as c_int, stdin).is_null() }
}

fn truncate_at_newline(buffer: &mut [c_char]) {
    for ch in buffer.iter_mut() {
        if *ch == 0 {
            break;
        }
        if *ch == b'\n' as c_char {
            *ch = 0;
            break;
        }
    }
}

fn main() {
    let mut graph = match create_graph() {
        Some(graph) => graph,
        None => {
            eprintln_failed_create_graph();
            std::process::exit(1);
        }
    };

    let mut input = [0 as c_char; MAX_INPUT];
    let mut choice: c_int = 0;

    unsafe {
        libc::printf(cstr!("City Route Management System\n"));
        libc::printf(cstr!("Commands are read from stdin\n"));
    }

    loop {
        print_menu();

        if !fgets_into(&mut input) {
            break;
        }

        let scan_result =
            unsafe { libc::sscanf(input.as_ptr(), cstr!("%d"), &mut choice as *mut c_int) };
        if scan_result != 1 {
            unsafe {
                libc::printf(cstr!("Invalid input\n"));
            }
            continue;
        }

        match choice {
            1 => {
                unsafe {
                    libc::printf(cstr!("Enter city name: "));
                }
                if !fgets_into(&mut input) {
                    break;
                }

                truncate_at_newline(&mut input);

                if add_node(&mut graph, input.as_ptr()).is_some() {
                    unsafe {
                        libc::printf(cstr!("Added city: %s\n"), input.as_ptr());
                    }
                } else {
                    unsafe {
                        libc::printf(cstr!("Failed to add city\n"));
                    }
                }
            }
            2 => {
                let mut from_city = [0 as c_char; MAX_INPUT];
                let mut to_city = [0 as c_char; MAX_INPUT];
                let mut distance: c_int = 0;

                unsafe {
                    libc::printf(cstr!("Enter from city: "));
                }
                if !fgets_into(&mut from_city) {
                    break;
                }
                truncate_at_newline(&mut from_city);

                unsafe {
                    libc::printf(cstr!("Enter to city: "));
                }
                if !fgets_into(&mut to_city) {
                    break;
                }
                truncate_at_newline(&mut to_city);

                unsafe {
                    libc::printf(cstr!("Enter distance: "));
                }
                if !fgets_into(&mut input) {
                    break;
                }
                if unsafe {
                    libc::sscanf(input.as_ptr(), cstr!("%d"), &mut distance as *mut c_int)
                } != 1
                {
                    unsafe {
                        libc::printf(cstr!("Invalid distance\n"));
                    }
                    continue;
                }

                let from = get_node_by_name(&graph, from_city.as_ptr());
                let to = get_node_by_name(&graph, to_city.as_ptr());

                if from.is_none() {
                    unsafe {
                        libc::printf(cstr!("City '%s' not found\n"), from_city.as_ptr());
                    }
                    continue;
                }
                if to.is_none() {
                    unsafe {
                        libc::printf(cstr!("City '%s' not found\n"), to_city.as_ptr());
                    }
                    continue;
                }

                if add_edge(&mut graph, from.unwrap(), to.unwrap(), distance) == 0 {
                    unsafe {
                        libc::printf(
                            cstr!("Added route: %s -> %s (distance: %d)\n"),
                            from_city.as_ptr(),
                            to_city.as_ptr(),
                            distance,
                        );
                    }
                } else {
                    unsafe {
                        libc::printf(cstr!("Failed to add route\n"));
                    }
                }
            }
            3 => {
                print_graph(Some(&graph));
            }
            4 => {
                unsafe {
                    libc::printf(cstr!("Enter city name: "));
                }
                if !fgets_into(&mut input) {
                    break;
                }
                truncate_at_newline(&mut input);

                let node = get_node_by_name(&graph, input.as_ptr());
                if let Some(node_idx) = node {
                    print_node(&graph, Some(node_idx));
                } else {
                    unsafe {
                        libc::printf(cstr!("City '%s' not found\n"), input.as_ptr());
                    }
                }
            }
            5 => {
                let mut start_city = [0 as c_char; MAX_INPUT];
                let mut end_city = [0 as c_char; MAX_INPUT];

                unsafe {
                    libc::printf(cstr!("Enter start city: "));
                }
                if !fgets_into(&mut start_city) {
                    break;
                }
                truncate_at_newline(&mut start_city);

                unsafe {
                    libc::printf(cstr!("Enter end city: "));
                }
                if !fgets_into(&mut end_city) {
                    break;
                }
                truncate_at_newline(&mut end_city);

                let start = get_node_by_name(&graph, start_city.as_ptr());
                let end = get_node_by_name(&graph, end_city.as_ptr());

                if start.is_none() {
                    unsafe {
                        libc::printf(cstr!("City '%s' not found\n"), start_city.as_ptr());
                    }
                    continue;
                }
                if end.is_none() {
                    unsafe {
                        libc::printf(cstr!("City '%s' not found\n"), end_city.as_ptr());
                    }
                    continue;
                }

                let mut path_length: c_int = 0;
                let path = find_shortest_path(&graph, start.unwrap(), end.unwrap(), &mut path_length);

                if let Some(path) = path {
                    unsafe {
                        libc::printf(
                            cstr!("Shortest path from %s to %s:\n"),
                            start_city.as_ptr(),
                            end_city.as_ptr(),
                        );
                        for (i, node_idx) in path.iter().enumerate() {
                            libc::printf(
                                cstr!("  %d. %s\n"),
                                (i + 1) as c_int,
                                graph.nodes[*node_idx].city_name.as_ptr(),
                            );
                        }
                    }
                } else {
                    unsafe {
                        libc::printf(cstr!("No path found\n"));
                    }
                }
            }
            6 => {
                unsafe {
                    libc::printf(cstr!("Enter start city for shallow copy: "));
                }
                if !fgets_into(&mut input) {
                    break;
                }
                truncate_at_newline(&mut input);

                let node = get_node_by_name(&graph, input.as_ptr());
                if node.is_none() {
                    unsafe {
                        libc::printf(cstr!("City '%s' not found\n"), input.as_ptr());
                    }
                    continue;
                }

                let copy = shallow_copy(&mut graph, node.unwrap());
                if let Some(copy_idx) = copy {
                    unsafe {
                        libc::printf(cstr!("Created shallow copy starting from %s\n"), input.as_ptr());
                        libc::printf(cstr!("Reference counts incremented for all reachable nodes\n"));
                    }
                    print_node(&graph, Some(copy_idx));
                } else {
                    unsafe {
                        libc::printf(cstr!("Failed to create shallow copy\n"));
                    }
                }
            }
            7 => {
                unsafe {
                    libc::printf(cstr!("Enter city name to delete: "));
                }
                if !fgets_into(&mut input) {
                    break;
                }
                truncate_at_newline(&mut input);

                let node = get_node_by_name(&graph, input.as_ptr());
                if node.is_none() {
                    unsafe {
                        libc::printf(cstr!("City '%s' not found\n"), input.as_ptr());
                    }
                    continue;
                }

                let node_idx = node.unwrap();
                unsafe {
                    libc::printf(cstr!("Current ref count: %d\n"), graph.nodes[node_idx].ref_count);
                }
                delete_node(&mut graph, node_idx);
                unsafe {
                    libc::printf(cstr!("Decremented reference count for %s\n"), input.as_ptr());
                    libc::printf(cstr!("Note: Node will be freed when ref count reaches 0\n"));
                }
            }
            8 => {
                unsafe {
                    libc::printf(cstr!("Freeing graph and exiting...\n"));
                }
                free_graph(&mut graph);
                return;
            }
            _ => unsafe {
                libc::printf(cstr!("Invalid choice\n"));
            },
        }
    }

    free_graph(&mut graph);
}
