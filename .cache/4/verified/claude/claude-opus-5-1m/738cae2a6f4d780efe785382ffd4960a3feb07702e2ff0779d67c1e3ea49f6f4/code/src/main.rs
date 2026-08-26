//! Translation of `main.c` — ASCII art drawing application.
//!
//! Behaviour (including quirks and bugs) is preserved exactly; output is byte
//! identical to the original C program.

mod cio;
mod scene;
mod shape;

use cio::{discard_line, In, Out};
use scene::{
    scene_add_shape, scene_create, scene_equals, scene_list_shapes, scene_load, scene_print,
    scene_remove_shape, scene_save, Scene, MAX_SCENE_NAME,
};
use shape::{fmt_ptr, shape_equals, shape_type_name, ShapeManager, SHAPE_COUNT};

const MAX_SCENES: usize = 10;

/// The globals of `main.c`: `scenes` / `scene_count` plus the shape singletons.
struct App {
    scenes: Vec<Scene>,
    mgr: ShapeManager,
    out: Out,
    input: In,
}

impl App {
    fn scene_count(&self) -> i32 {
        self.scenes.len() as i32
    }
}

/// `print_menu`
fn print_menu(out: &mut Out) {
    out.s("\n");
    out.s("=========================================\n");
    out.s("  ASCII ART DRAWING APPLICATION\n");
    out.s("=========================================\n");
    out.s("1. View all available shapes\n");
    out.s("2. Create new scene\n");
    out.s("3. Add shape to scene\n");
    out.s("4. Remove shape from scene\n");
    out.s("5. View scene\n");
    out.s("6. List all scenes\n");
    out.s("7. Save scene\n");
    out.s("8. Load scene\n");
    out.s("9. Compare two shapes\n");
    out.s("10. Compare two scenes\n");
    out.s("11. Delete scene\n");
    out.s("12. Exit\n");
    out.s("=========================================\n");
    out.s("Choice: ");
}

/// `view_all_shapes`
fn view_all_shapes(app: &mut App) {
    app.out.s("\n=== Available Shapes ===\n");
    for i in 0..SHAPE_COUNT {
        p!(app.out, "\n{}. ", i + 1);
        let shape = app.mgr.get(i);
        app.mgr.print(&mut app.out, shape);
    }
}

/// `create_new_scene`
fn create_new_scene(app: &mut App) {
    if app.scenes.len() >= MAX_SCENES {
        app.out.s("Error: Maximum scenes reached\n");
        return;
    }

    app.out.s("Enter scene name: ");
    let name = match app.input.fgets(MAX_SCENE_NAME) {
        Some(line) => line,
        None => return,
    };
    let name = cio::strip_newline(&name);

    let index = app.scene_count();
    match scene_create(Some(&name)) {
        Some(scene) => {
            app.scenes.push(scene);
            app.out.s("Scene '");
            app.out.b(&name);
            p!(app.out, "' created (index {})\n", index);
        }
        None => {
            app.out.s("Error creating scene\n");
        }
    }
}

/// `add_shape_to_scene`
fn add_shape_to_scene(app: &mut App) {
    if app.scenes.is_empty() {
        app.out
            .s("No scenes available. Create a scene first.\n");
        return;
    }

    p!(app.out, "Select scene (0-{}): ", app.scene_count() - 1);
    let scene_idx = match app.input.scan_int() {
        Some(v) => v,
        None => {
            app.out.s("Invalid input\n");
            discard_line(&mut app.input);
            return;
        }
    };
    discard_line(&mut app.input);

    if scene_idx < 0 || scene_idx >= app.scene_count() {
        app.out.s("Invalid scene index\n");
        return;
    }

    app.out.s("\nSelect shape to add:\n");
    for i in 0..SHAPE_COUNT {
        p!(app.out, "{}. {}\n", i, shape_type_name(i));
    }
    app.out.s("Choice: ");

    let shape_type = match app.input.scan_int() {
        Some(v) => v,
        None => {
            app.out.s("Invalid input\n");
            discard_line(&mut app.input);
            return;
        }
    };
    discard_line(&mut app.input);

    if shape_type < 0 || shape_type >= SHAPE_COUNT {
        app.out.s("Invalid shape type\n");
        return;
    }

    let shape = app.mgr.get(shape_type);
    let scene = &mut app.scenes[scene_idx as usize];
    if scene_add_shape(scene, shape) == 0 {
        let r = shape.expect("validated shape");
        p!(
            app.out,
            "Shape '{}' added to scene (reusing singleton at {})\n",
            app.mgr.name(r),
            fmt_ptr(app.mgr.addr(r))
        );
    } else {
        app.out.s("Error adding shape\n");
    }
}

