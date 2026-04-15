use std::io::{self, Write};
use driver::*;

fn print_menu() {
    println!("\n=== DAG City Route Manager ===");
    println!("1. Add city (node)");
    println!("2. Add route (edge)");
    println!("3. Show all cities");
    println!("4. Show city details");
    println!("5. Find shortest path");
    println!("6. Make shallow copy of subsection");
    println!("7. Delete node");
    println!("8. Exit");
    print!("Choice: ");
    io::stdout().flush().unwrap();
}

fn read_line() -> Option<String> {
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        if input.is_empty() {
            None
        } else {
            Some(input.trim_end_matches('\n').trim_end_matches('\r').to_string())
        }
    } else {
        None
    }
}

fn main() {
    let mut graph = match create_graph() {
        Some(g) => g,
        None => {
            eprintln!("Failed to create graph");
            std::process::exit(1);
        }
    };

    println!("City Route Management System");
    println!("Commands are read from stdin");

    loop {
        print_menu();

        let input = match read_line() {
            Some(s) => s,
            None => break,
        };

        let choice: i32 = match input.trim().parse() {
            Ok(num) => num,
            Err(_) => {
                println!("Invalid input");
                continue;
            }
        };

        match choice {
            1 => {
                print!("Enter city name: ");
                io::stdout().flush().unwrap();
                let city_name = match read_line() {
                    Some(s) => s,
                    None => break,
                };

                if let Some(_) = add_node(&mut graph, &city_name) {
                    println!("Added city: {}", city_name);
                } else {
                    println!("Failed to add city");
                }
            }
            2 => {
                print!("Enter from city: ");
                io::stdout().flush().unwrap();
                let from_city = match read_line() {
                    Some(s) => s,
                    None => break,
                };

                print!("Enter to city: ");
                io::stdout().flush().unwrap();
                let to_city = match read_line() {
                    Some(s) => s,
                    None => break,
                };

                print!("Enter distance: ");
                io::stdout().flush().unwrap();
                let distance_str = match read_line() {
                    Some(s) => s,
                    None => break,
                };
                let distance: i32 = match distance_str.trim().parse() {
                    Ok(d) => d,
                    Err(_) => {
                        println!("Invalid distance");
                        continue;
                    }
                };

                let from = get_node_by_name(&graph, &from_city);
                let to = get_node_by_name(&graph, &to_city);

                if from.is_none() {
                    println!("City '{}' not found", from_city);
                    continue;
                }
                if to.is_none() {
                    println!("City '{}' not found", to_city);
                    continue;
                }

                if add_edge(&from.unwrap(), &to.unwrap(), distance) == 0 {
                    println!("Added route: {} -> {} (distance: {})", from_city, to_city, distance);
                } else {
                    println!("Failed to add route");
                }
            }
            3 => {
                print_graph(&graph);
            }
            4 => {
                print!("Enter city name: ");
                io::stdout().flush().unwrap();
                let city_name = match read_line() {
                    Some(s) => s,
                    None => break,
                };

                if let Some(node) = get_node_by_name(&graph, &city_name) {
                    print_node(&node);
                } else {
                    println!("City '{}' not found", city_name);
                }
            }
            5 => {
                print!("Enter start city: ");
                io::stdout().flush().unwrap();
                let start_city = match read_line() {
                    Some(s) => s,
                    None => break,
                };

                print!("Enter end city: ");
                io::stdout().flush().unwrap();
                let end_city = match read_line() {
                    Some(s) => s,
                    None => break,
                };

                let start = get_node_by_name(&graph, &start_city);
                let end = get_node_by_name(&graph, &end_city);

                if start.is_none() {
                    println!("City '{}' not found", start_city);
                    continue;
                }
                if end.is_none() {
                    println!("City '{}' not found", end_city);
                    continue;
                }

                if let Some(path) = find_shortest_path(&start.unwrap(), &end.unwrap()) {
                    println!("Shortest path from {} to {}:", start_city, end_city);
                    for (i, node) in path.iter().enumerate() {
                        println!("  {}. {}", i + 1, node.borrow().city_name);
                    }
                } else {
                    println!("No path found");
                }
            }
            6 => {
                print!("Enter start city for shallow copy: ");
                io::stdout().flush().unwrap();
                let city_name = match read_line() {
                    Some(s) => s,
                    None => break,
                };

                if let Some(node) = get_node_by_name(&graph, &city_name) {
                    if let Some(copy) = shallow_copy(&node) {
                        println!("Created shallow copy starting from {}", city_name);
                        println!("Reference counts incremented for all reachable nodes");
                        print_node(&copy);
                    } else {
                        println!("Failed to create shallow copy");
                    }
                } else {
                    println!("City '{}' not found", city_name);
                }
            }
            7 => {
                print!("Enter city name to delete: ");
                io::stdout().flush().unwrap();
                let city_name = match read_line() {
                    Some(s) => s,
                    None => break,
                };

                if let Some(node) = get_node_by_name(&graph, &city_name) {
                    println!("Current ref count: {}", node.borrow().ref_count);
                    delete_node(&node);
                    println!("Decremented reference count for {}", city_name);
                    println!("Note: Node will be freed when ref count reaches 0");
                } else {
                    println!("City '{}' not found", city_name);
                }
            }
            8 => {
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
