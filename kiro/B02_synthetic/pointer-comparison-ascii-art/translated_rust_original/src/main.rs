mod shape;
mod scene;

use shape::*;
use scene::*;
use std::io::{self, Write, BufRead};

const MAX_SCENES: usize = 10;

static mut SCENES: [*mut Scene; MAX_SCENES] = [std::ptr::null_mut(); MAX_SCENES];
static mut SCENE_COUNT: i32 = 0;

fn read_line() -> Option<String> {
    let mut buf = String::new();
    let stdin = io::stdin();
    match stdin.lock().read_line(&mut buf) {
        Ok(0) => None,
        Ok(_) => Some(buf),
        Err(_) => None,
    }
}

fn read_int() -> Option<i32> {
    io::stdout().flush().ok();
    let line = read_line()?;
    line.trim().parse().ok()
}

fn read_fgets_string() -> Option<String> {
    io::stdout().flush().ok();
    let line = read_line()?;
    let s = line.trim_end_matches('\n').to_string();
    Some(s)
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
    io::stdout().flush().ok();
}

fn view_all_shapes() {
    print!("\n=== Available Shapes ===\n");
    for i in 0..SHAPE_COUNT {
        print!("\n{}. ", i + 1);
        shape_print(shape_get(i));
    }
}

fn create_new_scene() {
    let scene_count = unsafe { SCENE_COUNT };
    if scene_count >= MAX_SCENES as i32 {
        print!("Error: Maximum scenes reached\n");
        return;
    }
    print!("Enter scene name: ");
    let name = match read_fgets_string() {
        Some(n) => n,
        None => return,
    };
    let scene = scene_create(&name);
    if !scene.is_null() {
        print!("Scene '{}' created (index {})\n", name, scene_count);
        unsafe {
            SCENES[scene_count as usize] = scene;
            SCENE_COUNT += 1;
        }
    } else {
        print!("Error creating scene\n");
    }
}

fn add_shape_to_scene() {
    let scene_count = unsafe { SCENE_COUNT };
    if scene_count == 0 {
        print!("No scenes available. Create a scene first.\n");
        return;
    }
    print!("Select scene (0-{}): ", scene_count - 1);
    let scene_idx = match read_int() {
        Some(v) => v,
        None => { print!("Invalid input\n"); return; }
    };
    if scene_idx < 0 || scene_idx >= scene_count {
        print!("Invalid scene index\n");
        return;
    }
    print!("\nSelect shape to add:\n");
    for i in 0..SHAPE_COUNT {
        print!("{}. {}\n", i, shape_type_name(i));
    }
    print!("Choice: ");
    let shape_type = match read_int() {
        Some(v) => v,
        None => { print!("Invalid input\n"); return; }
    };
    if shape_type < 0 || shape_type >= SHAPE_COUNT as i32 {
        print!("Invalid shape type\n");
        return;
    }
    let shape = shape_get(shape_type as usize);
    let scene = unsafe { SCENES[scene_idx as usize] };
    if scene_add_shape(scene, shape) == 0 {
        let s = unsafe { &*shape };
        print!("Shape '{}' added to scene (reusing singleton at {:p})\n", s.name, shape);
    } else {
        print!("Error adding shape\n");
    }
}

fn remove_shape_from_scene() {
    let scene_count = unsafe { SCENE_COUNT };
    if scene_count == 0 {
        print!("No scenes available\n");
        return;
    }
    print!("Select scene (0-{}): ", scene_count - 1);
    let scene_idx = match read_int() {
        Some(v) => v,
        None => { print!("Invalid input\n"); return; }
    };
    if scene_idx < 0 || scene_idx >= scene_count {
        print!("Invalid scene index\n");
        return;
    }
    let scene = unsafe { SCENES[scene_idx as usize] };
    scene_list_shapes(scene);
    let sc = unsafe { &*scene };
    if sc.shape_count == 0 {
        print!("Scene is empty\n");
        return;
    }
    print!("Select shape to remove (1-{}): ", sc.shape_count);
    let shape_idx = match read_int() {
        Some(v) => v,
        None => { print!("Invalid input\n"); return; }
    };
    if scene_remove_shape(scene, shape_idx - 1) == 0 {
        print!("Shape removed\n");
    } else {
        print!("Error removing shape\n");
    }
}

fn view_scene() {
    let scene_count = unsafe { SCENE_COUNT };
    if scene_count == 0 {
        print!("No scenes available\n");
        return;
    }
    print!("Select scene (0-{}): ", scene_count - 1);
    let scene_idx = match read_int() {
        Some(v) => v,
        None => { print!("Invalid input\n"); return; }
    };
    if scene_idx < 0 || scene_idx >= scene_count {
        print!("Invalid scene index\n");
        return;
    }
    scene_print(unsafe { SCENES[scene_idx as usize] });
}

fn list_all_scenes() {
    let scene_count = unsafe { SCENE_COUNT };
    print!("\n=== All Scenes ===\n");
    if scene_count == 0 {
        print!("No scenes created yet\n");
        return;
    }
    for i in 0..scene_count as usize {
        let s = unsafe { &*SCENES[i] };
        print!("{}. {} ({} shapes)\n", i, s.name, s.shape_count);
    }
}

fn save_scene_to_file() {
    let scene_count = unsafe { SCENE_COUNT };
    if scene_count == 0 {
        print!("No scenes available\n");
        return;
    }
    print!("Select scene (0-{}): ", scene_count - 1);
    let scene_idx = match read_int() {
        Some(v) => v,
        None => { print!("Invalid input\n"); return; }
    };
    if scene_idx < 0 || scene_idx >= scene_count {
        print!("Invalid scene index\n");
        return;
    }
    print!("Enter filename: ");
    let filename = match read_fgets_string() {
        Some(f) => f,
        None => return,
    };
    scene_save(unsafe { SCENES[scene_idx as usize] }, &filename);
}

