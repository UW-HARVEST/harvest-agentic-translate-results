mod scene;
mod shape;

use scene::{
    scene_add_shape, scene_create, scene_destroy, scene_equals, scene_list_shapes, scene_load,
    scene_print, scene_remove_shape, scene_save, SceneT, MAX_SCENE_NAME,
};
use shape::{
    shape_equals, shape_get, shape_manager_cleanup, shape_manager_init, shape_print,
    shape_type_name, ShapeTypeT, SHAPE_COUNT,
};
use std::io::{self, Write};

const MAX_SCENES: usize = 10;

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

fn read_line_trimmed() -> Option<String> {
    let mut input = String::new();
    if io::stdin().read_line(&mut input).ok()? == 0 {
        return None;
    }
    while input.ends_with('\n') || input.ends_with('\r') {
        input.pop();
    }
    Some(input)
}

fn read_i32() -> Option<i32> {
    read_line_trimmed()?.trim().parse().ok()
}

fn view_all_shapes() {
    println!("\n=== Available Shapes ===");
    for (i, shape_type) in ShapeTypeT::all().iter().enumerate() {
        println!("\n{}. ", i + 1);
        shape_print(shape_get(*shape_type));
    }
}

fn create_new_scene(scenes: &mut Vec<Box<SceneT>>) {
    if scenes.len() >= MAX_SCENES {
        println!("Error: Maximum scenes reached");
        return;
    }

    print!("Enter scene name: ");
    let _ = io::stdout().flush();
    let Some(mut name) = read_line_trimmed() else {
        return;
    };
    name = name.chars().take(MAX_SCENE_NAME - 1).collect();

    if let Some(scene) = scene_create(Some(&name)) {
        println!("Scene '{}' created (index {})", name, scenes.len());
        scenes.push(scene);
    } else {
        println!("Error creating scene");
    }
}

fn add_shape_to_scene(scenes: &mut [Box<SceneT>]) {
    if scenes.is_empty() {
        println!("No scenes available. Create a scene first.");
        return;
    }

    print!("Select scene (0-{}): ", scenes.len() - 1);
    let _ = io::stdout().flush();
    let Some(scene_idx) = read_i32() else {
        println!("Invalid input");
        return;
    };

    if scene_idx < 0 || scene_idx as usize >= scenes.len() {
        println!("Invalid scene index");
        return;
    }

    println!("\nSelect shape to add:");
    for i in 0..SHAPE_COUNT {
        let shape_type = ShapeTypeT::from_i32(i as i32).unwrap();
        println!("{}. {}", i, shape_type_name(shape_type));
    }
    print!("Choice: ");
    let _ = io::stdout().flush();

    let Some(shape_type_num) = read_i32() else {
        println!("Invalid input");
        return;
    };

    let Some(shape_type) = ShapeTypeT::from_i32(shape_type_num) else {
        println!("Invalid shape type");
        return;
    };

    let Some(shape) = shape_get(shape_type) else {
        println!("Error adding shape");
        return;
    };

    if scene_add_shape(&mut scenes[scene_idx as usize], shape) == 0 {
        println!(
            "Shape '{}' added to scene (reusing singleton at {:p})",
            shape.name, shape
        );
    } else {
        println!("Error adding shape");
    }
}

fn remove_shape_from_scene(scenes: &mut [Box<SceneT>]) {
    if scenes.is_empty() {
        println!("No scenes available");
        return;
    }

    print!("Select scene (0-{}): ", scenes.len() - 1);
    let _ = io::stdout().flush();
    let Some(scene_idx) = read_i32() else {
        println!("Invalid input");
        return;
    };

    if scene_idx < 0 || scene_idx as usize >= scenes.len() {
        println!("Invalid scene index");
        return;
    }

    let scene = &scenes[scene_idx as usize];
    scene_list_shapes(Some(scene));

    if scene.shapes.is_empty() {
        println!("Scene is empty");
        return;
    }

    print!("Select shape to remove (1-{}): ", scene.shapes.len());
    let _ = io::stdout().flush();
    let Some(shape_idx) = read_i32() else {
        println!("Invalid input");
        return;
    };

    if scene_remove_shape(&mut scenes[scene_idx as usize], shape_idx - 1) == 0 {
        println!("Shape removed");
    } else {
        println!("Error removing shape");
    }
}

fn view_scene(scenes: &[Box<SceneT>]) {
    if scenes.is_empty() {
        println!("No scenes available");
        return;
    }

    print!("Select scene (0-{}): ", scenes.len() - 1);
    let _ = io::stdout().flush();
    let Some(scene_idx) = read_i32() else {
        println!("Invalid input");
        return;
    };

    if scene_idx < 0 || scene_idx as usize >= scenes.len() {
        println!("Invalid scene index");
        return;
    }

    scene_print(Some(&scenes[scene_idx as usize]));
}

fn list_all_scenes(scenes: &[Box<SceneT>]) {
    println!("\n=== All Scenes ===");
    if scenes.is_empty() {
        println!("No scenes created yet");
        return;
    }

    for (i, scene) in scenes.iter().enumerate() {
        println!("{}. {} ({} shapes)", i, scene.name, scene.shapes.len());
    }
}

fn save_scene_to_file(scenes: &[Box<SceneT>]) {
    if scenes.is_empty() {
        println!("No scenes available");
        return;
    }

    print!("Select scene (0-{}): ", scenes.len() - 1);
    let _ = io::stdout().flush();
    let Some(scene_idx) = read_i32() else {
        println!("Invalid input");
        return;
    };

    if scene_idx < 0 || scene_idx as usize >= scenes.len() {
        println!("Invalid scene index");
        return;
    }

    print!("Enter filename: ");
    let _ = io::stdout().flush();
    let Some(filename) = read_line_trimmed() else {
        return;
    };

    let _ = scene_save(Some(&scenes[scene_idx as usize]), Some(&filename));
}

