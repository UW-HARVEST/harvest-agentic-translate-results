//! Translation of `c_src/src/main.c`.

mod cio;
mod dag_lib;

use cio::{sscanf_d, strip_newline, In, Out};
use dag_lib::Graph;

const MAX_INPUT: usize = 256;

fn print_menu(out: &mut Out) {
    out.s("\n=== DAG City Route Manager ===\n");
    out.s("1. Add city (node)\n");
    out.s("2. Add route (edge)\n");
    out.s("3. Show all cities\n");
    out.s("4. Show city details\n");
    out.s("5. Find shortest path\n");
    out.s("6. Make shallow copy of subsection\n");
    out.s("7. Delete node\n");
    out.s("8. Exit\n");
    out.s("Choice: ");
}

fn main() {
    let mut out = Out::new();
    let mut stdin = In::new();

    let mut graph = Graph::create_graph();

    out.s("City Route Management System\n");
    out.s("Commands are read from stdin\n");

    loop {
        print_menu(&mut out);

        let input = match stdin.fgets(MAX_INPUT) {
            Some(line) => line,
            None => break,
        };

        let choice = match sscanf_d(&input) {
            Some(value) => value,
            None => {
                out.s("Invalid input\n");
                continue;
            }
        };

        match choice {
            1 => 'case: {
                // Add city
                out.s("Enter city name: ");
                let input = match stdin.fgets(MAX_INPUT) {
                    Some(line) => line,
                    None => break 'case,
                };

                // Remove newline
                let name = strip_newline(&input).to_vec();

                match graph.add_node(&name) {
                    Some(_) => {
                        out.s("Added city: ");
                        out.write(&name);
                        out.s("\n");
                    }
                    None => {
                        out.s("Failed to add city\n");
                    }
                }
            }

            2 => 'case: {
                // Add route
                out.s("Enter from city: ");
                let from_buf = match stdin.fgets(MAX_INPUT) {
                    Some(line) => line,
                    None => break 'case,
                };
                let from_city = strip_newline(&from_buf).to_vec();

                out.s("Enter to city: ");
                let to_buf = match stdin.fgets(MAX_INPUT) {
                    Some(line) => line,
                    None => break 'case,
                };
                let to_city = strip_newline(&to_buf).to_vec();

                out.s("Enter distance: ");
                let input = match stdin.fgets(MAX_INPUT) {
                    Some(line) => line,
                    None => break 'case,
                };
                let distance = match sscanf_d(&input) {
                    Some(value) => value,
                    None => {
                        out.s("Invalid distance\n");
                        break 'case;
                    }
                };

                let from = graph.get_node_by_name(&from_city);
                let to = graph.get_node_by_name(&to_city);

                let from = match from {
                    Some(r) => r,
                    None => {
                        out.s("City '");
                        out.write(&from_city);
                        out.s("' not found\n");
                        break 'case;
                    }
                };
                let to = match to {
                    Some(r) => r,
                    None => {
                        out.s("City '");
                        out.write(&to_city);
                        out.s("' not found\n");
                        break 'case;
                    }
                };

                if graph.add_edge(from, to, distance) == 0 {
                    out.s("Added route: ");
                    out.write(&from_city);
                    out.s(" -> ");
                    out.write(&to_city);
                    out.write(format!(" (distance: {})\n", distance).as_bytes());
                } else {
                    out.s("Failed to add route\n");
                }
            }

            3 => {
                // Show all cities
                graph.print_graph(&mut out);
            }

            4 => 'case: {
                // Show city details
                out.s("Enter city name: ");
                let input = match stdin.fgets(MAX_INPUT) {
                    Some(line) => line,
                    None => break 'case,
                };
                let name = strip_newline(&input).to_vec();

                match graph.get_node_by_name(&name) {
                    Some(r) => graph.print_node(&mut out, r),
                    None => {
                        out.s("City '");
                        out.write(&name);
                        out.s("' not found\n");
                    }
                }
            }

            5 => 'case: {
                // Find shortest path
                out.s("Enter start city: ");
                let start_buf = match stdin.fgets(MAX_INPUT) {
                    Some(line) => line,
                    None => break 'case,
                };
                let start_city = strip_newline(&start_buf).to_vec();

                out.s("Enter end city: ");
                let end_buf = match stdin.fgets(MAX_INPUT) {
                    Some(line) => line,
                    None => break 'case,
                };
                let end_city = strip_newline(&end_buf).to_vec();

                let start = graph.get_node_by_name(&start_city);
                let end = graph.get_node_by_name(&end_city);

                let start = match start {
                    Some(r) => r,
                    None => {
                        out.s("City '");
                        out.write(&start_city);
                        out.s("' not found\n");
                        break 'case;
                    }
                };
                let end = match end {
                    Some(r) => r,
                    None => {
                        out.s("City '");
                        out.write(&end_city);
                        out.s("' not found\n");
                        break 'case;
                    }
                };

                // stdout is fully buffered in C while stderr is not, so nothing
                // needs to be flushed here.
                let (path, path_length) = graph.find_shortest_path(start, end);

                match path {
                    Some(path) => {
                        out.s("Shortest path from ");
                        out.write(&start_city);
                        out.s(" to ");
                        out.write(&end_city);
                        out.s(":\n");
                        for i in 0..path_length as usize {
                            out.write(format!("  {}. ", i + 1).as_bytes());
                            out.write(&graph.node(path[i]).city_name);
                            out.s("\n");
                        }
                    }
                    None => {
                        out.s("No path found\n");
                    }
                }
            }

            6 => 'case: {
                // Make shallow copy
                out.s("Enter start city for shallow copy: ");
                let input = match stdin.fgets(MAX_INPUT) {
                    Some(line) => line,
                    None => break 'case,
                };
                let name = strip_newline(&input).to_vec();

                let node = match graph.get_node_by_name(&name) {
                    Some(r) => r,
                    None => {
                        out.s("City '");
                        out.write(&name);
                        out.s("' not found\n");
                        break 'case;
                    }
                };

                match graph.shallow_copy(node) {
                    Some(copy) => {
                        out.s("Created shallow copy starting from ");
                        out.write(&name);
                        out.s("\n");
                        out.s("Reference counts incremented for all reachable nodes\n");
                        graph.print_node(&mut out, copy);
                    }
                    None => {
                        out.s("Failed to create shallow copy\n");
                    }
                }
            }

            7 => 'case: {
                // Delete node
                out.s("Enter city name to delete: ");
                let input = match stdin.fgets(MAX_INPUT) {
                    Some(line) => line,
                    None => break 'case,
                };
                let name = strip_newline(&input).to_vec();

                let node = match graph.get_node_by_name(&name) {
                    Some(r) => r,
                    None => {
                        out.s("City '");
                        out.write(&name);
                        out.s("' not found\n");
                        break 'case;
                    }
                };

                out.write(
                    format!("Current ref count: {}\n", graph.node(node).ref_count).as_bytes(),
                );
                graph.delete_node(node);
                out.s("Decremented reference count for ");
                out.write(&name);
                out.s("\n");
                out.s("Note: Node will be freed when ref count reaches 0\n");
            }

            8 => {
                // Exit
                out.s("Freeing graph and exiting...\n");
                graph.free_graph();
                out.flush();
                std::process::exit(0);
            }

            _ => {
                out.s("Invalid choice\n");
            }
        }
    }

    graph.free_graph();
    out.flush();
    std::process::exit(0);
}
