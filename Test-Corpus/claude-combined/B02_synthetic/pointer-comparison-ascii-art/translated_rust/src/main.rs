// Translation of the C ASCII art drawing application to Rust.
// Aims to reproduce the C printf/scanf output byte-for-byte for the
// deterministic code paths.  Pointer addresses (printed with %p in C)
// are inherently non-deterministic; the format is matched but the
// numeric values cannot be made identical to the C run.

#![allow(dead_code)]

use std::cell::RefCell;
use std::io::{Read, Write};

mod shape;
mod scene;

use shape::{shape_manager_cleanup, shape_manager_init, shape_get, shape_print, shape_equals,
            shape_type_name, ShapeType, SHAPE_COUNT};
use scene::{scene_create, scene_destroy, scene_add_shape, scene_remove_shape, scene_print,
            scene_equals, scene_save, scene_load, scene_list_shapes, MAX_SCENE_NAME, Scene};

const MAX_SCENES: usize = 10;

// Global stdin reader so we can mimic C scanf/getchar/fgets semantics.
thread_local! {
    static STDIN_BUF: RefCell<StdinReader> = RefCell::new(StdinReader::new());
}

struct StdinReader {
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
}

impl StdinReader {
    fn new() -> Self {
        let mut buf = Vec::new();
        let _ = std::io::stdin().lock().read_to_end(&mut buf);
        StdinReader { buf, pos: 0, eof: false }
    }

    fn peek(&self) -> Option<u8> {
        if self.pos < self.buf.len() {
            Some(self.buf[self.pos])
        } else {
            None
        }
    }

    fn read_byte(&mut self) -> Option<u8> {
        if self.pos < self.buf.len() {
            let b = self.buf[self.pos];
            self.pos += 1;
            Some(b)
        } else {
            self.eof = true;
            None
        }
    }

    // Read a line as fgets does: stops at newline (which is included),
    // limit to size-1 bytes plus a NUL.  Returns None on immediate EOF.
    fn fgets(&mut self, size: usize) -> Option<Vec<u8>> {
        if self.pos >= self.buf.len() {
            return None;
        }
        let mut out: Vec<u8> = Vec::new();
        while out.len() < size.saturating_sub(1) {
            match self.read_byte() {
                Some(b) => {
                    out.push(b);
                    if b == b'\n' {
                        break;
                    }
                }
                None => break,
            }
        }
        Some(out)
    }

