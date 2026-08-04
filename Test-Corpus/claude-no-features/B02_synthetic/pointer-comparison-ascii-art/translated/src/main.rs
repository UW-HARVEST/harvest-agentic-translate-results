// main.rs — translation of c_src/src/main.c

mod io_util;
mod scene;
mod shape;

use std::io::Write;

use io_util::{flush_stdout, strip_newline, StdinReader};
use scene::{scene_create, scene_destroy_noop, Scene};

const MAX_SCENES: usize = 10;

struct App {
    scenes: Vec<Option<Scene>>, // length always equals scene_count valid entries
    reader: StdinReader,
}

impl App {
    fn new() -> Self {
        App {
            scenes: Vec::with_capacity(MAX_SCENES),
            reader: StdinReader::new(),
        }
    }

    fn scene_count(&self) -> usize {
        self.scenes.len()
    }
}

fn print_menu() {
    print!("\n");
    print!("=========================================\n");
    print!("  ASCII ART DRAWING APPLICATION\n");
    print!("=========================================\n");
    print!("1. View all available shapes\n");
    print!("2. Create new scene\n");
    print!("3. Add shape to scene\n");
    print!("4. Remove shape from scene\n");
    print!("5. View scene\n");
    print!("6. List all scenes\n");
    print!("7. Save scene\n");
    print!("8. Load scene\n");
    print!("9. Compare two shapes\n");
    print!("10. Compare two scenes\n");
    print!("11. Delete scene\n");
    print!("12. Exit\n");
    print!("=========================================\n");
    print!("Choice: ");
    flush_stdout();
}

fn view_all_shapes() {
    println!();
    println!("=== Available Shapes ===");
    for i in 0..shape::SHAPE_COUNT {
        println!();
        print!("{}. ", i + 1);
        let p = shape::shape_get_ptr(i);
        shape::shape_print(p);
    }
}

fn create_new_scene(app: &mut App) {
    if app.scene_count() >= MAX_SCENES {
        println!("Error: Maximum scenes reached");
        return;
    }
    print!("Enter scene name: ");
    flush_stdout();
    let line = match app.reader.fgets(scene::MAX_SCENE_NAME) {
        Some(l) => l,
        None => return,
    };
    let stripped = strip_newline(&line);
    let name = String::from_utf8_lossy(stripped).into_owned();

    let new_scene = scene_create(&name);
    app.scenes.push(Some(new_scene));
    let idx = app.scenes.len() - 1;
    println!("Scene '{}' created (index {})", name, idx);
}

fn read_int_with_consume(app: &mut App) -> Option<i32> {
    let v = app.reader.scanf_int();
    // C: while (getchar() != '\n');
    app.reader.consume_until_newline();
    v
}

fn add_shape_to_scene(app: &mut App) {
    if app.scene_count() == 0 {
        println!("No scenes available. Create a scene first.");
        return;
    }
    print!("Select scene (0-{}): ", app.scene_count() - 1);
    flush_stdout();
    let scene_idx = match read_int_with_consume(app) {
        Some(v) => v,
        None => {
            println!("Invalid input");
            return;
        }
    };
    if scene_idx < 0 || scene_idx as usize >= app.scene_count() {
        println!("Invalid scene index");
        return;
    }

    println!();
    println!("Select shape to add:");
    for i in 0..shape::SHAPE_COUNT {
        println!("{}. {}", i, shape::shape_type_name(i));
    }
    print!("Choice: ");
    flush_stdout();

    let shape_type = match read_int_with_consume(app) {
        Some(v) => v,
        None => {
            println!("Invalid input");
            return;
        }
    };
    if shape_type < 0 || shape_type >= shape::SHAPE_COUNT {
        println!("Invalid shape type");
        return;
    }

    let shape_ptr = shape::shape_get_ptr(shape_type);
    let scene = app.scenes[scene_idx as usize].as_mut().unwrap();
    if scene::scene_add_shape(scene, shape_ptr) == 0 {
        let name = shape::shape_name(shape_ptr);
        let p = shape::fmt_ptr(shape_ptr);
        println!("Shape '{}' added to scene (reusing singleton at {})", name, p);
    } else {
        println!("Error adding shape");
    }
}

