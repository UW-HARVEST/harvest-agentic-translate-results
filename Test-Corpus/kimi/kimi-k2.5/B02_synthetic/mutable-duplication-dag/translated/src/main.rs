use std::io::{self, BufRead, Write};
use dag_lib::*;

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

fn main() {
    let mut graph = create_graph().expect("Failed to create graph");
    
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    
    println!("City Route Management System");
    println!("Commands are read from stdin");
    
    loop {
        print_menu();
        
        let input = match lines.next() {
            Some(Ok(line)) => line,
            _ => break,
        };
        
        let choice: i32 = match input.trim().parse() {
            Ok(n) => n,
            Err(_) => {
                println!("Invalid input");
                continue;
            }
        };
        
        match choice {
            1 => {
                print!("Enter city name: ");
                io::stdout().flush().unwrap();
                let city_name = match lines.next() {
                    Some(Ok(line)) => line.trim().to_string(),
                    _ => break,
                };
                
                if add_node(&mut graph, &city_name).is_some() {
                    println!("Added city: {}", city_name);
                } else {
                    println!("Failed to add city");
                }
            }
            
            2 => {
                print!("Enter from city: ");
                io::stdout().flush().unwrap();
                let from_city = match lines.next() {
                    Some(Ok(line)) => line.trim().to_string(),
                    _ => break,
                };
                
                print!("Enter to city: ");
                io::stdout().flush().unwrap();
                let to_city = match lines.next() {
                    Some(Ok(line)) => line.trim().to_string(),
                    _ => break,
                };
                
                print!("Enter distance: ");
                io::stdout().flush().unwrap();
                let distance_input = match lines.next() {
                    Some(Ok(line)) => line,
                    _ => break,
                };
                
                let distance: i32 = match distance_input.trim().parse() {
                    Ok(n) => n,
                    Err(_) => {
                        println!("Invalid distance");
                        continue;
                    }
                };
                
                let from = match get_node_by_name(&graph, &from_city) {
                    Some(n) => n,
                    None => {
                        println!("City '{}' not found", from_city);
                        continue;
                    }
                };
                
                let to = match get_node_by_name(&graph, &to_city) {
                    Some(n) => n,
                    None => {
                        println!("City '{}' not found", to_city);
                        continue;
                    }
                };
                
                if add_edge(&from, &to, distance) == 0 {
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
                let city_name = match lines.next() {
                    Some(Ok(line)) => line.trim().to_string(),
                    _ => break,
                };
                
                match get_node_by_name(&graph, &city_name) {
                    Some(node) => print_node(&node),
                    None => println!("City '{}' not found", city_name),
                }
            }
            
            5 => {
                print!("Enter start city: ");
                io::stdout().flush().unwrap();
                let start_city = match lines.next() {
                    Some(Ok(line)) => line.trim().to_string(),
                    _ => break,
                };
                
                print!("Enter end city: ");
                io::stdout().flush().unwrap();
                let end_city = match lines.next() {
                    Some(Ok(line)) => line.trim().to_string(),
                    _ => break,
                };
                
                let start = match get_node_by_name(&graph, &start_city) {
                    Some(n) => n,
                    None => {
                        println!("City '{}' not found", start_city);
                        continue;
                    }
                };
                
                let end = match get_node_by_name(&graph, &end_city) {
                    Some(n) => n,
                    None => {
                        println!("City '{}' not found", end_city);
                        continue;
                    }
                };
                
                match find_shortest_path(&start, &end) {
                    Some(path) => {
                        println!("Shortest path from {} to {}:", start_city, end_city);
                        for (i, node) in path.iter().enumerate() {
                            let guard = node.read().unwrap();
                            println!("  {}. {}", i + 1, guard.city_name);
                        }
                    }
                    None => println!("No path found"),
                }
            }
            
            6 => {
                print!("Enter start city for shallow copy: ");
                io::stdout().flush().unwrap();
                let city_name = match lines.next() {
                    Some(Ok(line)) => line.trim().to_string(),
                    _ => break,
                };
                
                let node = match get_node_by_name(&graph, &city_name) {
                    Some(n) => n,
                    None => {
                        println!("City '{}' not found", city_name);
                        continue;
                    }
                };
                
                match shallow_copy(&node) {
                    Some(copy) => {
                        println!("Created shallow copy starting from {}", city_name);
                        println!("Reference counts incremented for all reachable nodes");
                        print_node(&copy);
                    }
                    None => println!("Failed to create shallow copy"),
                }
            }
            
            7 => {
                print!("Enter city name to delete: ");
                io::stdout().flush().unwrap();
                let city_name = match lines.next() {
                    Some(Ok(line)) => line.trim().to_string(),
                    _ => break,
                };
                
                let node = match get_node_by_name(&graph, &city_name) {
                    Some(n) => n,
                    None => {
                        println!("City '{}' not found", city_name);
                        continue;
                    }
                };
                
                delete_node(&node);
                println!("Decremented reference count for {}", city_name);
                println!("Note: Node will be freed when ref count reaches 0");
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