fn load_scene_from_file() {
    let scene_count = unsafe { SCENE_COUNT };
    if scene_count >= MAX_SCENES as i32 {
        print!("Error: Maximum scenes reached\n");
        return;
    }
    print!("Enter filename: ");
    let filename = match read_fgets_string() {
        Some(f) => f,
        None => return,
    };
    let scene = scene_load(&filename);
    if !scene.is_null() {
        unsafe {
            SCENES[SCENE_COUNT as usize] = scene;
            SCENE_COUNT += 1;
        }
        print!("Scene loaded (index {})\n", unsafe { SCENE_COUNT } - 1);
    }
}

fn compare_shapes() {
    print!("\nSelect first shape (0-{}):\n", SHAPE_COUNT - 1);
    for i in 0..SHAPE_COUNT {
        print!("{}. {}\n", i, shape_type_name(i));
    }
    print!("Choice: ");
    let type1 = match read_int() {
        Some(v) => v,
        None => { print!("Invalid input\n"); return; }
    };
    print!("\nSelect second shape (0-{}): ", SHAPE_COUNT - 1);
    let type2 = match read_int() {
        Some(v) => v,
        None => { print!("Invalid input\n"); return; }
    };
    if type1 < 0 || type1 >= SHAPE_COUNT as i32 || type2 < 0 || type2 >= SHAPE_COUNT as i32 {
        print!("Invalid shape type\n");
        return;
    }
    let s1 = shape_get(type1 as usize);
    let s2 = shape_get(type2 as usize);
    let n1 = unsafe { &*s1 };
    let n2 = unsafe { &*s2 };
    print!("\nShape 1: {} (ptr: {:p})\n", n1.name, s1);
    print!("Shape 2: {} (ptr: {:p})\n", n2.name, s2);
    print!("Comparison of pointers: {}\n", if s1 == s2 { 1 } else { 0 });
    if shape_equals(s1, s2) {
        print!("Result: Shapes are EQUAL (same instance)\n");
    } else {
        print!("Result: Shapes are NOT EQUAL (different instances)\n");
    }
}

fn compare_scenes() {
    let scene_count = unsafe { SCENE_COUNT };
    if scene_count < 2 {
        print!("Need at least 2 scenes to compare\n");
        return;
    }
    print!("Select first scene (0-{}): ", scene_count - 1);
    let idx1 = match read_int() {
        Some(v) => v,
        None => { print!("Invalid input\n"); return; }
    };
    print!("Select second scene (0-{}): ", scene_count - 1);
    let idx2 = match read_int() {
        Some(v) => v,
        None => { print!("Invalid input\n"); return; }
    };
    if idx1 < 0 || idx1 >= scene_count || idx2 < 0 || idx2 >= scene_count {
        print!("Invalid scene index\n");
        return;
    }
    let sc1 = unsafe { SCENES[idx1 as usize] };
    let sc2 = unsafe { SCENES[idx2 as usize] };
    let s1 = unsafe { &*sc1 };
    let s2 = unsafe { &*sc2 };
    print!("\nScene 1: {} ({} shapes)\n", s1.name, s1.shape_count);
    scene_list_shapes(sc1);
    print!("\nScene 2: {} ({} shapes)\n", s2.name, s2.shape_count);
    scene_list_shapes(sc2);
    if scene_equals(sc1, sc2) {
        print!("\nResult: Scenes are EQUAL (1:1 correspondence)\n");
    } else {
        print!("\nResult: Scenes are NOT EQUAL\n");
    }
}

fn delete_scene() {
    let scene_count = unsafe { SCENE_COUNT };
    if scene_count == 0 {
        print!("No scenes available\n");
        return;
    }
    print!("Select scene to delete (0-{}): ", scene_count - 1);
    let scene_idx = match read_int() {
        Some(v) => v,
        None => { print!("Invalid input\n"); return; }
    };
    if scene_idx < 0 || scene_idx >= scene_count {
        print!("Invalid scene index\n");
        return;
    }
    unsafe {
        scene_destroy(SCENES[scene_idx as usize]);
        for i in scene_idx as usize..(SCENE_COUNT - 1) as usize {
            SCENES[i] = SCENES[i + 1];
        }
        SCENE_COUNT -= 1;
    }
    print!("Scene deleted\n");
}

fn main() {
    print!("\u{2554}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2557}\n");
    print!("\u{2551}  ASCII ART DRAWING APPLICATION        \u{2551}\n");
    print!("\u{2551}  Child-Friendly Shape Editor           \u{2551}\n");
    print!("\u{255a}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255d}\n");

    shape_manager_init();

    loop {
        print_menu();
        let input = match read_line() {
            Some(s) => s,
            None => break,
        };
        let choice: i32 = match input.trim().parse() {
            Ok(v) => v,
            Err(_) => { print!("Invalid input\n"); continue; }
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
                print!("\nCleaning up and exiting...\n");
                unsafe {
                    for i in 0..SCENE_COUNT as usize {
                        scene_destroy(SCENES[i]);
                    }
                }
                shape_manager_cleanup();
                print!("Goodbye!\n");
                return;
            }
            _ => { print!("Invalid choice\n"); }
        }
    }

    // Cleanup on EOF
    unsafe {
        for i in 0..SCENE_COUNT as usize {
            scene_destroy(SCENES[i]);
        }
    }
    shape_manager_cleanup();
}
