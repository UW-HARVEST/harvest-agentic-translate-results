//! Translation of `c_src/src/main.c`.

mod cio;
mod dag_lib;

use cio::{err, sscanf_int, strip_newline, CStdin, Out};
use dag_lib::*;

const MAX_INPUT: usize = 256;

fn print_menu(out: &mut Out) {
    out.write(b"\n=== DAG City Route Manager ===\n");
    out.write(b"1. Add city (node)\n");
    out.write(b"2. Add route (edge)\n");
    out.write(b"3. Show all cities\n");
    out.write(b"4. Show city details\n");
    out.write(b"5. Find shortest path\n");
    out.write(b"6. Make shallow copy of subsection\n");
    out.write(b"7. Delete node\n");
    out.write(b"8. Exit\n");
    out.write(b"Choice: ");
}

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let mut out = Out::new();
    let mut stdin = CStdin::new();
    let mut arena = Arena::new();

    let graph = create_graph();
    let mut graph = match graph {
        Some(g) => g,
        None => {
            err(b"Failed to create graph\n");
            return 1;
        }
    };

    out.write(b"City Route Management System\n");
    out.write(b"Commands are read from stdin\n");

    loop {
        print_menu(&mut out);

        let input = match stdin.fgets(MAX_INPUT) {
            Some(b) => b,
            None => break,
        };

        let choice = match sscanf_int(&input) {
            Some(c) => c,
            None => {
                out.write(b"Invalid input\n");
                continue;
            }
        };

        // Each arm below mirrors one `case` of the C `switch`.  A `break` inside
        // a case leaves the switch only, so the outer `while (1)` loop keeps
        // going -- reproduced here with a labeled block.
        'sw: {
            match choice {
                1 => {
                    // Add city
                    out.write(b"Enter city name: ");
                    let input = match stdin.fgets(MAX_INPUT) {
                        Some(b) => b,
                        None => break 'sw,
                    };

                    // Remove newline
                    let name = strip_newline(&input).to_vec();

                    let node = add_node(&mut arena, Some(&mut graph), Some(&name));
                    if node.is_some() {
                        let mut line = Vec::new();
                        line.extend_from_slice(b"Added city: ");
                        line.extend_from_slice(&name);
                        line.extend_from_slice(b"\n");
                        out.write(&line);
                    } else {
                        out.write(b"Failed to add city\n");
                    }
                }

                2 => {
                    // Add route
                    out.write(b"Enter from city: ");
                    let from_buf = match stdin.fgets(MAX_INPUT) {
                        Some(b) => b,
                        None => break 'sw,
                    };
                    let from_city = strip_newline(&from_buf).to_vec();

                    out.write(b"Enter to city: ");
                    let to_buf = match stdin.fgets(MAX_INPUT) {
                        Some(b) => b,
                        None => break 'sw,
                    };
                    let to_city = strip_newline(&to_buf).to_vec();

                    out.write(b"Enter distance: ");
                    let input = match stdin.fgets(MAX_INPUT) {
                        Some(b) => b,
                        None => break 'sw,
                    };
                    let distance = match sscanf_int(&input) {
                        Some(d) => d,
                        None => {
                            out.write(b"Invalid distance\n");
                            break 'sw;
                        }
                    };

                    let from = get_node_by_name(&arena, Some(&graph), Some(&from_city));
                    let to = get_node_by_name(&arena, Some(&graph), Some(&to_city));

                    if from.is_none() {
                        let mut line = Vec::new();
                        line.extend_from_slice(b"City '");
                        line.extend_from_slice(&from_city);
                        line.extend_from_slice(b"' not found\n");
                        out.write(&line);
                        break 'sw;
                    }
                    if to.is_none() {
                        let mut line = Vec::new();
                        line.extend_from_slice(b"City '");
                        line.extend_from_slice(&to_city);
                        line.extend_from_slice(b"' not found\n");
                        out.write(&line);
                        break 'sw;
                    }

                    if add_edge(&mut arena, from, to, distance) == 0 {
                        let mut line = Vec::new();
                        line.extend_from_slice(b"Added route: ");
                        line.extend_from_slice(&from_city);
                        line.extend_from_slice(b" -> ");
                        line.extend_from_slice(&to_city);
                        line.extend_from_slice(
                            format!(" (distance: {})\n", distance).as_bytes(),
                        );
                        out.write(&line);
                    } else {
                        out.write(b"Failed to add route\n");
                    }
                }

                3 => {
                    // Show all cities
                    print_graph(&mut out, &arena, Some(&graph));
                }

                4 => {
                    // Show city details
                    out.write(b"Enter city name: ");
                    let input = match stdin.fgets(MAX_INPUT) {
                        Some(b) => b,
                        None => break 'sw,
                    };
                    let name = strip_newline(&input).to_vec();

                    let node = get_node_by_name(&arena, Some(&graph), Some(&name));
                    if node.is_some() {
                        print_node(&mut out, &arena, node);
                    } else {
                        let mut line = Vec::new();
                        line.extend_from_slice(b"City '");
                        line.extend_from_slice(&name);
                        line.extend_from_slice(b"' not found\n");
                        out.write(&line);
                    }
                }

                5 => {
                    // Find shortest path
                    out.write(b"Enter start city: ");
                    let start_buf = match stdin.fgets(MAX_INPUT) {
                        Some(b) => b,
                        None => break 'sw,
                    };
                    let start_city = strip_newline(&start_buf).to_vec();

                    out.write(b"Enter end city: ");
                    let end_buf = match stdin.fgets(MAX_INPUT) {
                        Some(b) => b,
                        None => break 'sw,
                    };
                    let end_city = strip_newline(&end_buf).to_vec();

                    let start = get_node_by_name(&arena, Some(&graph), Some(&start_city));
                    let end = get_node_by_name(&arena, Some(&graph), Some(&end_city));

                    if start.is_none() {
                        let mut line = Vec::new();
                        line.extend_from_slice(b"City '");
                        line.extend_from_slice(&start_city);
                        line.extend_from_slice(b"' not found\n");
                        out.write(&line);
                        break 'sw;
                    }
                    if end.is_none() {
                        let mut line = Vec::new();
                        line.extend_from_slice(b"City '");
                        line.extend_from_slice(&end_city);
                        line.extend_from_slice(b"' not found\n");
                        out.write(&line);
                        break 'sw;
                    }

                    let mut path_length: i32 = 0;
                    let path = find_shortest_path(&arena, start, end, &mut path_length);

                    if let Some(path) = path {
                        let mut line = Vec::new();
                        line.extend_from_slice(b"Shortest path from ");
                        line.extend_from_slice(&start_city);
                        line.extend_from_slice(b" to ");
                        line.extend_from_slice(&end_city);
                        line.extend_from_slice(b":\n");
                        out.write(&line);
                        for i in 0..path_length {
                            let mut line = Vec::new();
                            line.extend_from_slice(format!("  {}. ", i + 1).as_bytes());
                            line.extend_from_slice(arena.get(path[i as usize]).name());
                            line.extend_from_slice(b"\n");
                            out.write(&line);
                        }
                    } else {
                        out.write(b"No path found\n");
                    }
                }

                6 => {
                    // Make shallow copy
                    out.write(b"Enter start city for shallow copy: ");
                    let input = match stdin.fgets(MAX_INPUT) {
                        Some(b) => b,
                        None => break 'sw,
                    };
                    let name = strip_newline(&input).to_vec();

                    let node = get_node_by_name(&arena, Some(&graph), Some(&name));
                    if node.is_none() {
                        let mut line = Vec::new();
                        line.extend_from_slice(b"City '");
                        line.extend_from_slice(&name);
                        line.extend_from_slice(b"' not found\n");
                        out.write(&line);
                        break 'sw;
                    }

                    let copy = shallow_copy(&mut arena, node);
                    if copy.is_some() {
                        let mut line = Vec::new();
                        line.extend_from_slice(b"Created shallow copy starting from ");
                        line.extend_from_slice(&name);
                        line.extend_from_slice(b"\n");
                        out.write(&line);
                        out.write(b"Reference counts incremented for all reachable nodes\n");
                        print_node(&mut out, &arena, copy);
                    } else {
                        out.write(b"Failed to create shallow copy\n");
                    }
                }

                7 => {
                    // Delete node
                    out.write(b"Enter city name to delete: ");
                    let input = match stdin.fgets(MAX_INPUT) {
                        Some(b) => b,
                        None => break 'sw,
                    };
                    let name = strip_newline(&input).to_vec();

                    let node = get_node_by_name(&arena, Some(&graph), Some(&name));
                    if node.is_none() {
                        let mut line = Vec::new();
                        line.extend_from_slice(b"City '");
                        line.extend_from_slice(&name);
                        line.extend_from_slice(b"' not found\n");
                        out.write(&line);
                        break 'sw;
                    }

                    out.write(
                        format!(
                            "Current ref count: {}\n",
                            arena.get(node.unwrap()).ref_count
                        )
                        .as_bytes(),
                    );
                    delete_node(&mut arena, node);
                    let mut line = Vec::new();
                    line.extend_from_slice(b"Decremented reference count for ");
                    line.extend_from_slice(&name);
                    line.extend_from_slice(b"\n");
                    out.write(&line);
                    out.write(b"Note: Node will be freed when ref count reaches 0\n");
                }

                8 => {
                    // Exit
                    out.write(b"Freeing graph and exiting...\n");
                    free_graph(&mut arena, Some(graph));
                    out.flush();
                    return 0;
                }

                _ => {
                    out.write(b"Invalid choice\n");
                }
            }
        }
    }

    free_graph(&mut arena, Some(graph));
    out.flush();
    0
}
