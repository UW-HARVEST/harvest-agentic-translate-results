//! Translation of main.c
//!
//! An interactive ASCII art scene editor. Output is byte-for-byte identical to
//! the original C program (pointer values printed with `%p` are inherently
//! environment dependent; deterministic stand-ins are used).

mod cio;
mod scene;
mod shape;

use cio::{cprintf, trim_at_newline, Arg, DigitAcc, In, Out};
use scene::{
    scene_add_shape, scene_create, scene_equals, scene_list_shapes, scene_load, scene_print,
    scene_remove_shape, scene_save, Scene, MAX_SCENE_NAME,
};
use shape::{shape_get, shape_name, shape_print, shape_ptr, shape_type_name, SHAPE_COUNT};

const MAX_SCENES: usize = 10;

struct App {
    out: Out,
    inp: In,
    scenes: Vec<Scene>,
}

impl App {
    fn scene_count(&self) -> i32 {
        self.scenes.len() as i32
    }
}

fn print_menu(app: &mut App) {
    let out = &mut app.out;
    cprintf(out, b"\n", &[]);
    cprintf(out, b"=========================================\n", &[]);
    cprintf(out, b"  ASCII ART DRAWING APPLICATION\n", &[]);
    cprintf(out, b"=========================================\n", &[]);
    cprintf(out, b"1. View all available shapes\n", &[]);
    cprintf(out, b"2. Create new scene\n", &[]);
    cprintf(out, b"3. Add shape to scene\n", &[]);
    cprintf(out, b"4. Remove shape from scene\n", &[]);
    cprintf(out, b"5. View scene\n", &[]);
    cprintf(out, b"6. List all scenes\n", &[]);
    cprintf(out, b"7. Save scene\n", &[]);
    cprintf(out, b"8. Load scene\n", &[]);
    cprintf(out, b"9. Compare two shapes\n", &[]);
    cprintf(out, b"10. Compare two scenes\n", &[]);
    cprintf(out, b"11. Delete scene\n", &[]);
    cprintf(out, b"12. Exit\n", &[]);
    cprintf(out, b"=========================================\n", &[]);
    cprintf(out, b"Choice: ", &[]);
}

fn view_all_shapes(app: &mut App) {
    cprintf(&mut app.out, b"\n=== Available Shapes ===\n", &[]);
    let mut i = 0;
    while i < SHAPE_COUNT {
        cprintf(&mut app.out, b"\n%d. ", &[Arg::D(i + 1)]);
        shape_print(&mut app.out, shape_get(i));
        i += 1;
    }
}

fn create_new_scene(app: &mut App) {
    if app.scenes.len() >= MAX_SCENES {
        cprintf(&mut app.out, b"Error: Maximum scenes reached\n", &[]);
        return;
    }

    cprintf(&mut app.out, b"Enter scene name: ", &[]);
    let raw = match app.inp.fgets(MAX_SCENE_NAME) {
        None => return,
        Some(r) => r,
    };
    let name = trim_at_newline(&raw).to_vec();

    let idx = app.scene_count();
    app.scenes.push(scene_create(Some(&name)));
    cprintf(
        &mut app.out,
        b"Scene '%s' created (index %d)\n",
        &[Arg::S(&name), Arg::D(idx)],
    );
}

fn add_shape_to_scene(app: &mut App) {
    if app.scenes.is_empty() {
        cprintf(
            &mut app.out,
            b"No scenes available. Create a scene first.\n",
            &[],
        );
        return;
    }

    let n = app.scene_count() - 1;
    cprintf(
        &mut app.out,
        b"Select scene (0-%d): ",
        &[Arg::D(n)],
    );
    let scene_idx = match app.inp.scan_int() {
        None => {
            cprintf(&mut app.out, b"Invalid input\n", &[]);
            app.inp.skip_to_newline();
            return;
        }
        Some(v) => v,
    };
    app.inp.skip_to_newline();

    if scene_idx < 0 || scene_idx >= app.scene_count() {
        cprintf(&mut app.out, b"Invalid scene index\n", &[]);
        return;
    }

    cprintf(&mut app.out, b"\nSelect shape to add:\n", &[]);
    let mut i = 0;
    while i < SHAPE_COUNT {
        cprintf(
            &mut app.out,
            b"%d. %s\n",
            &[Arg::D(i), Arg::S(shape_type_name(i))],
        );
        i += 1;
    }
    cprintf(&mut app.out, b"Choice: ", &[]);

    let shape_type = match app.inp.scan_int() {
        None => {
            cprintf(&mut app.out, b"Invalid input\n", &[]);
            app.inp.skip_to_newline();
            return;
        }
        Some(v) => v,
    };
    app.inp.skip_to_newline();

    if shape_type < 0 || shape_type >= SHAPE_COUNT {
        cprintf(&mut app.out, b"Invalid shape type\n", &[]);
        return;
    }

    let shape = shape_get(shape_type);
    let scene = &mut app.scenes[scene_idx as usize];
    if scene_add_shape(scene, shape) == 0 {
        let idx = shape.unwrap();
        cprintf(
            &mut app.out,
            b"Shape '%s' added to scene (reusing singleton at %p)\n",
            &[Arg::S(shape_name(idx)), Arg::P(shape_ptr(idx))],
        );
    } else {
        cprintf(&mut app.out, b"Error adding shape\n", &[]);
    }
}

