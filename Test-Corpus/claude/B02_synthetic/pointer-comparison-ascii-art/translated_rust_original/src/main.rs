// main.rs - Rust translation of main.c
#[macro_use]
mod out;
mod shape;
mod scene;
mod util;

use std::io::{self, BufRead, Read};

use shape::{
    shape_equals, shape_get, shape_get_by_index, shape_manager_cleanup, shape_manager_init,
    shape_print, shape_type_name_i32, ShapeType, Shape, SHAPE_COUNT,
};
use scene::{
    scene_add_shape, scene_create, scene_destroy, scene_equals, scene_list_shapes, scene_load,
    scene_print, scene_remove_shape, scene_save, Scene, MAX_SCENE_NAME,
};

const MAX_SCENES: usize = 10;

// Global mutable state - mirrors C's static globals.
static mut SCENES: [*mut Scene; MAX_SCENES] = [std::ptr::null_mut(); MAX_SCENES];
static mut SCENE_COUNT: i32 = 0;

/// A reader that mimics C stdin behavior. We read raw bytes from stdin so
/// we can implement both line-based (fgets) and scanf-style integer reads.
struct StdinReader {
    inner: io::BufReader<io::Stdin>,
}

impl StdinReader {
    fn new() -> Self {
        Self {
            inner: io::BufReader::new(io::stdin()),
        }
    }

