use std::io::{self, Write};

use driver::{
    add_edge, add_node, create_graph, delete_node, find_shortest_path, free_graph,
    get_node_by_name, print_graph, print_node, shallow_copy,
};

const MAX_INPUT: usize = 256;

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
    let _ = io::stdout().flush();
}

fn read_line() -> Option<String> {
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(0) => None,
        Ok(_) => {
            if input.len() > MAX_INPUT {
                input.truncate(MAX_INPUT);
            }
            Some(input.trim_end_matches(['\r', '\n']).to_string())
        }
        Err(_) => None,
    }
}

fn main() {
    let Some(mut graph) = create_graph() else {
        eprintln!("Failed to create graph");
        std::process::exit(1);
    };

    println!("City Route Management System");
    println!("Commands are read from stdin");

    loop {
        print_menu();

        let Some(input) = read_line() else {
            break;
        };

        let Ok(choice) = input.trim().parse::<i32>() else {
            println!("Invalid input");
            continue;
        };

        match choice {
            1 => {
                print!("Enter city name: ");
                let _ = io::stdout().flush();
                let Some(input) = read_line() else {
                    break;
                };

                if add_node(&mut graph, &input).is_some() {
                    println!("Added city: {}", input);
                } else {
                    println!("Failed to add city");
                }
            }
            2 => {
                print!("Enter from city: ");
                let _ = io::stdout().flush();
                let Some(from_city) = read_line() else {
                    break;
                };

                print!("Enter to city: ");
                let _ = io::stdout().flush();
                let Some(to_city) = read_line() else {
                    break;
                };

                print!("Enter distance: ");
                let _ = io::stdout().flush();
                let Some(input) = read_line() else {
                    break;
                };
                let Ok(distance) = input.trim().parse::<i32>() else {
                    println!("Invalid distance");
                    continue;
                };

                let Some(from) = get_node_by_name(&graph, &from_city) else {
                    println!("City '{}' not found", from_city);
                    continue;
                };
                let Some(to) = get_node_by_name(&graph, &to_city) else {
                    println!("City '{}' not found", to_city);
                    continue;
                };

                if add_edge(&from, &to, distance) == 0 {
                    println!(
                        "Added route: {} -> {} (distance: {})",
                        from_city, to_city, distance
                    );
                } else {
                    println!("Failed to add route");
                }
            }
            3 => {
                print_graph(&graph);
            }
            4 => {
                print!("Enter city name: ");
                let _ = io::stdout().flush();
                let Some(input) = read_line() else {
                    break;
                };

                if let Some(node) = get_node_by_name(&graph, &input) {
                    print_node(&node);
                } else {
                    println!("City '{}' not found", input);
                }
            }
            5 => {
                print!("Enter start city: ");
                let _ = io::stdout().flush();
                let Some(start_city) = read_line() else {
                    break;
                };

                print!("Enter end city: ");
                let _ = io::stdout().flush();
                let Some(end_city) = read_line() else {
                    break;
                };

                let Some(start) = get_node_by_name(&graph, &start_city) else {
                    println!("City '{}' not found", start_city);
                    continue;
                };
                let Some(end) = get_node_by_name(&graph, &end_city) else {
                    println!("City '{}' not found", end_city);
                    continue;
                };

                let mut path_length = 0;
                if let Some(path) = find_shortest_path(&start, &end, &mut path_length) {
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
                let _ = io::stdout().flush();
                let Some(input) = read_line() else {
                    break;
                };

                let Some(node) = get_node_by_name(&graph, &input) else {
                    println!("City '{}' not found", input);
                    continue;
                };

                if let Some(copy) = shallow_copy(&node) {
                    println!("Created shallow copy starting from {}", input);
                    println!("Reference counts incremented for all reachable nodes");
                    print_node(&copy);
                } else {
                    println!("Failed to create shallow copy");
                }
            }
            7 => {
                print!("Enter city name to delete: ");
                let _ = io::stdout().flush();
                let Some(input) = read_line() else {
                    break;
                };

                let Some(node) = get_node_by_name(&graph, &input) else {
                    println!("City '{}' not found", input);
                    continue;
                };

                println!("Current ref count: {}", node.borrow().ref_count);
                delete_node(&node);
                println!("Decremented reference count for {}", input);
                println!("Note: Node will be freed when ref count reaches 0");
            }
            8 => {
                println!("Freeing graph and exiting...");
                free_graph(&mut graph);
                return;
            }
            _ => {
                println!("Invalid choice");
            }
        }
    }

    free_graph(&mut graph);
}