fn remove_shape_from_scene(app: &mut App) {
    if app.scenes.is_empty() {
        cprintf(&mut app.out, b"No scenes available\n", &[]);
        return;
    }

    let n = app.scene_count() - 1;
    cprintf(
        &mut app.out,
        b"Select scene (0-%d): ",
        &[Arg::D(n)],
    );
    let scene_idx = match app.inp.scan_int() {
        None => {
            cprintf(&mut app.out, b"Invalid input\n", &[]);
            app.inp.skip_to_newline();
            return;
        }
        Some(v) => v,
    };
    app.inp.skip_to_newline();

    if scene_idx < 0 || scene_idx >= app.scene_count() {
        cprintf(&mut app.out, b"Invalid scene index\n", &[]);
        return;
    }

    scene_list_shapes(&mut app.out, &app.scenes[scene_idx as usize]);

    if app.scenes[scene_idx as usize].shape_count() == 0 {
        cprintf(&mut app.out, b"Scene is empty\n", &[]);
        return;
    }

    let n = app.scenes[scene_idx as usize].shape_count();
    cprintf(
        &mut app.out,
        b"Select shape to remove (1-%d): ",
        &[Arg::D(n)],
    );
    let shape_idx = match app.inp.scan_int() {
        None => {
            cprintf(&mut app.out, b"Invalid input\n", &[]);
            app.inp.skip_to_newline();
            return;
        }
        Some(v) => v,
    };
    app.inp.skip_to_newline();

    // C computes `shape_idx - 1` in `int`; INT_MIN wraps around.
    let target = shape_idx.wrapping_sub(1);
    if scene_remove_shape(&mut app.scenes[scene_idx as usize], target) == 0 {
        cprintf(&mut app.out, b"Shape removed\n", &[]);
    } else {
        cprintf(&mut app.out, b"Error removing shape\n", &[]);
    }
}

fn view_scene(app: &mut App) {
    if app.scenes.is_empty() {
        cprintf(&mut app.out, b"No scenes available\n", &[]);
        return;
    }

    let n = app.scene_count() - 1;
    cprintf(
        &mut app.out,
        b"Select scene (0-%d): ",
        &[Arg::D(n)],
    );
    let scene_idx = match app.inp.scan_int() {
        None => {
            cprintf(&mut app.out, b"Invalid input\n", &[]);
            app.inp.skip_to_newline();
            return;
        }
        Some(v) => v,
    };
    app.inp.skip_to_newline();

    if scene_idx < 0 || scene_idx >= app.scene_count() {
        cprintf(&mut app.out, b"Invalid scene index\n", &[]);
        return;
    }

    scene_print(&mut app.out, &app.scenes[scene_idx as usize]);
}

fn list_all_scenes(app: &mut App) {
    cprintf(&mut app.out, b"\n=== All Scenes ===\n", &[]);
    if app.scenes.is_empty() {
        cprintf(&mut app.out, b"No scenes created yet\n", &[]);
        return;
    }

    let mut i = 0;
    while i < app.scene_count() {
        let s = &app.scenes[i as usize];
        let name = s.name.clone();
        let count = s.shape_count();
        cprintf(
            &mut app.out,
            b"%d. %s (%d shapes)\n",
            &[Arg::D(i), Arg::S(&name), Arg::D(count)],
        );
        i += 1;
    }
}

fn save_scene_to_file(app: &mut App) {
    if app.scenes.is_empty() {
        cprintf(&mut app.out, b"No scenes available\n", &[]);
        return;
    }

    let n = app.scene_count() - 1;
    cprintf(
        &mut app.out,
        b"Select scene (0-%d): ",
        &[Arg::D(n)],
    );
    let scene_idx = match app.inp.scan_int() {
        None => {
            cprintf(&mut app.out, b"Invalid input\n", &[]);
            app.inp.skip_to_newline();
            return;
        }
        Some(v) => v,
    };
    app.inp.skip_to_newline();

    if scene_idx < 0 || scene_idx >= app.scene_count() {
        cprintf(&mut app.out, b"Invalid scene index\n", &[]);
        return;
    }

    cprintf(&mut app.out, b"Enter filename: ", &[]);
    let raw = match app.inp.fgets(256) {
        None => return,
        Some(r) => r,
    };
    let filename = trim_at_newline(&raw).to_vec();

    let scene = &app.scenes[scene_idx as usize];
    scene_save(&mut app.out, scene, &filename);
}

