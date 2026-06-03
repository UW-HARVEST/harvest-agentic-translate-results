// main.rs - Translation of main.c to Rust

mod scene;
mod shape;

use std::io::{self, BufRead, Write};

use scene::{
    scene_add_shape, scene_create, scene_destroy, scene_equals, scene_list_shapes, scene_load,
    scene_print, scene_remove_shape, scene_save, Scene, MAX_SCENE_NAME,
};
use shape::{
    shape_equals, shape_get, shape_get_by_index, shape_manager_cleanup, shape_manager_init,
    shape_print, shape_type_name, Shape, ShapeType, SHAPE_COUNT,
};

const MAX_SCENES: usize = 10;

struct AppState {
    scenes: Vec<Box<Scene>>,
}

impl AppState {
    fn new() -> Self {
        AppState { scenes: Vec::with_capacity(MAX_SCENES) }
    }

    fn scene_count(&self) -> usize {
        self.scenes.len()
    }
}

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
    let _ = io::stdout().flush();
}

fn read_line() -> Option<String> {
    let stdin = io::stdin();
    let mut line = String::new();
    let n = stdin.lock().read_line(&mut line).ok()?;
    if n == 0 {
        return None;
    }
    // Trim trailing newline
    while line.ends_with('\n') || line.ends_with('\r') {
        line.pop();
    }
    Some(line)
}

fn read_int_line() -> Option<i32> {
    let line = read_line()?;
    line.trim().parse::<i32>().ok()
}

fn view_all_shapes() {
    println!("\n=== Available Shapes ===");
    for i in 0..SHAPE_COUNT {
        print!("\n{}. ", i + 1);
        shape_print(shape_get_by_index(i as i32));
    }
}

fn create_new_scene(state: &mut AppState) {
    if state.scene_count() >= MAX_SCENES {
        println!("Error: Maximum scenes reached");
        return;
    }

    print!("Enter scene name: ");
    let _ = io::stdout().flush();
    let name = match read_line() {
        Some(n) => n,
        None => return,
    };

    // Truncate if needed
    let max_len = MAX_SCENE_NAME - 1;
    let name_trunc: String = if name.len() > max_len {
        let mut idx = max_len;
        while idx > 0 && !name.is_char_boundary(idx) {
            idx -= 1;
        }
        name[..idx].to_string()
    } else {
        name.clone()
    };

    match scene_create(Some(&name_trunc)) {
        Some(scene) => {
            let idx = state.scene_count();
            println!("Scene '{}' created (index {})", name_trunc, idx);
            state.scenes.push(scene);
        }
        None => {
            println!("Error creating scene");
        }
    }
}

fn add_shape_to_scene(state: &mut AppState) {
    if state.scene_count() == 0 {
        println!("No scenes available. Create a scene first.");
        return;
    }

    print!("Select scene (0-{}): ", state.scene_count() - 1);
    let _ = io::stdout().flush();
    let scene_idx = match read_int_line() {
        Some(n) => n,
        None => {
            println!("Invalid input");
            return;
        }
    };

    if scene_idx < 0 || scene_idx as usize >= state.scene_count() {
        println!("Invalid scene index");
        return;
    }

    println!("\nSelect shape to add:");
    for i in 0..SHAPE_COUNT {
        if let Some(t) = ShapeType::from_i32(i as i32) {
            println!("{}. {}", i, shape_type_name(t));
        }
    }
    print!("Choice: ");
    let _ = io::stdout().flush();

    let shape_type_val = match read_int_line() {
        Some(n) => n,
        None => {
            println!("Invalid input");
            return;
        }
    };

    if shape_type_val < 0 || shape_type_val as usize >= SHAPE_COUNT {
        println!("Invalid shape type");
        return;
    }

    let shape_opt = shape_get_by_index(shape_type_val);
    let scene = &mut state.scenes[scene_idx as usize];
    if scene_add_shape(scene, shape_opt) == 0 {
        if let Some(shape) = shape_opt {
            let ptr = shape as *const Shape;
            println!(
                "Shape '{}' added to scene (reusing singleton at {:p})",
                shape.name, ptr
            );
        }
    } else {
        println!("Error adding shape");
    }
}

