use std::io::{self, Write, BufRead};
use std::sync::Mutex;

mod shape;
mod scene;

use shape::{ShapeManager, ShapeType};
use scene::Scene;

const MAX_SCENES: usize = 10;

static SCENES: Mutex<Vec<Option<Scene>>> = Mutex::new(Vec::new());

fn print_menu() {
    println!();
    println!("=========================================");
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

fn view_all_shapes() {
    println!("\n=== Available Shapes ===");
    for i in 0..ShapeType::COUNT {
        println!("\n{}.", i + 1);
        if let Some(shape) = ShapeManager::get(ShapeType::from_usize(i)) {
            shape.print();
        }
    }
}

fn create_new_scene() {
    let mut scenes = SCENES.lock().unwrap();
    if scenes.len() >= MAX_SCENES {
        println!("Error: Maximum scenes reached");
        return;
    }
    
    print!("Enter scene name: ");
    io::stdout().flush().unwrap();
    
    let mut name = String::new();
    if io::stdin().lock().read_line(&mut name).is_err() {
        return;
    }
    let name = name.trim().to_string();
    
    let scene = Scene::new(&name);
    let idx = scenes.len();
    scenes.push(Some(scene));
    println!("Scene '{}' created (index {})", name, idx);
}

fn add_shape_to_scene() {
    let scenes = SCENES.lock().unwrap();
    if scenes.is_empty() {
        println!("No scenes available. Create a scene first.");
        return;
    }
    drop(scenes);
    
    print!("Select scene (0-{}): ", SCENES.lock().unwrap().len() - 1);
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    if io::stdin().lock().read_line(&mut input).is_err() {
        println!("Invalid input");
        return;
    }
    
    let scene_idx: usize = match input.trim().parse() {
        Ok(n) => n,
        Err(_) => {
            println!("Invalid input");
            return;
        }
    };
    
    let mut scenes = SCENES.lock().unwrap();
    if scene_idx >= scenes.len() {
        println!("Invalid scene index");
        return;
    }
    drop(scenes);
    
    println!("\nSelect shape to add:");
    for i in 0..ShapeType::COUNT {
        println!("{}. {}", i, ShapeType::from_usize(i).name());
    }
    print!("Choice: ");
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    if io::stdin().lock().read_line(&mut input).is_err() {
        println!("Invalid input");
        return;
    }
    
    let shape_type: usize = match input.trim().parse() {
        Ok(n) => n,
        Err(_) => {
            println!("Invalid input");
            return;
        }
    };
    
    if shape_type >= ShapeType::COUNT {
        println!("Invalid shape type");
        return;
    }
    
    let shape = ShapeManager::get(ShapeType::from_usize(shape_type));
    let mut scenes = SCENES.lock().unwrap();
    if let Some(ref mut scene) = scenes[scene_idx] {
        match scene.add_shape(shape.unwrap()) {
            Ok(_) => {
                let ptr = format!("{:p}", &shape);
                println!("Shape '{}' added to scene (reusing singleton at {})", 
                       shape.unwrap().name(), ptr);
            }
            Err(_) => println!("Error adding shape"),
        }
    }
}

fn remove_shape_from_scene() {
    let scenes = SCENES.lock().unwrap();
    if scenes.is_empty() {
        println!("No scenes available");
        return;
    }
    drop(scenes);
    
    print!("Select scene (0-{}): ", SCENES.lock().unwrap().len() - 1);
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    if io::stdin().lock().read_line(&mut input).is_err() {
        println!("Invalid input");
        return;
    }
    
    let scene_idx: usize = match input.trim().parse() {
        Ok(n) => n,
        Err(_) => {
            println!("Invalid input");
            return;
        }
    };
    
    let mut scenes = SCENES.lock().unwrap();
    if scene_idx >= scenes.len() {
        println!("Invalid scene index");
        return;
    }
    
    if let Some(ref scene) = scenes[scene_idx] {
        scene.list_shapes();
        
        if scene.shape_count() == 0 {
            println!("Scene is empty");
            return;
        }
        
        print!("Select shape to remove (1-{}): ", scene.shape_count());
        io::stdout().flush().unwrap();
        drop(scenes);
        
        let mut input = String::new();
        if io::stdin().lock().read_line(&mut input).is_err() {
            println!("Invalid input");
            return;
        }
        
        let shape_idx: usize = match input.trim().parse() {
            Ok(n) => n,
            Err(_) => {
                println!("Invalid input");
                return;
            }
        };
        
        let mut scenes = SCENES.lock().unwrap();
        if let Some(ref mut scene) = scenes[scene_idx] {
            match scene.remove_shape(shape_idx - 1) {
                Ok(_) => println!("Shape removed"),
                Err(_) => println!("Error removing shape"),
            }
        }
    }
}

fn view_scene() {
    let scenes = SCENES.lock().unwrap();
    if scenes.is_empty() {
        println!("No scenes available");
        return;
    }
    drop(scenes);
    
    print!("Select scene (0-{}): ", SCENES.lock().unwrap().len() - 1);
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    if io::stdin().lock().read_line(&mut input).is_err() {
        println!("Invalid input");
        return;
    }
    
    let scene_idx: usize = match input.trim().parse() {
        Ok(n) => n,
        Err(_) => {
            println!("Invalid input");
            return;
        }
    };
    
    let scenes = SCENES.lock().unwrap();
    if scene_idx >= scenes.len() {
        println!("Invalid scene index");
        return;
    }
    
    if let Some(ref scene) = scenes[scene_idx] {
        scene.print();
    }
}

fn list_all_scenes() {
    println!("\n=== All Scenes ===");
    let scenes = SCENES.lock().unwrap();
    if scenes.is_empty() {
        println!("No scenes created yet");
        return;
    }
    
    for (i, scene_opt) in scenes.iter().enumerate() {
        if let Some(ref scene) = scene_opt {
            println!("{}. {} ({} shapes)", i, scene.name(), scene.shape_count());
        }
    }
}

fn save_scene_to_file() {
    let scenes = SCENES.lock().unwrap();
    if scenes.is_empty() {
        println!("No scenes available");
        return;
    }
    drop(scenes);
    
    print!("Select scene (0-{}): ", SCENES.lock().unwrap().len() - 1);
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    if io::stdin().lock().read_line(&mut input).is_err() {
        println!("Invalid input");
        return;
    }
    
    let scene_idx: usize = match input.trim().parse() {
        Ok(n) => n,
        Err(_) => {
            println!("Invalid input");
            return;
        }
    };
    
    let scenes = SCENES.lock().unwrap();
    if scene_idx >= scenes.len() {
        println!("Invalid scene index");
        return;
    }
    drop(scenes);
    
    print!("Enter filename: ");
    io::stdout().flush().unwrap();
    
    let mut filename = String::new();
    if io::stdin().lock().read_line(&mut filename).is_err() {
        return;
    }
    let filename = filename.trim();
    
    let scenes = SCENES.lock().unwrap();
    if let Some(ref scene) = scenes[scene_idx] {
        scene.save(filename);
    }
}

fn load_scene_from_file() {
    let mut scenes = SCENES.lock().unwrap();
    if scenes.len() >= MAX_SCENES {
        println!("Error: Maximum scenes reached");
        return;
    }
    drop(scenes);
    
    print!("Enter filename: ");
    io::stdout().flush().unwrap();
    
    let mut filename = String::new();
    if io::stdin().lock().read_line(&mut filename).is_err() {
        return;
    }
    let filename = filename.trim();
    
    if let Some(scene) = Scene::load(filename) {
        let mut scenes = SCENES.lock().unwrap();
        let idx = scenes.len();
        scenes.push(Some(scene));
        println!("Scene loaded (index {})", idx);
    }
}

fn compare_shapes() {
    println!("\nSelect first shape (0-{}):", ShapeType::COUNT - 1);
    for i in 0..ShapeType::COUNT {
        println!("{}. {}", i, ShapeType::from_usize(i).name());
    }
    print!("Choice: ");
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    if io::stdin().lock().read_line(&mut input).is_err() {
        println!("Invalid input");
        return;
    }
    
    let type1: usize = match input.trim().parse() {
        Ok(n) => n,
        Err(_) => {
            println!("Invalid input");
            return;
        }
    };
    
    print!("\nSelect second shape (0-{}): ", ShapeType::COUNT - 1);
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    if io::stdin().lock().read_line(&mut input).is_err() {
        println!("Invalid input");
        return;
    }
    
    let type2: usize = match input.trim().parse() {
        Ok(n) => n,
        Err(_) => {
            println!("Invalid input");
            return;
        }
    };
    
    if type1 >= ShapeType::COUNT || type2 >= ShapeType::COUNT {
        println!("Invalid shape type");
        return;
    }
    
    let s1 = ShapeManager::get(ShapeType::from_usize(type1)).unwrap();
    let s2 = ShapeManager::get(ShapeType::from_usize(type2)).unwrap();
    
    println!("\nShape 1: {} (ptr: {:p})", s1.name(), s1);
    println!("Shape 2: {} (ptr: {:p})", s2.name(), s2);
    println!("Comparison of pointers: {}", std::ptr::eq(s1, s2));
    
    if ShapeManager::equals(s1, s2) {
        println!("Result: Shapes are EQUAL (same instance)");
    } else {
        println!("Result: Shapes are NOT EQUAL (different instances)");
    }
}

fn compare_scenes() {
    let scenes = SCENES.lock().unwrap();
    if scenes.len() < 2 {
        println!("Need at least 2 scenes to compare");
        return;
    }
    drop(scenes);
    
    print!("Select first scene (0-{}): ", SCENES.lock().unwrap().len() - 1);
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    if io::stdin().lock().read_line(&mut input).is_err() {
        println!("Invalid input");
        return;
    }
    
    let idx1: usize = match input.trim().parse() {
        Ok(n) => n,
        Err(_) => {
            println!("Invalid input");
            return;
        }
    };
    
    print!("Select second scene (0-{}): ", SCENES.lock().unwrap().len() - 1);
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    if io::stdin().lock().read_line(&mut input).is_err() {
        println!("Invalid input");
        return;
    }
    
    let idx2: usize = match input.trim().parse() {
        Ok(n) => n,
        Err(_) => {
            println!("Invalid input");
            return;
        }
    };
    
    let scenes = SCENES.lock().unwrap();
    if idx1 >= scenes.len() || idx2 >= scenes.len() {
        println!("Invalid scene index");
        return;
    }
    
    if let (Some(ref sc1), Some(ref sc2)) = (&scenes[idx1], &scenes[idx2]) {
        println!("\nScene 1: {} ({} shapes)", sc1.name(), sc1.shape_count());
        sc1.list_shapes();
        
        println!("\nScene 2: {} ({} shapes)", sc2.name(), sc2.shape_count());
        sc2.list_shapes();
        
        if Scene::equals(sc1, sc2) {
            println!("\nResult: Scenes are EQUAL (1:1 correspondence)");
        } else {
            println!("\nResult: Scenes are NOT EQUAL");
        }
    }
}

fn delete_scene() {
    let scenes = SCENES.lock().unwrap();
    if scenes.is_empty() {
        println!("No scenes available");
        return;
    }
    drop(scenes);
    
    print!("Select scene to delete (0-{}): ", SCENES.lock().unwrap().len() - 1);
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    if io::stdin().lock().read_line(&mut input).is_err() {
        println!("Invalid input");
        return;
    }
    
    let scene_idx: usize = match input.trim().parse() {
        Ok(n) => n,
        Err(_) => {
            println!("Invalid input");
            return;
        }
    };
    
    let mut scenes = SCENES.lock().unwrap();
    if scene_idx >= scenes.len() {
        println!("Invalid scene index");
        return;
    }
    
    scenes.remove(scene_idx);
    println!("Scene deleted");
}

fn main() {
    println!("╔════════════════════════════════════════╗");
    println!("║  ASCII ART DRAWING APPLICATION        ║");
    println!("║  Child-Friendly Shape Editor           ║");
    println!("╚════════════════════════════════════════╝");
    
    ShapeManager::init();
    
    loop {
        print_menu();
        
        let mut input = String::new();
        if io::stdin().lock().read_line(&mut input).is_err() {
            break;
        }
        
        let choice: i32 = match input.trim().parse() {
            Ok(n) => n,
            Err(_) => {
                println!("Invalid input");
                continue;
            }
        };
        
        match choice {
            1 => view_all_shapes(),
            2 => create_new_scene(),
            3 => add_shape_to_scene(),
            4 => remove_shape_from_scene(),
            5 => view_scene(),
            6 => list_all_scenes(),
            7 => save_scene_to_file(),
            8 => load_scene_from_file(),
            9 => compare_shapes(),
            10 => compare_scenes(),
            11 => delete_scene(),
            12 => {
                println!("\nCleaning up and exiting...");
                {
                    let mut scenes = SCENES.lock().unwrap();
                    scenes.clear();
                }
                ShapeManager::cleanup();
                println!("Goodbye!");
                return;
            }
            _ => println!("Invalid choice"),
        }
    }
    
    {
        let mut scenes = SCENES.lock().unwrap();
        scenes.clear();
    }
    ShapeManager::cleanup();
}