fn load_scene_from_file(scenes: &mut Vec<Box<SceneT>>) {
    if scenes.len() >= MAX_SCENES {
        println!("Error: Maximum scenes reached");
        return;
    }

    print!("Enter filename: ");
    let _ = io::stdout().flush();
    let Some(filename) = read_line_trimmed() else {
        return;
    };

    if let Some(scene) = scene_load(Some(&filename)) {
        scenes.push(scene);
        println!("Scene loaded (index {})", scenes.len() - 1);
    }
}

fn compare_shapes() {
    println!("\nSelect first shape (0-{}):", SHAPE_COUNT - 1);
    for i in 0..SHAPE_COUNT {
        let shape_type = ShapeTypeT::from_i32(i as i32).unwrap();
        println!("{}. {}", i, shape_type_name(shape_type));
    }
    print!("Choice: ");
    let _ = io::stdout().flush();

    let Some(type1) = read_i32() else {
        println!("Invalid input");
        return;
    };

    print!("\nSelect second shape (0-{}): ", SHAPE_COUNT - 1);
    let _ = io::stdout().flush();
    let Some(type2) = read_i32() else {
        println!("Invalid input");
        return;
    };

    let (Some(type1), Some(type2)) = (ShapeTypeT::from_i32(type1), ShapeTypeT::from_i32(type2)) else {
        println!("Invalid shape type");
        return;
    };

    let s1 = shape_get(type1).unwrap();
    let s2 = shape_get(type2).unwrap();

    println!("\nShape 1: {} (ptr: {:p})", s1.name, s1);
    println!("Shape 2: {} (ptr: {:p})", s2.name, s2);
    println!("Comparison of pointers: {}", std::ptr::eq(s1, s2) as i32);

    if shape_equals(Some(s1), Some(s2)) == 1 {
        println!("Result: Shapes are EQUAL (same instance)");
    } else {
        println!("Result: Shapes are NOT EQUAL (different instances)");
    }
}

fn compare_scenes(scenes: &[Box<SceneT>]) {
    if scenes.len() < 2 {
        println!("Need at least 2 scenes to compare");
        return;
    }

    print!("Select first scene (0-{}): ", scenes.len() - 1);
    let _ = io::stdout().flush();
    let Some(idx1) = read_i32() else {
        println!("Invalid input");
        return;
    };

    print!("Select second scene (0-{}): ", scenes.len() - 1);
    let _ = io::stdout().flush();
    let Some(idx2) = read_i32() else {
        println!("Invalid input");
        return;
    };

    if idx1 < 0 || idx1 as usize >= scenes.len() || idx2 < 0 || idx2 as usize >= scenes.len() {
        println!("Invalid scene index");
        return;
    }

    let sc1 = &scenes[idx1 as usize];
    let sc2 = &scenes[idx2 as usize];

    println!("\nScene 1: {} ({} shapes)", sc1.name, sc1.shapes.len());
    scene_list_shapes(Some(sc1));

    println!("\nScene 2: {} ({} shapes)", sc2.name, sc2.shapes.len());
    scene_list_shapes(Some(sc2));

    if scene_equals(Some(sc1), Some(sc2)) == 1 {
        println!("\nResult: Scenes are EQUAL (1:1 correspondence)");
    } else {
        println!("\nResult: Scenes are NOT EQUAL");
    }
}

fn delete_scene(scenes: &mut Vec<Box<SceneT>>) {
    if scenes.is_empty() {
        println!("No scenes available");
        return;
    }

    print!("Select scene to delete (0-{}): ", scenes.len() - 1);
    let _ = io::stdout().flush();
    let Some(scene_idx) = read_i32() else {
        println!("Invalid input");
        return;
    };

    if scene_idx < 0 || scene_idx as usize >= scenes.len() {
        println!("Invalid scene index");
        return;
    }

    let scene = scenes.remove(scene_idx as usize);
    scene_destroy(scene);
    println!("Scene deleted");
}

fn main() {
    println!("╔════════════════════════════════════════╗");
    println!("║  ASCII ART DRAWING APPLICATION        ║");
    println!("║  Child-Friendly Shape Editor           ║");
    println!("╚════════════════════════════════════════╝");

    shape_manager_init();

    let mut scenes: Vec<Box<SceneT>> = Vec::new();

    loop {
        print_menu();

        let Some(input) = read_line_trimmed() else {
            break;
        };

        let Ok(choice) = input.trim().parse::<i32>() else {
            println!("Invalid input");
            continue;
        };

        match choice {
            1 => view_all_shapes(),
            2 => create_new_scene(&mut scenes),
            3 => add_shape_to_scene(&mut scenes),
            4 => remove_shape_from_scene(&mut scenes),
            5 => view_scene(&scenes),
            6 => list_all_scenes(&scenes),
            7 => save_scene_to_file(&scenes),
            8 => load_scene_from_file(&mut scenes),
            9 => compare_shapes(),
            10 => compare_scenes(&scenes),
            11 => delete_scene(&mut scenes),
            12 => {
                println!("\nCleaning up and exiting...");
                while let Some(scene) = scenes.pop() {
                    scene_destroy(scene);
                }
                shape_manager_cleanup();
                println!("Goodbye!");
                return;
            }
            _ => println!("Invalid choice"),
        }
    }

    while let Some(scene) = scenes.pop() {
        scene_destroy(scene);
    }
    shape_manager_cleanup();
}