fn remove_shape_from_scene(state: &mut AppState) {
    if state.scene_count() == 0 {
        println!("No scenes available");
        return;
    }

    print!("Select scene (0-{}): ", state.scene_count() - 1);
    let _ = io::stdout().flush();
    let scene_idx = match read_int_line() {
        Some(n) => n,
        None => {
            println!("Invalid input");
            return;
        }
    };

    if scene_idx < 0 || scene_idx as usize >= state.scene_count() {
        println!("Invalid scene index");
        return;
    }

    let count = state.scenes[scene_idx as usize].shape_count;
    scene_list_shapes(Some(&state.scenes[scene_idx as usize]));

    if count == 0 {
        println!("Scene is empty");
        return;
    }

    print!("Select shape to remove (1-{}): ", count);
    let _ = io::stdout().flush();
    let shape_idx = match read_int_line() {
        Some(n) => n,
        None => {
            println!("Invalid input");
            return;
        }
    };

    if scene_remove_shape(&mut state.scenes[scene_idx as usize], shape_idx - 1) == 0 {
        println!("Shape removed");
    } else {
        println!("Error removing shape");
    }
}

fn view_scene(state: &AppState) {
    if state.scene_count() == 0 {
        println!("No scenes available");
        return;
    }

    print!("Select scene (0-{}): ", state.scene_count() - 1);
    let _ = io::stdout().flush();
    let scene_idx = match read_int_line() {
        Some(n) => n,
        None => {
            println!("Invalid input");
            return;
        }
    };

    if scene_idx < 0 || scene_idx as usize >= state.scene_count() {
        println!("Invalid scene index");
        return;
    }

    scene_print(Some(&state.scenes[scene_idx as usize]));
}

fn list_all_scenes(state: &AppState) {
    println!("\n=== All Scenes ===");
    if state.scene_count() == 0 {
        println!("No scenes created yet");
        return;
    }

    for (i, scene) in state.scenes.iter().enumerate() {
        println!("{}. {} ({} shapes)", i, scene.name, scene.shape_count);
    }
}

fn save_scene_to_file(state: &AppState) {
    if state.scene_count() == 0 {
        println!("No scenes available");
        return;
    }

    print!("Select scene (0-{}): ", state.scene_count() - 1);
    let _ = io::stdout().flush();
    let scene_idx = match read_int_line() {
        Some(n) => n,
        None => {
            println!("Invalid input");
            return;
        }
    };

    if scene_idx < 0 || scene_idx as usize >= state.scene_count() {
        println!("Invalid scene index");
        return;
    }

    print!("Enter filename: ");
    let _ = io::stdout().flush();
    let filename = match read_line() {
        Some(f) => f,
        None => return,
    };

    scene_save(Some(&state.scenes[scene_idx as usize]), Some(&filename));
}

fn load_scene_from_file(state: &mut AppState) {
    if state.scene_count() >= MAX_SCENES {
        println!("Error: Maximum scenes reached");
        return;
    }

    print!("Enter filename: ");
    let _ = io::stdout().flush();
    let filename = match read_line() {
        Some(f) => f,
        None => return,
    };

    if let Some(scene) = scene_load(Some(&filename)) {
        state.scenes.push(scene);
        println!("Scene loaded (index {})", state.scene_count() - 1);
    }
}

fn compare_shapes() {
    println!("\nSelect first shape (0-{}):", SHAPE_COUNT - 1);
    for i in 0..SHAPE_COUNT {
        if let Some(t) = ShapeType::from_i32(i as i32) {
            println!("{}. {}", i, shape_type_name(t));
        }
    }
    print!("Choice: ");
    let _ = io::stdout().flush();

    let type1 = match read_int_line() {
        Some(n) => n,
        None => {
            println!("Invalid input");
            return;
        }
    };

    print!("\nSelect second shape (0-{}): ", SHAPE_COUNT - 1);
    let _ = io::stdout().flush();
    let type2 = match read_int_line() {
        Some(n) => n,
        None => {
            println!("Invalid input");
            return;
        }
    };

    if type1 < 0
        || (type1 as usize) >= SHAPE_COUNT
        || type2 < 0
        || (type2 as usize) >= SHAPE_COUNT
    {
        println!("Invalid shape type");
        return;
    }

    let s1 = shape_get_by_index(type1);
    let s2 = shape_get_by_index(type2);

    if let Some(sh) = s1 {
        let ptr = sh as *const Shape;
        println!("\nShape 1: {} (ptr: {:p})", sh.name, ptr);
    }
    if let Some(sh) = s2 {
        let ptr = sh as *const Shape;
        println!("Shape 2: {} (ptr: {:p})", sh.name, ptr);
    }

    let same_ptr = match (s1, s2) {
        (Some(a), Some(b)) => std::ptr::eq(a, b) as i32,
        _ => 0,
    };
    println!("Comparison of pointers: {}", same_ptr);

    if shape_equals(s1, s2) != 0 {
        println!("Result: Shapes are EQUAL (same instance)");
    } else {
        println!("Result: Shapes are NOT EQUAL (different instances)");
    }
}