/// `remove_shape_from_scene`
fn remove_shape_from_scene(app: &mut App) {
    if app.scenes.is_empty() {
        app.out.s("No scenes available\n");
        return;
    }

    p!(app.out, "Select scene (0-{}): ", app.scene_count() - 1);
    let scene_idx = match app.input.scan_int() {
        Some(v) => v,
        None => {
            app.out.s("Invalid input\n");
            discard_line(&mut app.input);
            return;
        }
    };
    discard_line(&mut app.input);

    if scene_idx < 0 || scene_idx >= app.scene_count() {
        app.out.s("Invalid scene index\n");
        return;
    }

    scene_list_shapes(&mut app.out, &app.mgr, &app.scenes[scene_idx as usize]);

    if app.scenes[scene_idx as usize].shape_count() == 0 {
        app.out.s("Scene is empty\n");
        return;
    }

    p!(
        app.out,
        "Select shape to remove (1-{}): ",
        app.scenes[scene_idx as usize].shape_count()
    );
    let shape_idx = match app.input.scan_int() {
        Some(v) => v,
        None => {
            app.out.s("Invalid input\n");
            discard_line(&mut app.input);
            return;
        }
    };
    discard_line(&mut app.input);

    if scene_remove_shape(
        &mut app.scenes[scene_idx as usize],
        shape_idx.wrapping_sub(1),
    ) == 0
    {
        app.out.s("Shape removed\n");
    } else {
        app.out.s("Error removing shape\n");
    }
}

/// `view_scene`
fn view_scene(app: &mut App) {
    if app.scenes.is_empty() {
        app.out.s("No scenes available\n");
        return;
    }

    p!(app.out, "Select scene (0-{}): ", app.scene_count() - 1);
    let scene_idx = match app.input.scan_int() {
        Some(v) => v,
        None => {
            app.out.s("Invalid input\n");
            discard_line(&mut app.input);
            return;
        }
    };
    discard_line(&mut app.input);

    if scene_idx < 0 || scene_idx >= app.scene_count() {
        app.out.s("Invalid scene index\n");
        return;
    }

    scene_print(&mut app.out, &app.mgr, &app.scenes[scene_idx as usize]);
}

/// `list_all_scenes`
fn list_all_scenes(app: &mut App) {
    app.out.s("\n=== All Scenes ===\n");
    if app.scenes.is_empty() {
        app.out.s("No scenes created yet\n");
        return;
    }

    for i in 0..app.scenes.len() {
        p!(app.out, "{}. ", i);
        let name = app.scenes[i].name.clone();
        app.out.b(&name);
        p!(app.out, " ({} shapes)\n", app.scenes[i].shape_count());
    }
}

/// `save_scene_to_file`
fn save_scene_to_file(app: &mut App) {
    if app.scenes.is_empty() {
        app.out.s("No scenes available\n");
        return;
    }

    p!(app.out, "Select scene (0-{}): ", app.scene_count() - 1);
    let scene_idx = match app.input.scan_int() {
        Some(v) => v,
        None => {
            app.out.s("Invalid input\n");
            discard_line(&mut app.input);
            return;
        }
    };
    discard_line(&mut app.input);

    if scene_idx < 0 || scene_idx >= app.scene_count() {
        app.out.s("Invalid scene index\n");
        return;
    }

    app.out.s("Enter filename: ");
    let filename = match app.input.fgets(256) {
        Some(line) => line,
        None => return,
    };
    let filename = cio::strip_newline(&filename);

    scene_save(
        &mut app.out,
        &app.mgr,
        &app.scenes[scene_idx as usize],
        &filename,
    );
}

/// `load_scene_from_file`
fn load_scene_from_file(app: &mut App) {
    if app.scenes.len() >= MAX_SCENES {
        app.out.s("Error: Maximum scenes reached\n");
        return;
    }

    app.out.s("Enter filename: ");
    let filename = match app.input.fgets(256) {
        Some(line) => line,
        None => return,
    };
    let filename = cio::strip_newline(&filename);

    if let Some(scene) = scene_load(&mut app.out, &app.mgr, &filename) {
        app.scenes.push(scene);
        p!(app.out, "Scene loaded (index {})\n", app.scene_count() - 1);
    }
}

/// `compare_shapes`
fn compare_shapes(app: &mut App) {
    p!(app.out, "\nSelect first shape (0-{}):\n", SHAPE_COUNT - 1);
    for i in 0..SHAPE_COUNT {
        p!(app.out, "{}. {}\n", i, shape_type_name(i));
    }
    app.out.s("Choice: ");

    let type1 = match app.input.scan_int() {
        Some(v) => v,
        None => {
            app.out.s("Invalid input\n");
            discard_line(&mut app.input);
            return;
        }
    };
    discard_line(&mut app.input);

    p!(app.out, "\nSelect second shape (0-{}): ", SHAPE_COUNT - 1);
    let type2 = match app.input.scan_int() {
        Some(v) => v,
        None => {
            app.out.s("Invalid input\n");
            discard_line(&mut app.input);
            return;
        }
    };
    discard_line(&mut app.input);

    if type1 < 0 || type1 >= SHAPE_COUNT || type2 < 0 || type2 >= SHAPE_COUNT {
        app.out.s("Invalid shape type\n");
        return;
    }

    let s1 = app.mgr.get(type1).expect("validated shape");
    let s2 = app.mgr.get(type2).expect("validated shape");

    p!(
        app.out,
        "\nShape 1: {} (ptr: {})\n",
        app.mgr.name(s1),
        fmt_ptr(app.mgr.addr(s1))
    );
    p!(
        app.out,
        "Shape 2: {} (ptr: {})\n",
        app.mgr.name(s2),
        fmt_ptr(app.mgr.addr(s2))
    );
    p!(
        app.out,
        "Comparison of pointers: {}\n",
        if s1 == s2 { 1 } else { 0 }
    );

    if shape_equals(s1, s2) != 0 {
        app.out.s("Result: Shapes are EQUAL (same instance)\n");
    } else {
        app.out
            .s("Result: Shapes are NOT EQUAL (different instances)\n");
    }
}