fn load_scene_from_file(app: &mut App) {
    if app.scenes.len() >= MAX_SCENES {
        cprintf(&mut app.out, b"Error: Maximum scenes reached\n", &[]);
        return;
    }

    cprintf(&mut app.out, b"Enter filename: ", &[]);
    let raw = match app.inp.fgets(256) {
        None => return,
        Some(r) => r,
    };
    let filename = trim_at_newline(&raw).to_vec();

    if let Some(scene) = scene_load(&mut app.out, &filename) {
        app.scenes.push(scene);
        let n = app.scene_count() - 1;
        cprintf(
            &mut app.out,
            b"Scene loaded (index %d)\n",
            &[Arg::D(n)],
        );
    }
}

fn compare_shapes(app: &mut App) {
    cprintf(
        &mut app.out,
        b"\nSelect first shape (0-%d):\n",
        &[Arg::D(SHAPE_COUNT - 1)],
    );
    let mut i = 0;
    while i < SHAPE_COUNT {
        cprintf(
            &mut app.out,
            b"%d. %s\n",
            &[Arg::D(i), Arg::S(shape_type_name(i))],
        );
        i += 1;
    }
    cprintf(&mut app.out, b"Choice: ", &[]);

    let type1 = match app.inp.scan_int() {
        None => {
            cprintf(&mut app.out, b"Invalid input\n", &[]);
            app.inp.skip_to_newline();
            return;
        }
        Some(v) => v,
    };
    app.inp.skip_to_newline();

    cprintf(
        &mut app.out,
        b"\nSelect second shape (0-%d): ",
        &[Arg::D(SHAPE_COUNT - 1)],
    );
    let type2 = match app.inp.scan_int() {
        None => {
            cprintf(&mut app.out, b"Invalid input\n", &[]);
            app.inp.skip_to_newline();
            return;
        }
        Some(v) => v,
    };
    app.inp.skip_to_newline();

    if type1 < 0 || type1 >= SHAPE_COUNT || type2 < 0 || type2 >= SHAPE_COUNT {
        cprintf(&mut app.out, b"Invalid shape type\n", &[]);
        return;
    }

    let s1 = shape_get(type1).unwrap();
    let s2 = shape_get(type2).unwrap();

    cprintf(
        &mut app.out,
        b"\nShape 1: %s (ptr: %p)\n",
        &[Arg::S(shape_name(s1)), Arg::P(shape_ptr(s1))],
    );
    cprintf(
        &mut app.out,
        b"Shape 2: %s (ptr: %p)\n",
        &[Arg::S(shape_name(s2)), Arg::P(shape_ptr(s2))],
    );
    cprintf(
        &mut app.out,
        b"Comparison of pointers: %d\n",
        &[Arg::D(if s1 == s2 { 1 } else { 0 })],
    );

    if s1 == s2 {
        cprintf(
            &mut app.out,
            b"Result: Shapes are EQUAL (same instance)\n",
            &[],
        );
    } else {
        cprintf(
            &mut app.out,
            b"Result: Shapes are NOT EQUAL (different instances)\n",
            &[],
        );
    }
}

