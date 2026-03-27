use std::io::{self, BufRead, Write};
use dag_city_route_manager::*;

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
    let mut graph = match rs_create_graph() {
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
                print!("Enter city name: ");
                io::stdout().flush().unwrap();
                if !read_line(&stdin, &mut input) {
                    break;
                }
                let name = input.trim_end_matches('\n').to_string();
                if rs_add_node(&mut graph, &name).is_some() {
                    print!("Added city: {}\n", name);
                } else {
                    print!("Failed to add city\n");
                }
                io::stdout().flush().unwrap();
            }
            2 => {
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

                let from_idx = rs_get_node_by_name(&graph, &from_city);
                let to_idx = rs_get_node_by_name(&graph, &to_city);

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

                if rs_add_edge(&mut graph, from_idx, to_idx, distance) == 0 {
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
                rs_print_graph(&graph);
                io::stdout().flush().unwrap();
            }
            4 => {
                print!("Enter city name: ");
                io::stdout().flush().unwrap();
                if !read_line(&stdin, &mut input) {
                    break;
                }
                let name = input.trim_end_matches('\n').to_string();
                match rs_get_node_by_name(&graph, &name) {
                    Some(idx) => rs_print_node(&graph, idx),
                    None => print!("City '{}' not found\n", name),
                }
                io::stdout().flush().unwrap();
            }
            5 => {
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

                let start_idx = match rs_get_node_by_name(&graph, &start_city) {
                    Some(i) => i,
                    None => {
                        print!("City '{}' not found\n", start_city);
                        io::stdout().flush().unwrap();
                        continue;
                    }
                };
                let end_idx = match rs_get_node_by_name(&graph, &end_city) {
                    Some(i) => i,
                    None => {
                        print!("City '{}' not found\n", end_city);
                        io::stdout().flush().unwrap();
                        continue;
                    }
                };

                match rs_find_shortest_path(&graph, start_idx, end_idx) {
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
                print!("Enter start city for shallow copy: ");
                io::stdout().flush().unwrap();
                if !read_line(&stdin, &mut input) {
                    break;
                }
                let name = input.trim_end_matches('\n').to_string();
                let node_idx = match rs_get_node_by_name(&graph, &name) {
                    Some(i) => i,
                    None => {
                        print!("City '{}' not found\n", name);
                        io::stdout().flush().unwrap();
                        continue;
                    }
                };

                match rs_shallow_copy(&mut graph, node_idx) {
                    Some(copy_idx) => {
                        print!("Created shallow copy starting from {}\n", name);
                        print!("Reference counts incremented for all reachable nodes\n");
                        rs_print_node(&graph, copy_idx);
                    }
                    None => {
                        print!("Failed to create shallow copy\n");
                    }
                }
                io::stdout().flush().unwrap();
            }
            7 => {
                print!("Enter city name to delete: ");
                io::stdout().flush().unwrap();
                if !read_line(&stdin, &mut input) {
                    break;
                }
                let name = input.trim_end_matches('\n').to_string();
                let node_idx = match rs_get_node_by_name(&graph, &name) {
                    Some(i) => i,
                    None => {
                        print!("City '{}' not found\n", name);
                        io::stdout().flush().unwrap();
                        continue;
                    }
                };

                print!("Current ref count: {}\n", graph.nodes[node_idx].ref_count);
                rs_delete_node(&mut graph, node_idx);
                print!("Decremented reference count for {}\n", name);
                print!("Note: Node will be freed when ref count reaches 0\n");
                io::stdout().flush().unwrap();
            }
            8 => {
                print!("Freeing graph and exiting...\n");
                io::stdout().flush().unwrap();
                rs_free_graph(&mut graph);
                std::process::exit(0);
            }
            _ => {
                print!("Invalid choice\n");
                io::stdout().flush().unwrap();
            }
        }
    }

    rs_free_graph(&mut graph);
}