fn compare_scenes(state: &AppState) {
    if state.scene_count() < 2 {
        println!("Need at least 2 scenes to compare");
        return;
    }

    print!("Select first scene (0-{}): ", state.scene_count() - 1);
    let _ = io::stdout().flush();
    let idx1 = match read_int_line() {
        Some(n) => n,
        None => {
            println!("Invalid input");
            return;
        }
    };

    print!("Select second scene (0-{}): ", state.scene_count() - 1);
    let _ = io::stdout().flush();
    let idx2 = match read_int_line() {
        Some(n) => n,
        None => {
            println!("Invalid input");
            return;
        }
    };

    if idx1 < 0
        || (idx1 as usize) >= state.scene_count()
        || idx2 < 0
        || (idx2 as usize) >= state.scene_count()
    {
        println!("Invalid scene index");
        return;
    }

    let sc1 = &state.scenes[idx1 as usize];
    let sc2 = &state.scenes[idx2 as usize];

    println!("\nScene 1: {} ({} shapes)", sc1.name, sc1.shape_count);
    scene_list_shapes(Some(sc1));

    println!("\nScene 2: {} ({} shapes)", sc2.name, sc2.shape_count);
    scene_list_shapes(Some(sc2));

    if scene_equals(Some(sc1), Some(sc2)) != 0 {
        println!("\nResult: Scenes are EQUAL (1:1 correspondence)");
    } else {
        println!("\nResult: Scenes are NOT EQUAL");
    }
}

fn delete_scene(state: &mut AppState) {
    if state.scene_count() == 0 {
        println!("No scenes available");
        return;
    }

    print!("Select scene to delete (0-{}): ", state.scene_count() - 1);
    let _ = io::stdout().flush();
    let scene_idx = match read_int_line() {
        Some(n) => n,
        None => {
            println!("Invalid input");
            return;
        }
    };

    if scene_idx < 0 || scene_idx as usize >= state.scene_count() {
        println!("Invalid scene index");
        return;
    }

    let removed = state.scenes.remove(scene_idx as usize);
    scene_destroy(removed);
    println!("Scene deleted");
}

fn main() {
    println!("╔════════════════════════════════════════╗");
    println!("║  ASCII ART DRAWING APPLICATION        ║");
    println!("║  Child-Friendly Shape Editor           ║");
    println!("╚════════════════════════════════════════╝");

    shape_manager_init();

    let mut state = AppState::new();

    loop {
        print_menu();

        let line = match read_line() {
            Some(l) => l,
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
            1 => view_all_shapes(),
            2 => create_new_scene(&mut state),
            3 => add_shape_to_scene(&mut state),
            4 => remove_shape_from_scene(&mut state),
            5 => view_scene(&state),
            6 => list_all_scenes(&state),
            7 => save_scene_to_file(&state),
            8 => load_scene_from_file(&mut state),
            9 => compare_shapes(),
            10 => compare_scenes(&state),
            11 => delete_scene(&mut state),
            12 => {
                println!("\nCleaning up and exiting...");
                while let Some(s) = state.scenes.pop() {
                    scene_destroy(s);
                }
                shape_manager_cleanup();
                println!("Goodbye!");
                return;
            }
            _ => println!("Invalid choice"),
        }
    }

    while let Some(s) = state.scenes.pop() {
        scene_destroy(s);
    }
    shape_manager_cleanup();

    // Suppress unused warning
    let _ = shape_get(ShapeType::Tree);
}