/// `compare_scenes`
fn compare_scenes(app: &mut App) {
    if app.scene_count() < 2 {
        app.out.s("Need at least 2 scenes to compare\n");
        return;
    }

    p!(app.out, "Select first scene (0-{}): ", app.scene_count() - 1);
    let idx1 = match app.input.scan_int() {
        Some(v) => v,
        None => {
            app.out.s("Invalid input\n");
            discard_line(&mut app.input);
            return;
        }
    };
    discard_line(&mut app.input);

    p!(app.out, "Select second scene (0-{}): ", app.scene_count() - 1);
    let idx2 = match app.input.scan_int() {
        Some(v) => v,
        None => {
            app.out.s("Invalid input\n");
            discard_line(&mut app.input);
            return;
        }
    };
    discard_line(&mut app.input);

    if idx1 < 0 || idx1 >= app.scene_count() || idx2 < 0 || idx2 >= app.scene_count() {
        app.out.s("Invalid scene index\n");
        return;
    }

    let sc1 = idx1 as usize;
    let sc2 = idx2 as usize;

    app.out.s("\nScene 1: ");
    let name1 = app.scenes[sc1].name.clone();
    app.out.b(&name1);
    p!(app.out, " ({} shapes)\n", app.scenes[sc1].shape_count());
    scene_list_shapes(&mut app.out, &app.mgr, &app.scenes[sc1]);

    app.out.s("\nScene 2: ");
    let name2 = app.scenes[sc2].name.clone();
    app.out.b(&name2);
    p!(app.out, " ({} shapes)\n", app.scenes[sc2].shape_count());
    scene_list_shapes(&mut app.out, &app.mgr, &app.scenes[sc2]);

    if scene_equals(&app.scenes[sc1], &app.scenes[sc2]) != 0 {
        app.out
            .s("\nResult: Scenes are EQUAL (1:1 correspondence)\n");
    } else {
        app.out.s("\nResult: Scenes are NOT EQUAL\n");
    }
}

/// `delete_scene`
fn delete_scene(app: &mut App) {
    if app.scenes.is_empty() {
        app.out.s("No scenes available\n");
        return;
    }

    p!(
        app.out,
        "Select scene to delete (0-{}): ",
        app.scene_count() - 1
    );
    let scene_idx = match app.input.scan_int() {
        Some(v) => v,
        None => {
            app.out.s("Invalid input\n");
            discard_line(&mut app.input);
            return;
        }
    };
    discard_line(&mut app.input);

    if scene_idx < 0 || scene_idx >= app.scene_count() {
        app.out.s("Invalid scene index\n");
        return;
    }

    // scene_destroy + shift the remaining scenes
    app.scenes.remove(scene_idx as usize);

    app.out.s("Scene deleted\n");
}

fn main() {
    let mut app = App {
        scenes: Vec::new(),
        mgr: ShapeManager::new(),
        out: Out::new(),
        input: cio::stdin_reader(),
    };

    app.out
        .s("\u{2554}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2557}\n");
    app.out
        .s("\u{2551}  ASCII ART DRAWING APPLICATION        \u{2551}\n");
    app.out
        .s("\u{2551}  Child-Friendly Shape Editor           \u{2551}\n");
    app.out
        .s("\u{255A}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255D}\n");

    // Initialize shape manager (allocate all shapes once)
    app.mgr.init();

    loop {
        print_menu(&mut app.out);

        let input = match app.input.fgets(256) {
            Some(line) => line,
            None => break,
        };

        let choice = match cio::sscanf_int(&input) {
            Some(v) => v,
            None => {
                app.out.s("Invalid input\n");
                continue;
            }
        };

        match choice {
            1 => view_all_shapes(&mut app),
            2 => create_new_scene(&mut app),
            3 => add_shape_to_scene(&mut app),
            4 => remove_shape_from_scene(&mut app),
            5 => view_scene(&mut app),
            6 => list_all_scenes(&mut app),
            7 => save_scene_to_file(&mut app),
            8 => load_scene_from_file(&mut app),
            9 => compare_shapes(&mut app),
            10 => compare_scenes(&mut app),
            11 => delete_scene(&mut app),
            12 => {
                app.out.s("\nCleaning up and exiting...\n");
                app.scenes.clear();
                app.mgr.cleanup();
                app.out.s("Goodbye!\n");
                app.out.flush();
                return;
            }
            _ => {
                app.out.s("Invalid choice\n");
            }
        }
    }

    // Cleanup
    app.scenes.clear();
    app.mgr.cleanup();

    app.out.flush();
}