fn remove_shape_from_scene(app: &mut App) {
    if app.scene_count() == 0 {
        println!("No scenes available");
        return;
    }
    print!("Select scene (0-{}): ", app.scene_count() - 1);
    flush_stdout();
    let scene_idx = match read_int_with_consume(app) {
        Some(v) => v,
        None => {
            println!("Invalid input");
            return;
        }
    };
    if scene_idx < 0 || scene_idx as usize >= app.scene_count() {
        println!("Invalid scene index");
        return;
    }

    {
        let scene = app.scenes[scene_idx as usize].as_ref().unwrap();
        scene::scene_list_shapes(scene);
        if scene.shapes.is_empty() {
            println!("Scene is empty");
            return;
        }
    }

    let scene_len = app.scenes[scene_idx as usize]
        .as_ref()
        .unwrap()
        .shapes
        .len();
    print!("Select shape to remove (1-{}): ", scene_len);
    flush_stdout();

    let shape_idx = match read_int_with_consume(app) {
        Some(v) => v,
        None => {
            println!("Invalid input");
            return;
        }
    };

    let scene = app.scenes[scene_idx as usize].as_mut().unwrap();
    if scene::scene_remove_shape(scene, shape_idx - 1) == 0 {
        println!("Shape removed");
    } else {
        println!("Error removing shape");
    }
}

fn view_scene(app: &mut App) {
    if app.scene_count() == 0 {
        println!("No scenes available");
        return;
    }
    print!("Select scene (0-{}): ", app.scene_count() - 1);
    flush_stdout();
    let scene_idx = match read_int_with_consume(app) {
        Some(v) => v,
        None => {
            println!("Invalid input");
            return;
        }
    };
    if scene_idx < 0 || scene_idx as usize >= app.scene_count() {
        println!("Invalid scene index");
        return;
    }
    let scene = app.scenes[scene_idx as usize].as_ref().unwrap();
    scene::scene_print(scene);
}

fn list_all_scenes(app: &App) {
    println!();
    println!("=== All Scenes ===");
    if app.scene_count() == 0 {
        println!("No scenes created yet");
        return;
    }
    for (i, s) in app.scenes.iter().enumerate() {
        let s = s.as_ref().unwrap();
        println!("{}. {} ({} shapes)", i, s.name, s.shapes.len());
    }
}

fn save_scene_to_file(app: &mut App) {
    if app.scene_count() == 0 {
        println!("No scenes available");
        return;
    }
    print!("Select scene (0-{}): ", app.scene_count() - 1);
    flush_stdout();
    let scene_idx = match read_int_with_consume(app) {
        Some(v) => v,
        None => {
            println!("Invalid input");
            return;
        }
    };
    if scene_idx < 0 || scene_idx as usize >= app.scene_count() {
        println!("Invalid scene index");
        return;
    }
    print!("Enter filename: ");
    flush_stdout();
    let line = match app.reader.fgets(256) {
        Some(l) => l,
        None => return,
    };
    let stripped = strip_newline(&line);
    let filename = String::from_utf8_lossy(stripped).into_owned();
    let scene = app.scenes[scene_idx as usize].as_ref().unwrap();
    scene::scene_save(scene, &filename);
}

fn load_scene_from_file(app: &mut App) {
    if app.scene_count() >= MAX_SCENES {
        println!("Error: Maximum scenes reached");
        return;
    }
    print!("Enter filename: ");
    flush_stdout();
    let line = match app.reader.fgets(256) {
        Some(l) => l,
        None => return,
    };
    let stripped = strip_newline(&line);
    let filename = String::from_utf8_lossy(stripped).into_owned();
    if let Some(scene) = scene::scene_load(&filename) {
        app.scenes.push(Some(scene));
        let new_idx = app.scenes.len() - 1;
        println!("Scene loaded (index {})", new_idx);
    }
}

fn compare_shapes(app: &mut App) {
    println!();
    println!("Select first shape (0-{}):", shape::SHAPE_COUNT - 1);
    for i in 0..shape::SHAPE_COUNT {
        println!("{}. {}", i, shape::shape_type_name(i));
    }
    print!("Choice: ");
    flush_stdout();
    let type1 = match read_int_with_consume(app) {
        Some(v) => v,
        None => {
            println!("Invalid input");
            return;
        }
    };
    println!();
    print!("Select second shape (0-{}): ", shape::SHAPE_COUNT - 1);
    flush_stdout();
    let type2 = match read_int_with_consume(app) {
        Some(v) => v,
        None => {
            println!("Invalid input");
            return;
        }
    };
    if type1 < 0 || type1 >= shape::SHAPE_COUNT || type2 < 0 || type2 >= shape::SHAPE_COUNT {
        println!("Invalid shape type");
        return;
    }
    let p1 = shape::shape_get_ptr(type1);
    let p2 = shape::shape_get_ptr(type2);
    println!();
    println!("Shape 1: {} (ptr: {})", shape::shape_name(p1), shape::fmt_ptr(p1));
    println!("Shape 2: {} (ptr: {})", shape::shape_name(p2), shape::fmt_ptr(p2));
    let cmp = if p1 == p2 { 1 } else { 0 };
    println!("Comparison of pointers: {}", cmp);
    if shape::shape_equals(p1, p2) {
        println!("Result: Shapes are EQUAL (same instance)");
    } else {
        println!("Result: Shapes are NOT EQUAL (different instances)");
    }
}