    // Skip whitespace then read an integer like scanf("%d").
    fn scanf_int(&mut self) -> Option<i32> {
        // skip whitespace
        while let Some(b) = self.peek() {
            if (b as char).is_ascii_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
        let start = self.pos;
        let mut has_digit = false;
        let mut sign_present = false;
        if let Some(b) = self.peek() {
            if b == b'-' || b == b'+' {
                self.pos += 1;
                sign_present = true;
            }
        }
        while let Some(b) = self.peek() {
            if b.is_ascii_digit() {
                self.pos += 1;
                has_digit = true;
            } else {
                break;
            }
        }
        if !has_digit {
            // Reset position if no digit consumed (matches scanf behavior:
            // unmatched chars left in stream).  Restore sign to stream too.
            if sign_present {
                self.pos = start;
            }
            return None;
        }
        let s = std::str::from_utf8(&self.buf[start..self.pos]).ok()?;
        s.parse::<i32>().ok()
    }

    // Mimic getchar(): returns next byte or None on EOF.
    fn getchar(&mut self) -> Option<u8> {
        self.read_byte()
    }
}

fn fgets(size: usize) -> Option<Vec<u8>> {
    STDIN_BUF.with(|s| s.borrow_mut().fgets(size))
}

fn scanf_int() -> Option<i32> {
    STDIN_BUF.with(|s| s.borrow_mut().scanf_int())
}

// while (getchar() != '\n');  -- consumes through newline (or EOF).
fn consume_to_newline() {
    STDIN_BUF.with(|s| {
        let mut r = s.borrow_mut();
        loop {
            match r.getchar() {
                Some(b'\n') => break,
                Some(_) => continue,
                None => break,
            }
        }
    });
}

// printf wrapper that matches C stdout buffering semantics by flushing
// before stdin reads.
fn print(s: &str) {
    let stdout = std::io::stdout();
    let mut h = stdout.lock();
    h.write_all(s.as_bytes()).ok();
}

fn flush_stdout() {
    let _ = std::io::stdout().flush();
}

fn print_menu() {
    print("\n");
    print("=========================================\n");
    print("  ASCII ART DRAWING APPLICATION\n");
    print("=========================================\n");
    print("1. View all available shapes\n");
    print("2. Create new scene\n");
    print("3. Add shape to scene\n");
    print("4. Remove shape from scene\n");
    print("5. View scene\n");
    print("6. List all scenes\n");
    print("7. Save scene\n");
    print("8. Load scene\n");
    print("9. Compare two shapes\n");
    print("10. Compare two scenes\n");
    print("11. Delete scene\n");
    print("12. Exit\n");
    print("=========================================\n");
    print("Choice: ");
    flush_stdout();
}

fn view_all_shapes() {
    print("\n=== Available Shapes ===\n");
    for i in 0..SHAPE_COUNT as i32 {
        print(&format!("\n{}. ", i + 1));
        if let Some(s) = shape_get(ShapeType::from_int(i)) {
            shape_print(Some(s));
        } else {
            shape_print(None);
        }
    }
}

fn create_new_scene(scenes: &mut Vec<Scene>) {
    if scenes.len() >= MAX_SCENES {
        print("Error: Maximum scenes reached\n");
        return;
    }

    print("Enter scene name: ");
    flush_stdout();
    let raw = match fgets(MAX_SCENE_NAME) {
        Some(s) => s,
        None => return,
    };
    // Strip newline like strcspn(name, "\n") = 0
    let mut name_bytes: Vec<u8> = raw.into_iter().take_while(|b| *b != b'\n').collect();
    // Truncate to MAX_SCENE_NAME-1 to mirror fgets buffer cap
    if name_bytes.len() > MAX_SCENE_NAME - 1 {
        name_bytes.truncate(MAX_SCENE_NAME - 1);
    }
    let name = String::from_utf8_lossy(&name_bytes).into_owned();

    let scene = scene_create(Some(&name));
    let idx = scenes.len();
    print(&format!("Scene '{}' created (index {})\n", name, idx));
    scenes.push(scene);
}

fn add_shape_to_scene(scenes: &mut Vec<Scene>) {
    if scenes.is_empty() {
        print("No scenes available. Create a scene first.\n");
        return;
    }

    print(&format!("Select scene (0-{}): ", scenes.len() as i32 - 1));
    flush_stdout();
    let scene_idx = match scanf_int() {
        Some(v) => v,
        None => {
            print("Invalid input\n");
            consume_to_newline();
            return;
        }
    };
    consume_to_newline();

    if scene_idx < 0 || scene_idx as usize >= scenes.len() {
        print("Invalid scene index\n");
        return;
    }

    print("\nSelect shape to add:\n");
    for i in 0..SHAPE_COUNT as i32 {
        print(&format!("{}. {}\n", i, shape_type_name(ShapeType::from_int(i))));
    }
    print("Choice: ");
    flush_stdout();

    let shape_type_n = match scanf_int() {
        Some(v) => v,
        None => {
            print("Invalid input\n");
            consume_to_newline();
            return;
        }
    };
    consume_to_newline();

    if shape_type_n < 0 || shape_type_n >= SHAPE_COUNT as i32 {
        print("Invalid shape type\n");
        return;
    }

    let shape = shape_get(ShapeType::from_int(shape_type_n));
    let scene = &mut scenes[scene_idx as usize];
    if scene_add_shape(scene, shape).is_ok() {
        let s = shape.unwrap();
        print(&format!(
            "Shape '{}' added to scene (reusing singleton at {})\n",
            s.name(),
            ptr_format(s as *const _ as *const ())
        ));
    } else {
        print("Error adding shape\n");
    }
}

fn remove_shape_from_scene(scenes: &mut Vec<Scene>) {
    if scenes.is_empty() {
        print("No scenes available\n");
        return;
    }

    print(&format!("Select scene (0-{}): ", scenes.len() as i32 - 1));
    flush_stdout();
    let scene_idx = match scanf_int() {
        Some(v) => v,
        None => {
            print("Invalid input\n");
            consume_to_newline();
            return;
        }
    };
    consume_to_newline();

    if scene_idx < 0 || scene_idx as usize >= scenes.len() {
        print("Invalid scene index\n");
        return;
    }

    {
        let scene = &scenes[scene_idx as usize];
        scene_list_shapes(Some(scene));

        if scene.shape_count == 0 {
            print("Scene is empty\n");
            return;
        }
        print(&format!("Select shape to remove (1-{}): ", scene.shape_count));
        flush_stdout();
    }

    let shape_idx = match scanf_int() {
        Some(v) => v,
        None => {
            print("Invalid input\n");
            consume_to_newline();
            return;
        }
    };
    consume_to_newline();

    let scene = &mut scenes[scene_idx as usize];
    if scene_remove_shape(scene, shape_idx - 1).is_ok() {
        print("Shape removed\n");
    } else {
        print("Error removing shape\n");
    }
}

fn view_scene(scenes: &Vec<Scene>) {
    if scenes.is_empty() {
        print("No scenes available\n");
        return;
    }

    print(&format!("Select scene (0-{}): ", scenes.len() as i32 - 1));
    flush_stdout();
    let scene_idx = match scanf_int() {
        Some(v) => v,
        None => {
            print("Invalid input\n");
            consume_to_newline();
            return;
        }
    };
    consume_to_newline();

    if scene_idx < 0 || scene_idx as usize >= scenes.len() {
        print("Invalid scene index\n");
        return;
    }

    scene_print(Some(&scenes[scene_idx as usize]));
}

fn list_all_scenes(scenes: &Vec<Scene>) {
    print("\n=== All Scenes ===\n");
    if scenes.is_empty() {
        print("No scenes created yet\n");
        return;
    }

    for (i, s) in scenes.iter().enumerate() {
        print(&format!("{}. {} ({} shapes)\n", i, s.name, s.shape_count));
    }
}

fn save_scene_to_file(scenes: &Vec<Scene>) {
    if scenes.is_empty() {
        print("No scenes available\n");
        return;
    }

    print(&format!("Select scene (0-{}): ", scenes.len() as i32 - 1));
    flush_stdout();
    let scene_idx = match scanf_int() {
        Some(v) => v,
        None => {
            print("Invalid input\n");
            consume_to_newline();
            return;
        }
    };
    consume_to_newline();

    if scene_idx < 0 || scene_idx as usize >= scenes.len() {
        print("Invalid scene index\n");
        return;
    }

    print("Enter filename: ");
    flush_stdout();
    let raw = match fgets(256) {
        Some(s) => s,
        None => return,
    };
    let fn_bytes: Vec<u8> = raw.into_iter().take_while(|b| *b != b'\n').collect();
    let filename = String::from_utf8_lossy(&fn_bytes).into_owned();

    scene_save(&scenes[scene_idx as usize], &filename);
}

fn load_scene_from_file(scenes: &mut Vec<Scene>) {
    if scenes.len() >= MAX_SCENES {
        print("Error: Maximum scenes reached\n");
        return;
    }

    print("Enter filename: ");
    flush_stdout();
    let raw = match fgets(256) {
        Some(s) => s,
        None => return,
    };
    let fn_bytes: Vec<u8> = raw.into_iter().take_while(|b| *b != b'\n').collect();
    let filename = String::from_utf8_lossy(&fn_bytes).into_owned();

    if let Some(scene) = scene_load(&filename) {
        scenes.push(scene);
        print(&format!("Scene loaded (index {})\n", scenes.len() - 1));
    }
}

fn compare_shapes() {
    print(&format!("\nSelect first shape (0-{}):\n", SHAPE_COUNT as i32 - 1));
    for i in 0..SHAPE_COUNT as i32 {
        print(&format!("{}. {}\n", i, shape_type_name(ShapeType::from_int(i))));
    }
    print("Choice: ");
    flush_stdout();

    let type1 = match scanf_int() {
        Some(v) => v,
        None => {
            print("Invalid input\n");
            consume_to_newline();
            return;
        }
    };
    consume_to_newline();

    print(&format!("\nSelect second shape (0-{}): ", SHAPE_COUNT as i32 - 1));
    flush_stdout();
    let type2 = match scanf_int() {
        Some(v) => v,
        None => {
            print("Invalid input\n");
            consume_to_newline();
            return;
        }
    };
    consume_to_newline();

    if type1 < 0 || type1 >= SHAPE_COUNT as i32 || type2 < 0 || type2 >= SHAPE_COUNT as i32 {
        print("Invalid shape type\n");
        return;
    }

    let s1 = shape_get(ShapeType::from_int(type1)).unwrap();
    let s2 = shape_get(ShapeType::from_int(type2)).unwrap();

    print(&format!("\nShape 1: {} (ptr: {})\n", s1.name(), ptr_format(s1 as *const _ as *const ())));
    print(&format!("Shape 2: {} (ptr: {})\n", s2.name(), ptr_format(s2 as *const _ as *const ())));
    print(&format!("Comparison of pointers: {}\n", if std::ptr::eq(s1, s2) { 1 } else { 0 }));

    if shape_equals(Some(s1), Some(s2)) != 0 {
        print("Result: Shapes are EQUAL (same instance)\n");
    } else {
        print("Result: Shapes are NOT EQUAL (different instances)\n");
    }
}

fn compare_scenes(scenes: &Vec<Scene>) {
    if scenes.len() < 2 {
        print("Need at least 2 scenes to compare\n");
        return;
    }

    print(&format!("Select first scene (0-{}): ", scenes.len() as i32 - 1));
    flush_stdout();
    let idx1 = match scanf_int() {
        Some(v) => v,
        None => {
            print("Invalid input\n");
            consume_to_newline();
            return;
        }
    };
    consume_to_newline();

    print(&format!("Select second scene (0-{}): ", scenes.len() as i32 - 1));
    flush_stdout();
    let idx2 = match scanf_int() {
        Some(v) => v,
        None => {
            print("Invalid input\n");
            consume_to_newline();
            return;
        }
    };
    consume_to_newline();

    if idx1 < 0 || idx1 as usize >= scenes.len() || idx2 < 0 || idx2 as usize >= scenes.len() {
        print("Invalid scene index\n");
        return;
    }

    let sc1 = &scenes[idx1 as usize];
    let sc2 = &scenes[idx2 as usize];

    print(&format!("\nScene 1: {} ({} shapes)\n", sc1.name, sc1.shape_count));
    scene_list_shapes(Some(sc1));

    print(&format!("\nScene 2: {} ({} shapes)\n", sc2.name, sc2.shape_count));
    scene_list_shapes(Some(sc2));

    if scene_equals(Some(sc1), Some(sc2)) != 0 {
        print("\nResult: Scenes are EQUAL (1:1 correspondence)\n");
    } else {
        print("\nResult: Scenes are NOT EQUAL\n");
    }
}

fn delete_scene(scenes: &mut Vec<Scene>) {
    if scenes.is_empty() {
        print("No scenes available\n");
        return;
    }

    print(&format!("Select scene to delete (0-{}): ", scenes.len() as i32 - 1));
    flush_stdout();
    let scene_idx = match scanf_int() {
        Some(v) => v,
        None => {
            print("Invalid input\n");
            consume_to_newline();
            return;
        }
    };
    consume_to_newline();

    if scene_idx < 0 || scene_idx as usize >= scenes.len() {
        print("Invalid scene index\n");
        return;
    }

    let s = scenes.remove(scene_idx as usize);
    scene_destroy(s);
    print("Scene deleted\n");
}

// glibc-style %p output: "0xHEXVALUE" (lowercase, no leading zeros) for non-NULL,
// "(nil)" for NULL.
pub fn ptr_format(p: *const ()) -> String {
    if p.is_null() {
        "(nil)".to_string()
    } else {
        format!("0x{:x}", p as usize)
    }
}

fn main() {
    print("\u{2554}");  // ╔
    for _ in 0..40 { print("\u{2550}"); } // ═
    print("\u{2557}\n"); // ╗
    print("\u{2551}  ASCII ART DRAWING APPLICATION        \u{2551}\n");  // ║...║
    print("\u{2551}  Child-Friendly Shape Editor           \u{2551}\n");
    print("\u{255A}");  // ╚
    for _ in 0..40 { print("\u{2550}"); }
    print("\u{255D}\n"); // ╝

    shape_manager_init();

    let mut scenes: Vec<Scene> = Vec::new();

    loop {
        print_menu();

        let raw = match fgets(256) {
            Some(s) => s,
            None => break,
        };
        // sscanf(input, "%d", &choice)
        let s = String::from_utf8_lossy(&raw);
        let trimmed = s.trim_start();
        let choice: i32 = {
            // parse leading integer from trimmed
            let mut end = 0;
            let bytes = trimmed.as_bytes();
            let mut i = 0;
            if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'+') {
                i += 1;
            }
            let start_digit = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
                end = i;
            }
            if start_digit == end {
                print("Invalid input\n");
                continue;
            }
            match trimmed[..end].parse::<i32>() {
                Ok(v) => v,
                Err(_) => {
                    print("Invalid input\n");
                    continue;
                }
            }
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
                print("\nCleaning up and exiting...\n");
                while let Some(s) = scenes.pop() {
                    scene_destroy(s);
                }
                shape_manager_cleanup();
                print("Goodbye!\n");
                flush_stdout();
                return;
            }
            _ => print("Invalid choice\n"),
        }
    }

    while let Some(s) = scenes.pop() {
        scene_destroy(s);
    }
    shape_manager_cleanup();
    flush_stdout();
}

