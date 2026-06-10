// scene.rs - Translation of scene.c/scene.h

use crate::shape::{shape_equals, shape_print, Shape, ShapeManager};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

pub const MAX_SHAPES_IN_SCENE: usize = 50;
pub const MAX_SCENE_NAME: usize = 64;

pub struct Scene {
    pub name: String,
    pub shapes: Vec<*const Shape>, // pointers to singleton shapes from ShapeManager
    pub shape_count: i32,
}

pub fn scene_create(name: Option<&str>) -> Box<Scene> {
    let n = match name {
        Some(s) => {
            // strncpy(scene->name, name, MAX_SCENE_NAME - 1) - truncate to 63 bytes
            let mut s = s.to_string();
            if s.len() > MAX_SCENE_NAME - 1 {
                s.truncate(MAX_SCENE_NAME - 1);
            }
            s
        }
        None => "Untitled Scene".to_string(),
    };
    Box::new(Scene {
        name: n,
        shapes: Vec::new(),
        shape_count: 0,
    })
}

pub fn scene_add_shape(scene: &mut Scene, shape: *const Shape) -> i32 {
    if shape.is_null() {
        return -1;
    }
    if scene.shape_count as usize >= MAX_SHAPES_IN_SCENE {
        eprintln!("Error: Scene is full");
        return -1;
    }
    scene.shapes.push(shape);
    scene.shape_count += 1;
    0
}

pub fn scene_remove_shape(scene: &mut Scene, index: i32) -> i32 {
    if index < 0 || index >= scene.shape_count {
        return -1;
    }
    scene.shapes.remove(index as usize);
    scene.shape_count -= 1;
    0
}

pub fn scene_print(scene: &Scene) {
    println!("\n=== Scene: {} ===", scene.name);
    println!("Contains {} shape(s)\n", scene.shape_count);

    for i in 0..scene.shape_count as usize {
        println!("Shape #{}:", i + 1);
        shape_print(scene.shapes[i]);
        println!();
    }
}

pub fn scene_equals(s1: &Scene, s2: &Scene) -> i32 {
    if s1.shape_count != s2.shape_count {
        return 0;
    }

    let mut matched = vec![false; MAX_SHAPES_IN_SCENE];

    for i in 0..s1.shape_count as usize {
        let mut found = false;
        for j in 0..s2.shape_count as usize {
            if !matched[j] && shape_equals(s1.shapes[i], s2.shapes[j]) != 0 {
                matched[j] = true;
                found = true;
                break;
            }
        }
        if !found {
            return 0;
        }
    }
    1
}

pub fn scene_save(scene: &Scene, filename: &str) -> i32 {
    let file = match File::create(filename) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Error: Could not open file '{}' for writing", filename);
            return -1;
        }
    };

    let mut writer = std::io::BufWriter::new(file);

    // Write scene name
    if writeln!(writer, "{}", scene.name).is_err() {
        return -1;
    }

    // Write shape count
    if writeln!(writer, "{}", scene.shape_count).is_err() {
        return -1;
    }

    // Write shape types
    for i in 0..scene.shape_count as usize {
        // SAFETY: shapes pointers are valid for the lifetime of the program
        let t = unsafe { (*scene.shapes[i]).type_ };
        if writeln!(writer, "{}", t).is_err() {
            return -1;
        }
    }
    drop(writer);

    println!("Scene saved to '{}'", filename);
    0
}

pub fn scene_load(filename: &str, mgr: &ShapeManager) -> Option<Box<Scene>> {
    let file = match File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Error: Could not open file '{}' for reading", filename);
            return None;
        }
    };

    let mut reader = BufReader::new(file);

    // Read scene name with fgets-style (up to MAX_SCENE_NAME bytes including newline).
    // C uses fgets(name, MAX_SCENE_NAME, file) which reads up to MAX_SCENE_NAME-1 chars.
    let mut name_line = String::new();
    if reader.read_line(&mut name_line).unwrap_or(0) == 0 {
        return None;
    }

    // C: name[strcspn(name, "\n")] = 0; -> trim newline
    let name_trimmed = name_line.trim_end_matches('\n').trim_end_matches('\r');

    // C: fgets buffer is MAX_SCENE_NAME bytes, so name read is up to 63 chars + null.
    // After trim, take at most MAX_SCENE_NAME-1 chars (mimicking buffer size limit).
    let mut name_buf = name_trimmed.to_string();
    if name_buf.len() > MAX_SCENE_NAME - 1 {
        name_buf.truncate(MAX_SCENE_NAME - 1);
    }

    let mut scene = scene_create(Some(&name_buf));

    // fscanf("%d\n", &shape_count) - read an integer, skip whitespace.
    // We need to read remaining content and parse integers.
    let mut rest = String::new();
    use std::io::Read;
    if reader.read_to_string(&mut rest).is_err() {
        return None;
    }

    let mut tokens = rest.split_ascii_whitespace();

    let shape_count: i32 = match tokens.next().and_then(|s| s.parse().ok()) {
        Some(n) => n,
        None => return None,
    };

    for _ in 0..shape_count {
        let t: i32 = match tokens.next().and_then(|s| s.parse().ok()) {
            Some(n) => n,
            None => return None,
        };

        let shape = mgr.shape_get(t);
        if !shape.is_null() {
            scene_add_shape(&mut scene, shape);
        }
    }

    println!("Scene loaded from '{}'", filename);
    Some(scene)
}

pub fn scene_list_shapes(scene: &Scene) {
    println!("\nScene: {}", scene.name);
    println!("Shapes ({}):", scene.shape_count);

    for i in 0..scene.shape_count as usize {
        // SAFETY: shapes pointers are valid for lifetime of program
        let s = unsafe { &*scene.shapes[i] };
        println!("  {}. {} (ptr: {})", i + 1, s.name, format_ptr(scene.shapes[i] as *const ()));
    }
}

/// Format a pointer the way C's %p does on glibc: "0x" followed by lowercase hex without leading zeros.
/// On glibc Linux: printf("%p", NULL) gives "(nil)", non-null gives "0x<hex>".
pub fn format_ptr(p: *const ()) -> String {
    if p.is_null() {
        "(nil)".to_string()
    } else {
        format!("0x{:x}", p as usize)
    }
}