fn compare_scenes(app: &mut App) {
    if app.scene_count() < 2 {
        println!("Need at least 2 scenes to compare");
        return;
    }
    print!("Select first scene (0-{}): ", app.scene_count() - 1);
    flush_stdout();
    let idx1 = match read_int_with_consume(app) {
        Some(v) => v,
        None => {
            println!("Invalid input");
            return;
        }
    };
    print!("Select second scene (0-{}): ", app.scene_count() - 1);
    flush_stdout();
    let idx2 = match read_int_with_consume(app) {
        Some(v) => v,
        None => {
            println!("Invalid input");
            return;
        }
    };
    if idx1 < 0
        || idx1 as usize >= app.scene_count()
        || idx2 < 0
        || idx2 as usize >= app.scene_count()
    {
        println!("Invalid scene index");
        return;
    }
    let sc1 = app.scenes[idx1 as usize].as_ref().unwrap();
    let sc2 = app.scenes[idx2 as usize].as_ref().unwrap();

    println!();
    println!("Scene 1: {} ({} shapes)", sc1.name, sc1.shapes.len());
    scene::scene_list_shapes(sc1);

    println!();
    println!("Scene 2: {} ({} shapes)", sc2.name, sc2.shapes.len());
    scene::scene_list_shapes(sc2);

    if scene::scene_equals(sc1, sc2) {
        println!();
        println!("Result: Scenes are EQUAL (1:1 correspondence)");
    } else {
        println!();
        println!("Result: Scenes are NOT EQUAL");
    }
}

fn delete_scene(app: &mut App) {
    if app.scene_count() == 0 {
        println!("No scenes available");
        return;
    }
    print!("Select scene to delete (0-{}): ", app.scene_count() - 1);
    flush_stdout();
    let scene_idx = match read_int_with_consume(app) {
        Some(v) => v,
        None => {
            println!("Invalid input");
            return;
        }
    };
    if scene_idx < 0 || scene_idx as usize >= app.scene_count() {
        println!("Invalid scene index");
        return;
    }
    let removed = app.scenes.remove(scene_idx as usize);
    scene_destroy_noop(removed);
    println!("Scene deleted");
}

fn main() {
    println!("╔════════════════════════════════════════╗");
    println!("║  ASCII ART DRAWING APPLICATION        ║");
    println!("║  Child-Friendly Shape Editor           ║");
    println!("╚════════════════════════════════════════╝");

    shape::shape_manager_init();

    let mut app = App::new();

    loop {
        print_menu();

        let line = match app.reader.fgets(256) {
            Some(l) => l,
            None => break,
        };
        // sscanf("%d", ...) — skip leading whitespace, parse int.
        let s = String::from_utf8_lossy(&line);
        let trimmed = s.trim_start();
        let mut parsed: Option<i32> = None;
        // Find the substring of digits (with optional leading sign).
        let bytes = trimmed.as_bytes();
        let mut idx = 0;
        if !bytes.is_empty() && (bytes[0] == b'-' || bytes[0] == b'+') {
            idx = 1;
        }
        let digits_start = idx;
        while idx < bytes.len() && bytes[idx].is_ascii_digit() {
            idx += 1;
        }
        if idx > digits_start {
            // Include the sign byte if present.
            let begin = if digits_start > 0 { digits_start - 1 } else { 0 };
            if let Ok(num) = std::str::from_utf8(&bytes[begin..idx])
                .unwrap_or("")
                .parse::<i64>()
            {
                parsed = Some(num as i32);
            }
        }

        let choice = match parsed {
            Some(c) => c,
            None => {
                println!("Invalid input");
                continue;
            }
        };

        match choice {
            1 => view_all_shapes(),
            2 => create_new_scene(&mut app),
            3 => add_shape_to_scene(&mut app),
            4 => remove_shape_from_scene(&mut app),
            5 => view_scene(&mut app),
            6 => list_all_scenes(&app),
            7 => save_scene_to_file(&mut app),
            8 => load_scene_from_file(&mut app),
            9 => compare_shapes(&mut app),
            10 => compare_scenes(&mut app),
            11 => delete_scene(&mut app),
            12 => {
                println!();
                println!("Cleaning up and exiting...");
                // Drop all scenes (they don't free shapes, just like in C).
                app.scenes.clear();
                shape::shape_manager_cleanup();
                println!("Goodbye!");
                // Ensure stdout flushes before process exit.
                let _ = std::io::stdout().flush();
                std::process::exit(0);
            }
            _ => {
                println!("Invalid choice");
            }
        }
    }

    // Cleanup on EOF.
    app.scenes.clear();
    shape::shape_manager_cleanup();
}