    /// Read a "line" up to size-1 bytes or a newline (inclusive).
    /// Returns None on EOF (matches fgets returning NULL).
    /// The trailing newline (if read) is included in the returned string.
    fn fgets(&mut self, size: usize) -> Option<String> {
        // glibc flushes line-buffered stdout when reading from stdin.
        // When stdout is a TTY it's line-buffered, so flush.
        // When stdout is block-buffered (redirected to file/pipe), don't flush
        // — this matches C's block-buffering behavior where stdout content
        // remains buffered until BUFSIZ is reached or program exits.
        out::cout_flush_if_tty();
        if size == 0 {
            return None;
        }
        let mut buf = Vec::new();
        let max = size - 1;
        let mut byte = [0u8; 1];
        let mut got_any = false;
        while buf.len() < max {
            match self.inner.read(&mut byte) {
                Ok(0) => break,
                Ok(_) => {
                    got_any = true;
                    buf.push(byte[0]);
                    if byte[0] == b'\n' {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        if !got_any {
            return None;
        }
        Some(String::from_utf8_lossy(&buf).into_owned())
    }

    /// Read characters until '\n' is read (and consume it). Mimics
    /// `while (getchar() != '\n');`. Returns on EOF.
    fn consume_line(&mut self) {
        let mut byte = [0u8; 1];
        loop {
            match self.inner.read(&mut byte) {
                Ok(0) => return,
                Ok(_) => {
                    if byte[0] == b'\n' {
                        return;
                    }
                }
                Err(_) => return,
            }
        }
    }

    /// Read an integer with scanf("%d") semantics: skip whitespace, then
    /// read optional sign and digits. Returns None on failure (no digits).
    fn scan_int(&mut self) -> Option<i32> {
        out::cout_flush_if_tty();
        // Skip whitespace using fill_buf so we can ungetc-like
        loop {
            let buf = self.inner.fill_buf().ok()?;
            if buf.is_empty() {
                return None;
            }
            let mut consumed = 0usize;
            let mut found_non_ws = false;
            for &b in buf.iter() {
                if (b as char).is_whitespace() {
                    consumed += 1;
                } else {
                    found_non_ws = true;
                    break;
                }
            }
            self.inner.consume(consumed);
            if found_non_ws {
                break;
            }
        }

        let mut sign: i32 = 1;
        let mut digits: Vec<u8> = Vec::new();

        // Check sign
        {
            let buf = self.inner.fill_buf().ok()?;
            if buf.is_empty() {
                return None;
            }
            let first = buf[0];
            if first == b'-' {
                sign = -1;
                self.inner.consume(1);
            } else if first == b'+' {
                self.inner.consume(1);
            }
        }

        // Read digits
        loop {
            let buf = match self.inner.fill_buf() {
                Ok(b) => b,
                Err(_) => break,
            };
            if buf.is_empty() {
                break;
            }
            let mut consumed = 0usize;
            let mut stop = false;
            for &b in buf.iter() {
                if b.is_ascii_digit() {
                    digits.push(b);
                    consumed += 1;
                } else {
                    stop = true;
                    break;
                }
            }
            self.inner.consume(consumed);
            if stop {
                break;
            }
        }

        if digits.is_empty() {
            return None;
        }
        let s = std::str::from_utf8(&digits).ok()?;
        s.parse::<i32>().ok().map(|v| v * sign)
    }
}

fn strip_newline(s: &mut String) {
    // C does: name[strcspn(name, "\n")] = 0;
    if let Some(pos) = s.find('\n') {
        s.truncate(pos);
    }
}

fn print_menu() {
    cprint!("\n");
    cprint!("=========================================\n");
    cprint!("  ASCII ART DRAWING APPLICATION\n");
    cprint!("=========================================\n");
    cprint!("1. View all available shapes\n");
    cprint!("2. Create new scene\n");
    cprint!("3. Add shape to scene\n");
    cprint!("4. Remove shape from scene\n");
    cprint!("5. View scene\n");
    cprint!("6. List all scenes\n");
    cprint!("7. Save scene\n");
    cprint!("8. Load scene\n");
    cprint!("9. Compare two shapes\n");
    cprint!("10. Compare two scenes\n");
    cprint!("11. Delete scene\n");
    cprint!("12. Exit\n");
    cprint!("=========================================\n");
    cprint!("Choice: ");
}

fn view_all_shapes() {
    cprint!("\n=== Available Shapes ===\n");
    for i in 0..SHAPE_COUNT {
        cprint!("\n{}. ", i + 1);
        let t = ShapeType::from_i32(i).unwrap();
        shape_print(shape_get(t) as *const Shape);
    }
}

fn create_new_scene(stdin: &mut StdinReader) {
    unsafe {
        if SCENE_COUNT as usize >= MAX_SCENES {
            cprint!("Error: Maximum scenes reached\n");
            return;
        }
    }

    cprint!("Enter scene name: ");
    let mut name = match stdin.fgets(MAX_SCENE_NAME) {
        Some(n) => n,
        None => return,
    };
    strip_newline(&mut name);

    let scene_ptr = scene_create(Some(&name));
    unsafe {
        if !scene_ptr.is_null() {
            SCENES[SCENE_COUNT as usize] = scene_ptr;
            cprint!("Scene '{}' created (index {})\n", name, SCENE_COUNT);
            SCENE_COUNT += 1;
        } else {
            cprint!("Error creating scene\n");
        }
    }
}

fn add_shape_to_scene(stdin: &mut StdinReader) {
    unsafe {
        if SCENE_COUNT == 0 {
            cprint!("No scenes available. Create a scene first.\n");
            return;
        }

        cprint!("Select scene (0-{}): ", SCENE_COUNT - 1);
        let scene_idx = match stdin.scan_int() {
            Some(v) => v,
            None => {
                cprint!("Invalid input\n");
                stdin.consume_line();
                return;
            }
        };
        stdin.consume_line();

        if scene_idx < 0 || scene_idx >= SCENE_COUNT {
            cprint!("Invalid scene index\n");
            return;
        }

        cprint!("\nSelect shape to add:\n");
        for i in 0..SHAPE_COUNT {
            cprint!("{}. {}\n", i, shape_type_name_i32(i));
        }
        cprint!("Choice: ");

        let shape_type = match stdin.scan_int() {
            Some(v) => v,
            None => {
                cprint!("Invalid input\n");
                stdin.consume_line();
                return;
            }
        };
        stdin.consume_line();

        if shape_type < 0 || shape_type >= SHAPE_COUNT {
            cprint!("Invalid shape type\n");
            return;
        }

        let shape_ptr = shape_get_by_index(shape_type);
        if scene_add_shape(SCENES[scene_idx as usize], shape_ptr) == 0 {
            let sh = &*shape_ptr;
            cprint!(
                "Shape '{}' added to scene (reusing singleton at {})\n",
                sh.name,
                util::format_ptr(shape_ptr as *const u8)
            );
        } else {
            cprint!("Error adding shape\n");
        }
    }
}

fn remove_shape_from_scene(stdin: &mut StdinReader) {
    unsafe {
        if SCENE_COUNT == 0 {
            cprint!("No scenes available\n");
            return;
        }

        cprint!("Select scene (0-{}): ", SCENE_COUNT - 1);
        let scene_idx = match stdin.scan_int() {
            Some(v) => v,
            None => {
                cprint!("Invalid input\n");
                stdin.consume_line();
                return;
            }
        };
        stdin.consume_line();

        if scene_idx < 0 || scene_idx >= SCENE_COUNT {
            cprint!("Invalid scene index\n");
            return;
        }

        scene_list_shapes(SCENES[scene_idx as usize]);

        let scene_ref = &*SCENES[scene_idx as usize];
        if scene_ref.shape_count == 0 {
            cprint!("Scene is empty\n");
            return;
        }

        cprint!(
            "Select shape to remove (1-{}): ",
            scene_ref.shape_count
        );
        let shape_idx = match stdin.scan_int() {
            Some(v) => v,
            None => {
                cprint!("Invalid input\n");
                stdin.consume_line();
                return;
            }
        };
        stdin.consume_line();

        if scene_remove_shape(SCENES[scene_idx as usize], shape_idx - 1) == 0 {
            cprint!("Shape removed\n");
        } else {
            cprint!("Error removing shape\n");
        }
    }
}

fn view_scene(stdin: &mut StdinReader) {
    unsafe {
        if SCENE_COUNT == 0 {
            cprint!("No scenes available\n");
            return;
        }

        cprint!("Select scene (0-{}): ", SCENE_COUNT - 1);
        let scene_idx = match stdin.scan_int() {
            Some(v) => v,
            None => {
                cprint!("Invalid input\n");
                stdin.consume_line();
                return;
            }
        };
        stdin.consume_line();

        if scene_idx < 0 || scene_idx >= SCENE_COUNT {
            cprint!("Invalid scene index\n");
            return;
        }

        scene_print(SCENES[scene_idx as usize]);
    }
}

fn list_all_scenes() {
    cprint!("\n=== All Scenes ===\n");
    unsafe {
        if SCENE_COUNT == 0 {
            cprint!("No scenes created yet\n");
            return;
        }
        for i in 0..SCENE_COUNT {
            let s = &*SCENES[i as usize];
            cprint!("{}. {} ({} shapes)\n", i, s.name, s.shape_count);
        }
    }
}

fn save_scene_to_file(stdin: &mut StdinReader) {
    unsafe {
        if SCENE_COUNT == 0 {
            cprint!("No scenes available\n");
            return;
        }

        cprint!("Select scene (0-{}): ", SCENE_COUNT - 1);
        let scene_idx = match stdin.scan_int() {
            Some(v) => v,
            None => {
                cprint!("Invalid input\n");
                stdin.consume_line();
                return;
            }
        };
        stdin.consume_line();

        if scene_idx < 0 || scene_idx >= SCENE_COUNT {
            cprint!("Invalid scene index\n");
            return;
        }

        cprint!("Enter filename: ");
        let mut filename = match stdin.fgets(256) {
            Some(s) => s,
            None => return,
        };
        strip_newline(&mut filename);

        scene_save(SCENES[scene_idx as usize], &filename);
    }
}

fn load_scene_from_file(stdin: &mut StdinReader) {
    unsafe {
        if SCENE_COUNT as usize >= MAX_SCENES {
            cprint!("Error: Maximum scenes reached\n");
            return;
        }

        cprint!("Enter filename: ");
        let mut filename = match stdin.fgets(256) {
            Some(s) => s,
            None => return,
        };
        strip_newline(&mut filename);

        let scene_ptr = scene_load(&filename);
        if !scene_ptr.is_null() {
            SCENES[SCENE_COUNT as usize] = scene_ptr;
            SCENE_COUNT += 1;
            cprint!("Scene loaded (index {})\n", SCENE_COUNT - 1);
        }
    }
}

fn compare_shapes(stdin: &mut StdinReader) {
    cprint!("\nSelect first shape (0-{}):\n", SHAPE_COUNT - 1);
    for i in 0..SHAPE_COUNT {
        cprint!("{}. {}\n", i, shape_type_name_i32(i));
    }
    cprint!("Choice: ");

    let type1 = match stdin.scan_int() {
        Some(v) => v,
        None => {
            cprint!("Invalid input\n");
            stdin.consume_line();
            return;
        }
    };
    stdin.consume_line();

    cprint!("\nSelect second shape (0-{}): ", SHAPE_COUNT - 1);
    let type2 = match stdin.scan_int() {
        Some(v) => v,
        None => {
            cprint!("Invalid input\n");
            stdin.consume_line();
            return;
        }
    };
    stdin.consume_line();

    if type1 < 0 || type1 >= SHAPE_COUNT || type2 < 0 || type2 >= SHAPE_COUNT {
        cprint!("Invalid shape type\n");
        return;
    }

    let s1 = shape_get_by_index(type1);
    let s2 = shape_get_by_index(type2);

    unsafe {
        let s1r = &*s1;
        let s2r = &*s2;
        cprint!(
            "\nShape 1: {} (ptr: {})\n",
            s1r.name,
            util::format_ptr(s1 as *const u8)
        );
        cprint!(
            "Shape 2: {} (ptr: {})\n",
            s2r.name,
            util::format_ptr(s2 as *const u8)
        );
        // C's `s1 == s2` returns 0 or 1 (int)
        let cmp = if std::ptr::eq(s1, s2) { 1 } else { 0 };
        cprint!("Comparison of pointers: {}\n", cmp);

        if shape_equals(s1 as *const Shape, s2 as *const Shape) != 0 {
            cprint!("Result: Shapes are EQUAL (same instance)\n");
        } else {
            cprint!("Result: Shapes are NOT EQUAL (different instances)\n");
        }
    }
}

fn compare_scenes(stdin: &mut StdinReader) {
    unsafe {
        if SCENE_COUNT < 2 {
            cprint!("Need at least 2 scenes to compare\n");
            return;
        }

        cprint!("Select first scene (0-{}): ", SCENE_COUNT - 1);
        let idx1 = match stdin.scan_int() {
            Some(v) => v,
            None => {
                cprint!("Invalid input\n");
                stdin.consume_line();
                return;
            }
        };
        stdin.consume_line();

        cprint!("Select second scene (0-{}): ", SCENE_COUNT - 1);
        let idx2 = match stdin.scan_int() {
            Some(v) => v,
            None => {
                cprint!("Invalid input\n");
                stdin.consume_line();
                return;
            }
        };
        stdin.consume_line();

        if idx1 < 0 || idx1 >= SCENE_COUNT || idx2 < 0 || idx2 >= SCENE_COUNT {
            cprint!("Invalid scene index\n");
            return;
        }

        let sc1 = SCENES[idx1 as usize];
        let sc2 = SCENES[idx2 as usize];
        let sc1r = &*sc1;
        let sc2r = &*sc2;

        cprint!("\nScene 1: {} ({} shapes)\n", sc1r.name, sc1r.shape_count);
        scene_list_shapes(sc1);

        cprint!("\nScene 2: {} ({} shapes)\n", sc2r.name, sc2r.shape_count);
        scene_list_shapes(sc2);

        if scene_equals(sc1 as *const Scene, sc2 as *const Scene) != 0 {
            cprint!("\nResult: Scenes are EQUAL (1:1 correspondence)\n");
        } else {
            cprint!("\nResult: Scenes are NOT EQUAL\n");
        }
    }
}

fn delete_scene(stdin: &mut StdinReader) {
    unsafe {
        if SCENE_COUNT == 0 {
            cprint!("No scenes available\n");
            return;
        }

        cprint!("Select scene to delete (0-{}): ", SCENE_COUNT - 1);
        let scene_idx = match stdin.scan_int() {
            Some(v) => v,
            None => {
                cprint!("Invalid input\n");
                stdin.consume_line();
                return;
            }
        };
        stdin.consume_line();

        if scene_idx < 0 || scene_idx >= SCENE_COUNT {
            cprint!("Invalid scene index\n");
            return;
        }

        scene_destroy(SCENES[scene_idx as usize]);

        let mut i = scene_idx;
        while i < SCENE_COUNT - 1 {
            SCENES[i as usize] = SCENES[(i + 1) as usize];
            i += 1;
        }

        SCENE_COUNT -= 1;
        cprint!("Scene deleted\n");
    }
}

fn main() {
    cprint!("╔════════════════════════════════════════╗\n");
    cprint!("║  ASCII ART DRAWING APPLICATION        ║\n");
    cprint!("║  Child-Friendly Shape Editor           ║\n");
    cprint!("╚════════════════════════════════════════╝\n");

    shape_manager_init();

    let mut stdin = StdinReader::new();

    loop {
        print_menu();

        let input = match stdin.fgets(256) {
            Some(s) => s,
            None => break,
        };

        // sscanf-style parse: skip leading whitespace, read int.
        let trimmed = input.trim_start();
        let choice_str: String = trimmed
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '-' || *c == '+')
            .collect();
        let choice: i32 = match choice_str.parse() {
            Ok(v) => v,
            Err(_) => {
                cprint!("Invalid input\n");
                continue;
            }
        };

        match choice {
            1 => view_all_shapes(),
            2 => create_new_scene(&mut stdin),
            3 => add_shape_to_scene(&mut stdin),
            4 => remove_shape_from_scene(&mut stdin),
            5 => view_scene(&mut stdin),
            6 => list_all_scenes(),
            7 => save_scene_to_file(&mut stdin),
            8 => load_scene_from_file(&mut stdin),
            9 => compare_shapes(&mut stdin),
            10 => compare_scenes(&mut stdin),
            11 => delete_scene(&mut stdin),
            12 => {
                cprint!("\nCleaning up and exiting...\n");
                unsafe {
                    for i in 0..SCENE_COUNT {
                        scene_destroy(SCENES[i as usize]);
                    }
                }
                shape_manager_cleanup();
                cprint!("Goodbye!\n");
                out::cout_flush();
                return;
            }
            _ => {
                cprint!("Invalid choice\n");
            }
        }
    }

    // Cleanup on EOF break
    unsafe {
        for i in 0..SCENE_COUNT {
            scene_destroy(SCENES[i as usize]);
        }
    }
    shape_manager_cleanup();
    out::cout_flush();
}