fn compare_scenes(app: &mut App) {
    if app.scene_count() < 2 {
        cprintf(
            &mut app.out,
            b"Need at least 2 scenes to compare\n",
            &[],
        );
        return;
    }

    let n = app.scene_count() - 1;
    cprintf(
        &mut app.out,
        b"Select first scene (0-%d): ",
        &[Arg::D(n)],
    );
    let idx1 = match app.inp.scan_int() {
        None => {
            cprintf(&mut app.out, b"Invalid input\n", &[]);
            app.inp.skip_to_newline();
            return;
        }
        Some(v) => v,
    };
    app.inp.skip_to_newline();

    let n = app.scene_count() - 1;
    cprintf(
        &mut app.out,
        b"Select second scene (0-%d): ",
        &[Arg::D(n)],
    );
    let idx2 = match app.inp.scan_int() {
        None => {
            cprintf(&mut app.out, b"Invalid input\n", &[]);
            app.inp.skip_to_newline();
            return;
        }
        Some(v) => v,
    };
    app.inp.skip_to_newline();

    if idx1 < 0 || idx1 >= app.scene_count() || idx2 < 0 || idx2 >= app.scene_count() {
        cprintf(&mut app.out, b"Invalid scene index\n", &[]);
        return;
    }

    let name1 = app.scenes[idx1 as usize].name.clone();
    let count1 = app.scenes[idx1 as usize].shape_count();
    let name2 = app.scenes[idx2 as usize].name.clone();
    let count2 = app.scenes[idx2 as usize].shape_count();

    cprintf(
        &mut app.out,
        b"\nScene 1: %s (%d shapes)\n",
        &[Arg::S(&name1), Arg::D(count1)],
    );
    scene_list_shapes(&mut app.out, &app.scenes[idx1 as usize]);

    cprintf(
        &mut app.out,
        b"\nScene 2: %s (%d shapes)\n",
        &[Arg::S(&name2), Arg::D(count2)],
    );
    scene_list_shapes(&mut app.out, &app.scenes[idx2 as usize]);

    let equal = scene_equals(&app.scenes[idx1 as usize], &app.scenes[idx2 as usize]);
    if equal {
        cprintf(
            &mut app.out,
            b"\nResult: Scenes are EQUAL (1:1 correspondence)\n",
            &[],
        );
    } else {
        cprintf(&mut app.out, b"\nResult: Scenes are NOT EQUAL\n", &[]);
    }
}

fn delete_scene(app: &mut App) {
    if app.scenes.is_empty() {
        cprintf(&mut app.out, b"No scenes available\n", &[]);
        return;
    }

    let n = app.scene_count() - 1;
    cprintf(
        &mut app.out,
        b"Select scene to delete (0-%d): ",
        &[Arg::D(n)],
    );
    let scene_idx = match app.inp.scan_int() {
        None => {
            cprintf(&mut app.out, b"Invalid input\n", &[]);
            app.inp.skip_to_newline();
            return;
        }
        Some(v) => v,
    };
    app.inp.skip_to_newline();

    if scene_idx < 0 || scene_idx >= app.scene_count() {
        cprintf(&mut app.out, b"Invalid scene index\n", &[]);
        return;
    }

    app.scenes.remove(scene_idx as usize);
    cprintf(&mut app.out, b"Scene deleted\n", &[]);
}

fn main() {
    let mut app = App {
        out: Out::new(),
        inp: In::new(),
        scenes: Vec::new(),
    };

    cprintf(
        &mut app.out,
        "╔════════════════════════════════════════╗\n".as_bytes(),
        &[],
    );
    cprintf(
        &mut app.out,
        "║  ASCII ART DRAWING APPLICATION        ║\n".as_bytes(),
        &[],
    );
    cprintf(
        &mut app.out,
        "║  Child-Friendly Shape Editor           ║\n".as_bytes(),
        &[],
    );
    cprintf(
        &mut app.out,
        "╚════════════════════════════════════════╝\n".as_bytes(),
        &[],
    );

    // shape_manager_init(): the singletons are static data here.

    loop {
        print_menu(&mut app);

        let input = match app.inp.fgets(256) {
            None => break,
            Some(v) => v,
        };

        // sscanf(input, "%d", &choice)
        let choice = match sscanf_int(&input) {
            None => {
                cprintf(&mut app.out, b"Invalid input\n", &[]);
                continue;
            }
            Some(v) => v,
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
                cprintf(&mut app.out, b"\nCleaning up and exiting...\n", &[]);
                app.scenes.clear();
                cprintf(&mut app.out, b"Goodbye!\n", &[]);
                app.out.flush();
                return;
            }
            _ => cprintf(&mut app.out, b"Invalid choice\n", &[]),
        }
    }

    // Cleanup
    app.scenes.clear();
    app.out.flush();
}

/// `sscanf(buf, "%d", &x)` over a NUL-terminated C buffer.
fn sscanf_int(buf: &[u8]) -> Option<i32> {
    let s = match buf.iter().position(|&c| c == 0) {
        Some(p) => &buf[..p],
        None => buf,
    };

    let mut i = 0usize;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }
    if i >= s.len() {
        return None;
    }

    let mut negative = false;
    if s[i] == b'+' || s[i] == b'-' {
        negative = s[i] == b'-';
        i += 1;
    }
    if i >= s.len() || !s[i].is_ascii_digit() {
        return None;
    }

    let mut acc = DigitAcc::new();
    while i < s.len() && s[i].is_ascii_digit() {
        acc.push(s[i] - b'0');
        i += 1;
    }
    Some(acc.finish(negative))
}

// Keep the unbuffered stderr helper reachable even if a build configuration
// never triggers an error path.

