// main.rs - Rust translation of main.c

mod dag_lib;

use dag_lib::*;
use std::io::{self, BufRead, Write};

fn print_menu() {
    println!();
    println!("=== DAG City Route Manager ===");
    println!("1. Add city (node)");
    println!("2. Add route (edge)");
    println!("3. Show all cities");
    println!("4. Show city details");
    println!("5. Find shortest path");
    println!("6. Make shallow copy of subsection");
    println!("7. Delete node");
    println!("8. Exit");
    print!("Choice: ");
    io::stdout().flush().ok();
}

/// Read a line from stdin. Returns None on EOF, otherwise the trimmed line.
fn read_line<R: BufRead>(reader: &mut R) -> Option<String> {
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) => None,
        Ok(_) => {
            // Strip trailing newline characters
            while line.ends_with('\n') || line.ends_with('\r') {
                line.pop();
            }
            Some(line)
        }
        Err(_) => None,
    }
}

fn main() {
    let graph = create_graph();
    if graph.is_null() {
        eprintln!("Failed to create graph");
        std::process::exit(1);
    }

    let stdin = io::stdin();
    let mut reader = stdin.lock();

    println!("City Route Management System");
    println!("Commands are read from stdin");

    loop {
        print_menu();

        let line = match read_line(&mut reader) {
            Some(s) => s,
            None => break,
        };

        let choice: i32 = match line.trim().parse() {
            Ok(c) => c,
            Err(_) => {
                println!("Invalid input");
                continue;
            }
        };

        match choice {
            1 => {
                // Add city
                print!("Enter city name: ");
                io::stdout().flush().ok();
                let city = match read_line(&mut reader) {
                    Some(s) => s,
                    None => break,
                };

                let node = add_node(graph, &city);
                if !node.is_null() {
                    println!("Added city: {}", city);
                } else {
                    println!("Failed to add city");
                }
            }

            2 => {
                // Add route
                print!("Enter from city: ");
                io::stdout().flush().ok();
                let from_city = match read_line(&mut reader) {
                    Some(s) => s,
                    None => break,
                };

                print!("Enter to city: ");
                io::stdout().flush().ok();
                let to_city = match read_line(&mut reader) {
                    Some(s) => s,
                    None => break,
                };

                print!("Enter distance: ");
                io::stdout().flush().ok();
                let dist_line = match read_line(&mut reader) {
                    Some(s) => s,
                    None => break,
                };

                let distance: i32 = match dist_line.trim().parse() {
                    Ok(d) => d,
                    Err(_) => {
                        println!("Invalid distance");
                        continue;
                    }
                };

                let from = get_node_by_name(graph, &from_city);
                let to = get_node_by_name(graph, &to_city);

                if from.is_null() {
                    println!("City '{}' not found", from_city);
                    continue;
                }
                if to.is_null() {
                    println!("City '{}' not found", to_city);
                    continue;
                }

                if add_edge(from, to, distance) == 0 {
                    println!(
                        "Added route: {} -> {} (distance: {})",
                        from_city, to_city, distance
                    );
                } else {
                    println!("Failed to add route");
                }
            }

            3 => {
                // Show all cities
                print_graph(graph);
            }

            4 => {
                // Show city details
                print!("Enter city name: ");
                io::stdout().flush().ok();
                let city = match read_line(&mut reader) {
                    Some(s) => s,
                    None => break,
                };

                let node = get_node_by_name(graph, &city);
                if !node.is_null() {
                    print_node(node);
                } else {
                    println!("City '{}' not found", city);
                }
            }

            5 => {
                // Find shortest path
                print!("Enter start city: ");
                io::stdout().flush().ok();
                let start_city = match read_line(&mut reader) {
                    Some(s) => s,
                    None => break,
                };

                print!("Enter end city: ");
                io::stdout().flush().ok();
                let end_city = match read_line(&mut reader) {
                    Some(s) => s,
                    None => break,
                };

                let start = get_node_by_name(graph, &start_city);
                let end = get_node_by_name(graph, &end_city);

                if start.is_null() {
                    println!("City '{}' not found", start_city);
                    continue;
                }
                if end.is_null() {
                    println!("City '{}' not found", end_city);
                    continue;
                }

                match find_shortest_path(start, end) {
                    Some(path) => {
                        println!("Shortest path from {} to {}:", start_city, end_city);
                        for (i, node_ptr) in path.iter().enumerate() {
                            unsafe {
                                let n = &**node_ptr;
                                println!("  {}. {}", i + 1, city_name_to_str(&n.city_name));
                            }
                        }
                        // path Vec is dropped here, replacing C's free(path)
                    }
                    None => {
                        println!("No path found");
                    }
                }
            }

            6 => {
                // Make shallow copy
                print!("Enter start city for shallow copy: ");
                io::stdout().flush().ok();
                let city = match read_line(&mut reader) {
                    Some(s) => s,
                    None => break,
                };

                let node = get_node_by_name(graph, &city);
                if node.is_null() {
                    println!("City '{}' not found", city);
                    continue;
                }

                let copy = shallow_copy(node);
                if !copy.is_null() {
                    println!("Created shallow copy starting from {}", city);
                    println!("Reference counts incremented for all reachable nodes");
                    print_node(copy);
                } else {
                    println!("Failed to create shallow copy");
                }
            }

            7 => {
                // Delete node
                print!("Enter city name to delete: ");
                io::stdout().flush().ok();
                let city = match read_line(&mut reader) {
                    Some(s) => s,
                    None => break,
                };

                let node = get_node_by_name(graph, &city);
                if node.is_null() {
                    println!("City '{}' not found", city);
                    continue;
                }

                unsafe {
                    println!("Current ref count: {}", (*node).ref_count);
                }
                delete_node(node);
                println!("Decremented reference count for {}", city);
                println!("Note: Node will be freed when ref count reaches 0");
            }

            8 => {
                // Exit
                println!("Freeing graph and exiting...");
                free_graph(graph);
                return;
            }

            _ => {
                println!("Invalid choice");
            }
        }
    }

    free_graph(graph);
}
