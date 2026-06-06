// scene.rs — translation of c_src/src/scene.c
//
// Scenes hold pointers (as `usize`) into the shape singletons, not owned
// copies, so that pointer-equality comparisons are preserved.

use std::fs::File;
use std::io::{BufRead, BufReader, Write};

use crate::shape;

pub const MAX_SHAPES_IN_SCENE: usize = 50;
pub const MAX_SCENE_NAME: usize = 64;

pub struct Scene {
    pub name: String,
    pub shapes: Vec<usize>, // pointer values into the shape singletons
}

pub fn scene_create(name: &str) -> Scene {
    // Mirror C's strncpy(name, MAX_SCENE_NAME-1) — at most 63 bytes.
    let bytes = name.as_bytes();
    let take = bytes.len().min(MAX_SCENE_NAME - 1);
    let truncated = String::from_utf8_lossy(&bytes[..take]).into_owned();
    let final_name = if name.is_empty() {
        // C path uses strncpy of "" — which yields an empty string, not the
        // "Untitled Scene" branch (that branch only triggers when `name` is
        // NULL). Empty scene names are valid here.
        String::new()
    } else {
        truncated
    };
    Scene {
        name: final_name,
        shapes: Vec::new(),
    }
}

pub fn scene_add_shape(scene: &mut Scene, shape_ptr: usize) -> i32 {
    if shape_ptr == 0 {
        return -1;
    }
    if scene.shapes.len() >= MAX_SHAPES_IN_SCENE {
        eprintln!("Error: Scene is full");
        return -1;
    }
    scene.shapes.push(shape_ptr);
    0
}

/// In C, `scene_destroy(scene)` calls `free(scene)` but does NOT free the
/// shapes (they're singletons owned by the shape manager). In Rust, dropping
/// the Scene already does the right thing because `shapes` only holds raw
/// pointer values. This helper is provided to make the call sites read like
/// the C code.
pub fn scene_destroy_noop(scene: Option<Scene>) {
    drop(scene);
}

pub fn scene_remove_shape(scene: &mut Scene, index: i32) -> i32 {
    if index < 0 || (index as usize) >= scene.shapes.len() {
        return -1;
    }
    scene.shapes.remove(index as usize);
    0
}

pub fn scene_print(scene: &Scene) {
    println!();
    println!("=== Scene: {} ===", scene.name);
    println!("Contains {} shape(s)", scene.shapes.len());
    println!();

    for (i, ptr) in scene.shapes.iter().enumerate() {
        println!("Shape #{}:", i + 1);
        shape::shape_print(*ptr);
        println!();
    }
}

pub fn scene_equals(s1: &Scene, s2: &Scene) -> bool {
    if s1.shapes.len() != s2.shapes.len() {
        return false;
    }
    let mut matched = vec![false; s2.shapes.len()];
    for &p1 in &s1.shapes {
        let mut found = false;
        for (j, &p2) in s2.shapes.iter().enumerate() {
            if !matched[j] && shape::shape_equals(p1, p2) {
                matched[j] = true;
                found = true;
                break;
            }
        }
        if !found {
            return false;
        }
    }
    true
}

pub fn scene_save(scene: &Scene, filename: &str) -> i32 {
    let file = match File::create(filename) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Error: Could not open file '{}' for writing", filename);
            return -1;
        }
    };
    let mut file = file;
    let res = (|| -> std::io::Result<()> {
        writeln!(file, "{}", scene.name)?;
        writeln!(file, "{}", scene.shapes.len())?;
        for &p in &scene.shapes {
            let t = shape::shape_type_id(p);
            writeln!(file, "{}", t)?;
        }
        Ok(())
    })();
    if res.is_err() {
        // If a write error occurs, the C version still attempts fclose and
        // prints "Scene saved..."; we mirror that quirk by ignoring the error.
    }
    drop(file);
    println!("Scene saved to '{}'", filename);
    0
}

pub fn scene_load(filename: &str) -> Option<Scene> {
    let file = match File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Error: Could not open file '{}' for reading", filename);
            return None;
        }
    };
    let mut reader = BufReader::new(file);

    // Read the first line as the scene name (mirrors fgets with MAX_SCENE_NAME).
    let mut name_line = String::new();
    match reader.read_line(&mut name_line) {
        Ok(0) => return None,
        Ok(_) => {}
        Err(_) => return None,
    }
    // Strip trailing '\n' (mirrors strcspn(name, "\n")).
    if let Some(pos) = name_line.find('\n') {
        name_line.truncate(pos);
    }
    // Mirror fgets max-size truncation: at most MAX_SCENE_NAME - 1 bytes.
    if name_line.len() >= MAX_SCENE_NAME {
        name_line.truncate(MAX_SCENE_NAME - 1);
    }

    let mut scene = scene_create(&name_line);

    // Read shape_count via fscanf("%d\n", ...).
    let shape_count = match read_int_line(&mut reader) {
        Some(n) => n,
        None => return None,
    };

    for _ in 0..shape_count {
        let t = match read_int_line(&mut reader) {
            Some(n) => n,
            None => return None,
        };
        if t >= 0 && t < shape::SHAPE_COUNT {
            let ptr = shape::shape_get_ptr(t);
            if ptr != 0 {
                scene_add_shape(&mut scene, ptr);
            }
        }
    }

    println!("Scene loaded from '{}'", filename);
    Some(scene)
}

fn read_int_line<R: BufRead>(reader: &mut R) -> Option<i32> {
    // fscanf("%d\n", ...) skips leading whitespace, parses an integer, then
    // matches optional whitespace. Reading line-by-line and parsing is a
    // close-enough approximation for save-files produced by scene_save.
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => return None,
            Ok(_) => {}
            Err(_) => return None,
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        return trimmed.parse::<i32>().ok();
    }
}

pub fn scene_list_shapes(scene: &Scene) {
    println!();
    println!("Scene: {}", scene.name);
    println!("Shapes ({}):", scene.shapes.len());
    for (i, &ptr) in scene.shapes.iter().enumerate() {
        let name = shape::shape_name(ptr);
        let p = shape::fmt_ptr(ptr);
        println!("  {}. {} (ptr: {})", i + 1, name, p);
    }
}
