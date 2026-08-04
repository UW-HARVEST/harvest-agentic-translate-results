// main.rs - Translation of main.c

mod scene;
mod shape;

use scene::{
    format_ptr, scene_add_shape, scene_create, scene_equals, scene_list_shapes, scene_load,
    scene_print, scene_remove_shape, scene_save, Scene, MAX_SCENE_NAME,
};
use shape::{shape_equals, shape_print, shape_type_name, Shape, ShapeManager, SHAPE_COUNT};

use std::io::{self, Read, Write};

const MAX_SCENES: usize = 10;

/// Holds stdin reading state. C uses raw FILE* stdin which is byte-by-byte;
/// scanf reads across newlines while fgets does not.
/// We replicate this by reading raw bytes from a buffered stdin and managing
/// a peek-byte for `getchar` semantics.
struct StdinReader {
    inner: io::Stdin,
    peeked: Option<Option<u8>>, // None = no peek; Some(None) = EOF; Some(Some(b)) = b
}

impl StdinReader {
    fn new() -> Self {
        StdinReader {
            inner: io::stdin(),
            peeked: None,
        }
    }

    /// Read one byte (like C's getchar). Returns None at EOF.
    fn getchar(&mut self) -> Option<u8> {
        if let Some(p) = self.peeked.take() {
            return p;
        }
        let mut buf = [0u8; 1];
        let mut handle = self.inner.lock();
        match handle.read(&mut buf) {
            Ok(0) => None,
            Ok(_) => Some(buf[0]),
            Err(_) => None,
        }
    }

    /// fgets: read up to (n-1) bytes or until newline (newline included), null-terminate.
    /// Returns None on EOF before reading anything (matching C fgets behavior).
    /// Buffer size n is the max bytes including the null terminator.
    fn fgets(&mut self, n: usize) -> Option<String> {
        if n == 0 {
            return None;
        }
        let mut s = Vec::with_capacity(n);
        // We can read at most n-1 bytes
        let max = n - 1;
        let mut got_any = false;
        while s.len() < max {
            match self.getchar() {
                None => {
                    if !got_any {
                        return None;
                    }
                    break;
                }
                Some(b) => {
                    got_any = true;
                    s.push(b);
                    if b == b'\n' {
                        break;
                    }
                }
            }
        }
        // Convert to String preserving exact bytes (use lossy if needed; in practice ASCII)
        Some(String::from_utf8_lossy(&s).into_owned())
    }

    /// scanf("%d", &x) — skip leading whitespace (incl. newlines), read optional sign,
    /// then digits, leaving the next byte (the first non-digit, or EOF) unread.
    /// Returns Some(n) on success, None on failure (matches scanf return code).
    fn scanf_int(&mut self) -> Option<i32> {
        // Skip whitespace
        loop {
            let c = self.getchar()?;
            if !is_ws(c) {
                // push back
                self.peeked = Some(Some(c));
                break;
            }
        }

        let mut sign: i64 = 1;
        let mut started = false;
        // Check for sign
        let first = self.getchar()?;
        if first == b'+' {
            // ok
            started = false;
        } else if first == b'-' {
            sign = -1;
            started = false;
        } else if first.is_ascii_digit() {
            self.peeked = Some(Some(first));
        } else {
            // no number
            self.peeked = Some(Some(first));
            return None;
        }

        let mut value: i64 = 0;
        let mut have_digit = false;
        loop {
            match self.getchar() {
                None => break,
                Some(c) if c.is_ascii_digit() => {
                    have_digit = true;
                    value = value.wrapping_mul(10).wrapping_add((c - b'0') as i64);
                }
                Some(c) => {
                    self.peeked = Some(Some(c));
                    break;
                }
            }
        }

        if !have_digit {
            // If we consumed sign but no digit, scanf returns 0 (failure)
            // In C, the sign char is "consumed" — but since our peeked slot is taken by next char,
            // it's still effectively the same observable behavior.
            let _ = started; // suppress unused warning
            return None;
        }

        Some((sign * value) as i32)
    }
}

fn is_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

/// Print without newline, then flush so the prompt appears before stdin reads.
fn print_flush(s: &str) {
    print!("{}", s);
    let _ = io::stdout().flush();
}

/// Equivalent of `while (getchar() != '\n');` — discard until newline (or EOF).
fn discard_to_newline(stdin: &mut StdinReader) {
    loop {
        match stdin.getchar() {
            None => break,
            Some(b'\n') => break,
            _ => {}
        }
    }
}

