// scene.rs - Translation of scene.c/scene.h to Rust

use std::fs::File;
use std::io::{BufRead, BufReader, Write};

use crate::shape::{shape_equals, shape_get_by_index, shape_print, Shape};

pub const MAX_SHAPES_IN_SCENE: usize = 50;
pub const MAX_SCENE_NAME: usize = 64;

pub struct Scene {
    pub name: String,
    pub shapes: Vec<&'static Shape>,
    pub shape_count: i32,
}

impl Scene {
    pub fn capacity() -> usize {
        MAX_SHAPES_IN_SCENE
    }
}

pub fn scene_create(name: Option<&str>) -> Option<Box<Scene>> {
    let scene_name = match name {
        Some(s) if !s.is_empty() => {
            // Truncate to MAX_SCENE_NAME - 1 bytes (mimic strncpy semantics)
            let max_len = MAX_SCENE_NAME - 1;
            if s.len() > max_len {
                // Take a safe UTF-8 prefix.
                let mut idx = max_len;
                while idx > 0 && !s.is_char_boundary(idx) {
                    idx -= 1;
                }
                s[..idx].to_string()
            } else {
                s.to_string()
            }
        }
        Some(_) => "Untitled Scene".to_string(),
        None => "Untitled Scene".to_string(),
    };

    Some(Box::new(Scene {
        name: scene_name,
        shapes: Vec::with_capacity(MAX_SHAPES_IN_SCENE),
        shape_count: 0,
    }))
}

pub fn scene_destroy(_scene: Box<Scene>) {
    // Box drops automatically; shapes are not owned by the scene.
}

pub fn scene_add_shape(scene: &mut Scene, shape: Option<&'static Shape>) -> i32 {
    let shape = match shape {
        Some(s) => s,
        None => return -1,
    };

    if (scene.shape_count as usize) >= MAX_SHAPES_IN_SCENE {
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

pub fn scene_print(scene: Option<&Scene>) {
    let scene = match scene {
        Some(s) => s,
        None => {
            println!("(null scene)");
            return;
        }
    };

    println!("\n=== Scene: {} ===", scene.name);
    println!("Contains {} shape(s)\n", scene.shape_count);

    for (i, shape) in scene.shapes.iter().enumerate() {
        println!("Shape #{}:", i + 1);
        shape_print(Some(*shape));
        println!();
    }
}

pub fn scene_equals(s1: Option<&Scene>, s2: Option<&Scene>) -> i32 {
    let (s1, s2) = match (s1, s2) {
        (Some(a), Some(b)) => (a, b),
        _ => return 0,
    };

    if s1.shape_count != s2.shape_count {
        return 0;
    }

    let mut matched = vec![false; MAX_SHAPES_IN_SCENE];
    for i in 0..(s1.shape_count as usize) {
        let mut found = false;
        for j in 0..(s2.shape_count as usize) {
            if !matched[j] && shape_equals(Some(s1.shapes[i]), Some(s2.shapes[j])) != 0 {
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

pub fn scene_save(scene: Option<&Scene>, filename: Option<&str>) -> i32 {
    let scene = match scene {
        Some(s) => s,
        None => return -1,
    };
    let filename = match filename {
        Some(f) => f,
        None => return -1,
    };

    let file = match File::create(filename) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Error: Could not open file '{}' for writing", filename);
            return -1;
        }
    };

    let mut writer = std::io::BufWriter::new(file);
    if writeln!(writer, "{}", scene.name).is_err() {
        return -1;
    }
    if writeln!(writer, "{}", scene.shape_count).is_err() {
        return -1;
    }
    for shape in scene.shapes.iter() {
        if writeln!(writer, "{}", shape.shape_type.as_i32()).is_err() {
            return -1;
        }
    }
    if writer.flush().is_err() {
        return -1;
    }
    println!("Scene saved to '{}'", filename);
    0
}

pub fn scene_load(filename: Option<&str>) -> Option<Box<Scene>> {
    let filename = filename?;

    let file = match File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Error: Could not open file '{}' for reading", filename);
            return None;
        }
    };

    let mut reader = BufReader::new(file);
    let mut name_line = String::new();
    if reader.read_line(&mut name_line).ok()? == 0 {
        return None;
    }
    // Strip trailing newline characters (mimic strcspn)
    let name_trimmed = name_line.trim_end_matches(|c| c == '\n' || c == '\r');

    let mut scene = scene_create(Some(name_trimmed))?;

    let mut count_line = String::new();
    if reader.read_line(&mut count_line).ok()? == 0 {
        return None;
    }
    let shape_count: i32 = match count_line.trim().parse() {
        Ok(c) => c,
        Err(_) => return None,
    };

    for _ in 0..shape_count {
        let mut type_line = String::new();
        if reader.read_line(&mut type_line).ok()? == 0 {
            return None;
        }
        let type_val: i32 = match type_line.trim().parse() {
            Ok(t) => t,
            Err(_) => return None,
        };

        if let Some(shape) = shape_get_by_index(type_val) {
            scene_add_shape(&mut scene, Some(shape));
        }
    }

    println!("Scene loaded from '{}'", filename);
    Some(scene)
}

pub fn scene_list_shapes(scene: Option<&Scene>) {
    let scene = match scene {
        Some(s) => s,
        None => {
            println!("(null scene)");
            return;
        }
    };

    println!("\nScene: {}", scene.name);
    println!("Shapes ({}):", scene.shape_count);

    for (i, shape) in scene.shapes.iter().enumerate() {
        let ptr = *shape as *const Shape;
        println!("  {}. {} (ptr: {:p})", i + 1, shape.name, ptr);
    }
}
