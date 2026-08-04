mod scene;
mod shape;

use scene::Scene;
use shape::{shape_equals, shape_get, shape_manager_cleanup, shape_manager_init, shape_print, ShapeType};
use std::io::{self, Write};

fn print_menu() {
    println!("\n=========================================");
    println!("  ASCII ART DRAWING APPLICATION");
    println!("=========================================");
    println!("1. View all available shapes");
    println!("2. Create new scene");
    println!("3. Add shape to scene");
    println!("4. Remove shape from scene");
    println!("5. View scene");
    println!("6. List all scenes");
    println!("7. Save scene");
    println!("8. Load scene");
    println!("9. Compare two shapes");
    println!("10. Compare two scenes");
    println!("11. Delete scene");
    println!("12. Exit");
    println!("=========================================");
    print!("Choice: ");
    io::stdout().flush().unwrap();
}

fn read_line() -> String {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn read_int() -> Option<usize> {
    read_line().parse().ok()
}

fn main() {
    println!("╔════════════════════════════════════════╗");
    println!("║  ASCII ART DRAWING APPLICATION        ║");
    println!("║  Child-Friendly Shape Editor           ║");
    println!("╚════════════════════════════════════════╝");

    shape_manager_init();

    let mut scenes: Vec<Scene> = Vec::new();

    loop {
        print_menu();
        let choice = match read_int() {
            Some(c) => c,
            None => {
                println!("Invalid input");
                continue;
            }
        };

        match choice {
            1 => {
                println!("\n=== Available Shapes ===");
                for i in 0..ShapeType::COUNT {
                    if let Some(st) = ShapeType::from_usize(i) {
                        println!("\n{}. ", i + 1);
                        shape_print(shape_get(st));
                    }
                }
            }
            2 => {
                if scenes.len() >= 10 {
                    println!("Error: Maximum scenes reached");
                    continue;
                }
                print!("Enter scene name: ");
                io::stdout().flush().unwrap();
                let name = read_line();
                scenes.push(Scene::new(&name));
                println!("Scene '{}' created (index {})", name, scenes.len() - 1);
            }
            3 => {
                if scenes.is_empty() {
                    println!("No scenes available. Create a scene first.");
                    continue;
                }
                print!("Select scene (0-{}): ", scenes.len() - 1);
                io::stdout().flush().unwrap();
                let scene_idx = match read_int() {
                    Some(idx) if idx < scenes.len() => idx,
                    _ => {
                        println!("Invalid scene index");
                        continue;
                    }
                };

                println!("\nSelect shape to add:");
                for i in 0..ShapeType::COUNT {
                    if let Some(st) = ShapeType::from_usize(i) {
                        println!("{}. {}", i, st.name());
                    }
                }
                print!("Choice: ");
                io::stdout().flush().unwrap();
                let shape_type_idx = match read_int() {
                    Some(idx) if idx < ShapeType::COUNT => idx,
                    _ => {
                        println!("Invalid shape type");
                        continue;
                    }
                };

                if let Some(st) = ShapeType::from_usize(shape_type_idx) {
                    if let Some(shape) = shape_get(st) {
                        if scenes[scene_idx].add_shape(shape).is_ok() {
                            println!(
                                "Shape '{}' added to scene (reusing singleton at {:p})",
                                shape.name, shape
                            );
                        } else {
                            println!("Error adding shape");
                        }
                    }
                }
            }
            4 => {
                if scenes.is_empty() {
                    println!("No scenes available");
                    continue;
                }
                print!("Select scene (0-{}): ", scenes.len() - 1);
                io::stdout().flush().unwrap();
                let scene_idx = match read_int() {
                    Some(idx) if idx < scenes.len() => idx,
                    _ => {
                        println!("Invalid scene index");
                        continue;
                    }
                };

                scenes[scene_idx].list_shapes();

                if scenes[scene_idx].shapes.is_empty() {
                    println!("Scene is empty");
                    continue;
                }

                print!(
                    "Select shape to remove (1-{}): ",
                    scenes[scene_idx].shapes.len()
                );
                io::stdout().flush().unwrap();
                let shape_idx = match read_int() {
                    Some(idx) if idx > 0 && idx <= scenes[scene_idx].shapes.len() => idx - 1,
                    _ => {
                        println!("Invalid input");
                        continue;
                    }
                };

                if scenes[scene_idx].remove_shape(shape_idx).is_ok() {
                    println!("Shape removed");
                } else {
                    println!("Error removing shape");
                }
            }
            5 => {
                if scenes.is_empty() {
                    println!("No scenes available");
                    continue;
                }
                print!("Select scene (0-{}): ", scenes.len() - 1);
                io::stdout().flush().unwrap();
                let scene_idx = match read_int() {
                    Some(idx) if idx < scenes.len() => idx,
                    _ => {
                        println!("Invalid scene index");
                        continue;
                    }
                };
                scenes[scene_idx].print();
            }
            6 => {
                println!("\n=== All Scenes ===");
                if scenes.is_empty() {
                    println!("No scenes created yet");
                    continue;
                }
                for (i, scene) in scenes.iter().enumerate() {
                    println!("{}. {} ({} shapes)", i, scene.name, scene.shapes.len());
                }
            }
            7 => {
                if scenes.is_empty() {
                    println!("No scenes available");
                    continue;
                }
                print!("Select scene (0-{}): ", scenes.len() - 1);
                io::stdout().flush().unwrap();
                let scene_idx = match read_int() {
                    Some(idx) if idx < scenes.len() => idx,
                    _ => {
                        println!("Invalid scene index");
                        continue;
                    }
                };
                print!("Enter filename: ");
                io::stdout().flush().unwrap();
                let filename = read_line();
                if scenes[scene_idx].save(&filename).is_err() {
                    println!("Error: Could not open file '{}' for writing", filename);
                }
            }
            8 => {
                if scenes.len() >= 10 {
                    println!("Error: Maximum scenes reached");
                    continue;
                }
                print!("Enter filename: ");
                io::stdout().flush().unwrap();
                let filename = read_line();
                match Scene::load(&filename) {
                    Ok(scene) => {
                        scenes.push(scene);
                        println!("Scene loaded (index {})", scenes.len() - 1);
                    }
                    Err(_) => {
                        println!("Error: Could not open file '{}' for reading", filename);
                    }
                }
            }
            9 => {
                println!("\nSelect first shape (0-{}):", ShapeType::COUNT - 1);
                for i in 0..ShapeType::COUNT {
                    if let Some(st) = ShapeType::from_usize(i) {
                        println!("{}. {}", i, st.name());
                    }
                }
                print!("Choice: ");
                io::stdout().flush().unwrap();
                let type1 = match read_int() {
                    Some(idx) if idx < ShapeType::COUNT => idx,
                    _ => {
                        println!("Invalid shape type");
                        continue;
                    }
                };

                print!("\nSelect second shape (0-{}): ", ShapeType::COUNT - 1);
                io::stdout().flush().unwrap();
                let type2 = match read_int() {
                    Some(idx) if idx < ShapeType::COUNT => idx,
                    _ => {
                        println!("Invalid shape type");
                        continue;
                    }
                };

                let s1 = shape_get(ShapeType::from_usize(type1).unwrap());
                let s2 = shape_get(ShapeType::from_usize(type2).unwrap());

                if let (Some(shape1), Some(shape2)) = (s1, s2) {
                    println!("\nShape 1: {} (ptr: {:p})", shape1.name, shape1);
                    println!("Shape 2: {} (ptr: {:p})", shape2.name, shape2);
                    println!(
                        "Comparison of pointers: {}",
                        std::ptr::eq(shape1, shape2) as i32
                    );

                    if shape_equals(s1, s2) {
                        println!("Result: Shapes are EQUAL (same instance)");
                    } else {
                        println!("Result: Shapes are NOT EQUAL (different instances)");
                    }
                }
            }
            10 => {
                if scenes.len() < 2 {
                    println!("Need at least 2 scenes to compare");
                    continue;
                }
                print!("Select first scene (0-{}): ", scenes.len() - 1);
                io::stdout().flush().unwrap();
                let idx1 = match read_int() {
                    Some(idx) if idx < scenes.len() => idx,
                    _ => {
                        println!("Invalid scene index");
                        continue;
                    }
                };

                print!("Select second scene (0-{}): ", scenes.len() - 1);
                io::stdout().flush().unwrap();
                let idx2 = match read_int() {
                    Some(idx) if idx < scenes.len() => idx,
                    _ => {
                        println!("Invalid scene index");
                        continue;
                    }
                };

                let sc1 = &scenes[idx1];
                let sc2 = &scenes[idx2];

                println!("\nScene 1: {} ({} shapes)", sc1.name, sc1.shapes.len());
                sc1.list_shapes();

                println!("\nScene 2: {} ({} shapes)", sc2.name, sc2.shapes.len());
                sc2.list_shapes();

                if sc1.equals(sc2) {
                    println!("\nResult: Scenes are EQUAL (1:1 correspondence)");
                } else {
                    println!("\nResult: Scenes are NOT EQUAL");
                }
            }
            11 => {
                if scenes.is_empty() {
                    println!("No scenes available");
                    continue;
                }
                print!("Select scene to delete (0-{}): ", scenes.len() - 1);
                io::stdout().flush().unwrap();
                let scene_idx = match read_int() {
                    Some(idx) if idx < scenes.len() => idx,
                    _ => {
                        println!("Invalid scene index");
                        continue;
                    }
                };

                scenes.remove(scene_idx);
                println!("Scene deleted");
            }
            12 => {
                println!("\nCleaning up and exiting...");
                shape_manager_cleanup();
                println!("Goodbye!");
                break;
            }
            _ => {
                println!("Invalid choice");
            }
        }
    }
}