/// Strip trailing newline (C: name[strcspn(name, "\n")] = 0;)
fn strip_newline(s: &str) -> String {
    match s.find('\n') {
        Some(i) => s[..i].to_string(),
        None => s.to_string(),
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
    print_flush("Choice: ");
}

fn view_all_shapes(mgr: &ShapeManager) {
    println!("\n=== Available Shapes ===");
    for i in 0..SHAPE_COUNT {
        print!("\n{}. ", i + 1);
        let _ = io::stdout().flush();
        shape_print(mgr.shape_get(i));
    }
}

fn create_new_scene(scenes: &mut Vec<Box<Scene>>, scene_count: &mut usize, stdin: &mut StdinReader) {
    if *scene_count >= MAX_SCENES {
        println!("Error: Maximum scenes reached");
        return;
    }

    print_flush("Enter scene name: ");
    let line = match stdin.fgets(MAX_SCENE_NAME) {
        Some(l) => l,
        None => return,
    };
    let name = strip_newline(&line);

    let scene = scene_create(Some(&name));
    println!("Scene '{}' created (index {})", name, *scene_count);
    scenes.push(scene);
    *scene_count += 1;
}

fn add_shape_to_scene(
    scenes: &mut [Box<Scene>],
    scene_count: usize,
    mgr: &ShapeManager,
    stdin: &mut StdinReader,
) {
    if scene_count == 0 {
        println!("No scenes available. Create a scene first.");
        return;
    }

    print_flush(&format!("Select scene (0-{}): ", scene_count - 1));
    let scene_idx = match stdin.scanf_int() {
        Some(n) => {
            discard_to_newline(stdin);
            n
        }
        None => {
            println!("Invalid input");
            discard_to_newline(stdin);
            return;
        }
    };

    if scene_idx < 0 || scene_idx as usize >= scene_count {
        println!("Invalid scene index");
        return;
    }

    println!("\nSelect shape to add:");
    for i in 0..SHAPE_COUNT {
        println!("{}. {}", i, shape_type_name(i));
    }
    print_flush("Choice: ");

    let shape_type = match stdin.scanf_int() {
        Some(n) => {
            discard_to_newline(stdin);
            n
        }
        None => {
            println!("Invalid input");
            discard_to_newline(stdin);
            return;
        }
    };

    if shape_type < 0 || shape_type >= SHAPE_COUNT {
        println!("Invalid shape type");
        return;
    }

    let shape = mgr.shape_get(shape_type);
    if scene_add_shape(&mut scenes[scene_idx as usize], shape) == 0 {
        // SAFETY: shape is a valid singleton pointer.
        let s: &Shape = unsafe { &*shape };
        println!(
            "Shape '{}' added to scene (reusing singleton at {})",
            s.name,
            format_ptr(shape as *const ())
        );
    } else {
        println!("Error adding shape");
    }
}

fn remove_shape_from_scene(
    scenes: &mut [Box<Scene>],
    scene_count: usize,
    stdin: &mut StdinReader,
) {
    if scene_count == 0 {
        println!("No scenes available");
        return;
    }

    print_flush(&format!("Select scene (0-{}): ", scene_count - 1));
    let scene_idx = match stdin.scanf_int() {
        Some(n) => {
            discard_to_newline(stdin);
            n
        }
        None => {
            println!("Invalid input");
            discard_to_newline(stdin);
            return;
        }
    };

    if scene_idx < 0 || scene_idx as usize >= scene_count {
        println!("Invalid scene index");
        return;
    }

    scene_list_shapes(&scenes[scene_idx as usize]);

    if scenes[scene_idx as usize].shape_count == 0 {
        println!("Scene is empty");
        return;
    }

    print_flush(&format!(
        "Select shape to remove (1-{}): ",
        scenes[scene_idx as usize].shape_count
    ));
    let shape_idx = match stdin.scanf_int() {
        Some(n) => {
            discard_to_newline(stdin);
            n
        }
        None => {
            println!("Invalid input");
            discard_to_newline(stdin);
            return;
        }
    };

    if scene_remove_shape(&mut scenes[scene_idx as usize], shape_idx - 1) == 0 {
        println!("Shape removed");
    } else {
        println!("Error removing shape");
    }
}

fn view_scene(scenes: &[Box<Scene>], scene_count: usize, stdin: &mut StdinReader) {
    if scene_count == 0 {
        println!("No scenes available");
        return;
    }

    print_flush(&format!("Select scene (0-{}): ", scene_count - 1));
    let scene_idx = match stdin.scanf_int() {
        Some(n) => {
            discard_to_newline(stdin);
            n
        }
        None => {
            println!("Invalid input");
            discard_to_newline(stdin);
            return;
        }
    };

    if scene_idx < 0 || scene_idx as usize >= scene_count {
        println!("Invalid scene index");
        return;
    }

    scene_print(&scenes[scene_idx as usize]);
}

fn list_all_scenes(scenes: &[Box<Scene>], scene_count: usize) {
    println!("\n=== All Scenes ===");
    if scene_count == 0 {
        println!("No scenes created yet");
        return;
    }
    for i in 0..scene_count {
        println!("{}. {} ({} shapes)", i, scenes[i].name, scenes[i].shape_count);
    }
}

fn save_scene_to_file(scenes: &[Box<Scene>], scene_count: usize, stdin: &mut StdinReader) {
    if scene_count == 0 {
        println!("No scenes available");
        return;
    }

    print_flush(&format!("Select scene (0-{}): ", scene_count - 1));
    let scene_idx = match stdin.scanf_int() {
        Some(n) => {
            discard_to_newline(stdin);
            n
        }
        None => {
            println!("Invalid input");
            discard_to_newline(stdin);
            return;
        }
    };

    if scene_idx < 0 || scene_idx as usize >= scene_count {
        println!("Invalid scene index");
        return;
    }

    print_flush("Enter filename: ");
    let line = match stdin.fgets(256) {
        Some(l) => l,
        None => return,
    };
    let filename = strip_newline(&line);

    scene_save(&scenes[scene_idx as usize], &filename);
}

fn load_scene_from_file(
    scenes: &mut Vec<Box<Scene>>,
    scene_count: &mut usize,
    mgr: &ShapeManager,
    stdin: &mut StdinReader,
) {
    if *scene_count >= MAX_SCENES {
        println!("Error: Maximum scenes reached");
        return;
    }

    print_flush("Enter filename: ");
    let line = match stdin.fgets(256) {
        Some(l) => l,
        None => return,
    };
    let filename = strip_newline(&line);

    if let Some(scene) = scene_load(&filename, mgr) {
        scenes.push(scene);
        *scene_count += 1;
        println!("Scene loaded (index {})", *scene_count - 1);
    }
}

fn compare_shapes(mgr: &ShapeManager, stdin: &mut StdinReader) {
    println!("\nSelect first shape (0-{}):", SHAPE_COUNT - 1);
    for i in 0..SHAPE_COUNT {
        println!("{}. {}", i, shape_type_name(i));
    }
    print_flush("Choice: ");

    let type1 = match stdin.scanf_int() {
        Some(n) => {
            discard_to_newline(stdin);
            n
        }
        None => {
            println!("Invalid input");
            discard_to_newline(stdin);
            return;
        }
    };

    print_flush(&format!("\nSelect second shape (0-{}): ", SHAPE_COUNT - 1));
    let type2 = match stdin.scanf_int() {
        Some(n) => {
            discard_to_newline(stdin);
            n
        }
        None => {
            println!("Invalid input");
            discard_to_newline(stdin);
            return;
        }
    };

    if type1 < 0 || type1 >= SHAPE_COUNT || type2 < 0 || type2 >= SHAPE_COUNT {
        println!("Invalid shape type");
        return;
    }

    let s1 = mgr.shape_get(type1);
    let s2 = mgr.shape_get(type2);

    // SAFETY: s1 and s2 are valid singleton pointers (validated above).
    let s1_ref: &Shape = unsafe { &*s1 };
    let s2_ref: &Shape = unsafe { &*s2 };

    println!("\nShape 1: {} (ptr: {})", s1_ref.name, format_ptr(s1 as *const ()));
    println!("Shape 2: {} (ptr: {})", s2_ref.name, format_ptr(s2 as *const ()));
    println!("Comparison of pointers: {}", if s1 == s2 { 1 } else { 0 });

    if shape_equals(s1, s2) != 0 {
        println!("Result: Shapes are EQUAL (same instance)");
    } else {
        println!("Result: Shapes are NOT EQUAL (different instances)");
    }
}

fn compare_scenes(scenes: &[Box<Scene>], scene_count: usize, stdin: &mut StdinReader) {
    if scene_count < 2 {
        println!("Need at least 2 scenes to compare");
        return;
    }

    print_flush(&format!("Select first scene (0-{}): ", scene_count - 1));
    let idx1 = match stdin.scanf_int() {
        Some(n) => {
            discard_to_newline(stdin);
            n
        }
        None => {
            println!("Invalid input");
            discard_to_newline(stdin);
            return;
        }
    };

    print_flush(&format!("Select second scene (0-{}): ", scene_count - 1));
    let idx2 = match stdin.scanf_int() {
        Some(n) => {
            discard_to_newline(stdin);
            n
        }
        None => {
            println!("Invalid input");
            discard_to_newline(stdin);
            return;
        }
    };

    if idx1 < 0
        || idx1 as usize >= scene_count
        || idx2 < 0
        || idx2 as usize >= scene_count
    {
        println!("Invalid scene index");
        return;
    }

    let sc1 = &scenes[idx1 as usize];
    let sc2 = &scenes[idx2 as usize];

    println!("\nScene 1: {} ({} shapes)", sc1.name, sc1.shape_count);
    scene_list_shapes(sc1);

    println!("\nScene 2: {} ({} shapes)", sc2.name, sc2.shape_count);
    scene_list_shapes(sc2);

    if scene_equals(sc1, sc2) != 0 {
        println!("\nResult: Scenes are EQUAL (1:1 correspondence)");
    } else {
        println!("\nResult: Scenes are NOT EQUAL");
    }
}

fn delete_scene(
    scenes: &mut Vec<Box<Scene>>,
    scene_count: &mut usize,
    stdin: &mut StdinReader,
) {
    if *scene_count == 0 {
        println!("No scenes available");
        return;
    }

    print_flush(&format!("Select scene to delete (0-{}): ", *scene_count - 1));
    let scene_idx = match stdin.scanf_int() {
        Some(n) => {
            discard_to_newline(stdin);
            n
        }
        None => {
            println!("Invalid input");
            discard_to_newline(stdin);
            return;
        }
    };

    if scene_idx < 0 || scene_idx as usize >= *scene_count {
        println!("Invalid scene index");
        return;
    }

    scenes.remove(scene_idx as usize);
    *scene_count -= 1;
    println!("Scene deleted");
}

fn main() {
    println!("╔════════════════════════════════════════╗");
    println!("║  ASCII ART DRAWING APPLICATION        ║");
    println!("║  Child-Friendly Shape Editor           ║");
    println!("╚════════════════════════════════════════╝");

    let mgr = ShapeManager::new();

    let mut scenes: Vec<Box<Scene>> = Vec::new();
    let mut scene_count: usize = 0;
    let mut stdin = StdinReader::new();

    loop {
        print_menu();

        // fgets reads up to 255 bytes from stdin (buffer size 256)
        let input = match stdin.fgets(256) {
            Some(l) => l,
            None => break,
        };

        // sscanf("%d", &choice) - parse leading int from line
        let choice = match parse_leading_int(&input) {
            Some(n) => n,
            None => {
                println!("Invalid input");
                continue;
            }
        };

        match choice {
            1 => view_all_shapes(&mgr),
            2 => create_new_scene(&mut scenes, &mut scene_count, &mut stdin),
            3 => add_shape_to_scene(&mut scenes, scene_count, &mgr, &mut stdin),
            4 => remove_shape_from_scene(&mut scenes, scene_count, &mut stdin),
            5 => view_scene(&scenes, scene_count, &mut stdin),
            6 => list_all_scenes(&scenes, scene_count),
            7 => save_scene_to_file(&scenes, scene_count, &mut stdin),
            8 => load_scene_from_file(&mut scenes, &mut scene_count, &mgr, &mut stdin),
            9 => compare_shapes(&mgr, &mut stdin),
            10 => compare_scenes(&scenes, scene_count, &mut stdin),
            11 => delete_scene(&mut scenes, &mut scene_count, &mut stdin),
            12 => {
                println!("\nCleaning up and exiting...");
                // Drop all scenes (matches scene_destroy loop in C).
                drop(scenes);
                drop(mgr);
                println!("Goodbye!");
                return;
            }
            _ => println!("Invalid choice"),
        }
    }
}

/// Parse a leading signed integer from a string, mimicking sscanf("%d", ...).
/// Returns None if no valid integer at the start (after optional whitespace).
fn parse_leading_int(s: &str) -> Option<i32> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && is_ws(bytes[i]) {
        i += 1;
    }
    let mut sign: i64 = 1;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if start == i {
        return None;
    }
    let digits = std::str::from_utf8(&bytes[start..i]).ok()?;
    let n: i64 = digits.parse().ok()?;
    Some((sign * n) as i32)
}
