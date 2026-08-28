//! Translation of `c_src/src/main.c`.

mod cio;
mod dag_lib;

use cio::{chomp, cstr, sscanf_int, COut, CStdin};
use dag_lib::{
    add_edge, add_node, create_graph, delete_node, find_shortest_path, free_graph,
    get_node_by_name, print_graph, print_node, shallow_copy, Arena,
};

const MAX_INPUT: usize = 256;

fn print_menu(out: &mut COut) {
    out.put(b"\n=== DAG City Route Manager ===\n");
    out.put(b"1. Add city (node)\n");
    out.put(b"2. Add route (edge)\n");
    out.put(b"3. Show all cities\n");
    out.put(b"4. Show city details\n");
    out.put(b"5. Find shortest path\n");
    out.put(b"6. Make shallow copy of subsection\n");
    out.put(b"7. Delete node\n");
    out.put(b"8. Exit\n");
    out.put(b"Choice: ");
}

fn main() {
    let mut out = COut::new();
    let mut inp = CStdin::new();
    let mut arena = Arena::new();

    let graph = create_graph();
    let mut graph = match graph {
        Some(g) => g,
        None => {
            cio::eput(b"Failed to create graph\n");
            out.flush();
            std::process::exit(1);
        }
    };

    out.put(b"City Route Management System\n");
    out.put(b"Commands are read from stdin\n");

    loop {
        print_menu(&mut out);

        let line = match inp.fgets(MAX_INPUT) {
            Some(l) => l,
            None => break,
        };

        let choice = match sscanf_int(cstr(&line)) {
            Some(c) => c,
            None => {
                out.put(b"Invalid input\n");
                continue;
            }
        };

        match choice {
            1 => {
                // Add city
                'case1: {
                    out.put(b"Enter city name: ");
                    let line = match inp.fgets(MAX_INPUT) {
                        Some(l) => l,
                        None => break 'case1,
                    };

                    // Remove newline
                    let input = chomp(&line);

                    let node = add_node(&mut arena, &mut graph, &input);
                    if node.is_some() {
                        out.put(b"Added city: ");
                        out.put(&input);
                        out.put(b"\n");
                    } else {
                        out.put(b"Failed to add city\n");
                    }
                }
            }

            2 => {
                // Add route
                'case2: {
                    out.put(b"Enter from city: ");
                    let from_city = match inp.fgets(MAX_INPUT) {
                        Some(l) => l,
                        None => break 'case2,
                    };
                    let from_city = chomp(&from_city);

                    out.put(b"Enter to city: ");
                    let to_city = match inp.fgets(MAX_INPUT) {
                        Some(l) => l,
                        None => break 'case2,
                    };
                    let to_city = chomp(&to_city);

                    out.put(b"Enter distance: ");
                    let line = match inp.fgets(MAX_INPUT) {
                        Some(l) => l,
                        None => break 'case2,
                    };
                    let distance = match sscanf_int(cstr(&line)) {
                        Some(d) => d,
                        None => {
                            out.put(b"Invalid distance\n");
                            break 'case2;
                        }
                    };

                    let from = get_node_by_name(&arena, &graph, &from_city);
                    let to = get_node_by_name(&arena, &graph, &to_city);

                    let from = match from {
                        Some(f) => f,
                        None => {
                            out.put(b"City '");
                            out.put(&from_city);
                            out.put(b"' not found\n");
                            break 'case2;
                        }
                    };
                    let to = match to {
                        Some(t) => t,
                        None => {
                            out.put(b"City '");
                            out.put(&to_city);
                            out.put(b"' not found\n");
                            break 'case2;
                        }
                    };

                    if add_edge(&mut arena, from, to, distance) == 0 {
                        out.put(b"Added route: ");
                        out.put(&from_city);
                        out.put(b" -> ");
                        out.put(&to_city);
                        out.put(format!(" (distance: {})\n", distance).as_bytes());
                    } else {
                        out.put(b"Failed to add route\n");
                    }
                }
            }

            3 => {
                // Show all cities
                print_graph(&mut out, &arena, &graph);
            }

            4 => {
                // Show city details
                'case4: {
                    out.put(b"Enter city name: ");
                    let line = match inp.fgets(MAX_INPUT) {
                        Some(l) => l,
                        None => break 'case4,
                    };
                    let input = chomp(&line);

                    match get_node_by_name(&arena, &graph, &input) {
                        Some(node) => print_node(&mut out, &arena, node),
                        None => {
                            out.put(b"City '");
                            out.put(&input);
                            out.put(b"' not found\n");
                        }
                    }
                }
            }

            5 => {
                // Find shortest path
                'case5: {
                    out.put(b"Enter start city: ");
                    let start_city = match inp.fgets(MAX_INPUT) {
                        Some(l) => l,
                        None => break 'case5,
                    };
                    let start_city = chomp(&start_city);

                    out.put(b"Enter end city: ");
                    let end_city = match inp.fgets(MAX_INPUT) {
                        Some(l) => l,
                        None => break 'case5,
                    };
                    let end_city = chomp(&end_city);

                    let start = get_node_by_name(&arena, &graph, &start_city);
                    let end = get_node_by_name(&arena, &graph, &end_city);

                    let start = match start {
                        Some(s) => s,
                        None => {
                            out.put(b"City '");
                            out.put(&start_city);
                            out.put(b"' not found\n");
                            break 'case5;
                        }
                    };
                    let end = match end {
                        Some(e) => e,
                        None => {
                            out.put(b"City '");
                            out.put(&end_city);
                            out.put(b"' not found\n");
                            break 'case5;
                        }
                    };

                    let mut path_length: i32 = 0;
                    let path = find_shortest_path(&arena, start, end, &mut path_length);

                    match path {
                        Some(path) => {
                            out.put(b"Shortest path from ");
                            out.put(&start_city);
                            out.put(b" to ");
                            out.put(&end_city);
                            out.put(b":\n");
                            for i in 0..path_length as usize {
                                out.put(format!("  {}. ", i + 1).as_bytes());
                                out.put(arena.nodes[path[i]].name());
                                out.put(b"\n");
                            }
                            // free(path)
                        }
                        None => {
                            out.put(b"No path found\n");
                        }
                    }
                }
            }

            6 => {
                // Make shallow copy
                'case6: {
                    out.put(b"Enter start city for shallow copy: ");
                    let line = match inp.fgets(MAX_INPUT) {
                        Some(l) => l,
                        None => break 'case6,
                    };
                    let input = chomp(&line);

                    let node = match get_node_by_name(&arena, &graph, &input) {
                        Some(n) => n,
                        None => {
                            out.put(b"City '");
                            out.put(&input);
                            out.put(b"' not found\n");
                            break 'case6;
                        }
                    };

                    match shallow_copy(&mut arena, node) {
                        Some(copy) => {
                            out.put(b"Created shallow copy starting from ");
                            out.put(&input);
                            out.put(b"\n");
                            out.put(b"Reference counts incremented for all reachable nodes\n");
                            print_node(&mut out, &arena, copy);
                        }
                        None => {
                            out.put(b"Failed to create shallow copy\n");
                        }
                    }
                }
            }

            7 => {
                // Delete node
                'case7: {
                    out.put(b"Enter city name to delete: ");
                    let line = match inp.fgets(MAX_INPUT) {
                        Some(l) => l,
                        None => break 'case7,
                    };
                    let input = chomp(&line);

                    let node = match get_node_by_name(&arena, &graph, &input) {
                        Some(n) => n,
                        None => {
                            out.put(b"City '");
                            out.put(&input);
                            out.put(b"' not found\n");
                            break 'case7;
                        }
                    };

                    out.put(
                        format!("Current ref count: {}\n", arena.nodes[node].ref_count).as_bytes(),
                    );
                    delete_node(&mut arena, node);
                    out.put(b"Decremented reference count for ");
                    out.put(&input);
                    out.put(b"\n");
                    out.put(b"Note: Node will be freed when ref count reaches 0\n");
                }
            }

            8 => {
                // Exit
                out.put(b"Freeing graph and exiting...\n");
                free_graph(&mut arena, &graph);
                out.flush();
                return;
            }

            _ => {
                out.put(b"Invalid choice\n");
            }
        }
    }

    free_graph(&mut arena, &graph);
    out.flush();
}
