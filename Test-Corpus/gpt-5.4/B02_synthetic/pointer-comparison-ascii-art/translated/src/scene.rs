use crate::shape::{shape_equals, shape_get, shape_print, Shape, ShapeType};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

pub const MAX_SHAPES_IN_SCENE: usize = 50;
pub const MAX_SCENE_NAME: usize = 64;

#[derive(Debug)]
pub struct Scene {
    pub name: String,
    pub shapes: Vec<&'static Shape>,
}

pub type SceneT = Scene;

pub fn scene_create(name: Option<&str>) -> Option<Box<Scene>> {
    let scene_name = match name {
        Some(name) => name.chars().take(MAX_SCENE_NAME - 1).collect(),
        None => "Untitled Scene".to_string(),
    };

    Some(Box::new(Scene {
        name: scene_name,
        shapes: Vec::new(),
    }))
}

pub fn scene_destroy(_scene: Box<Scene>) {}

pub fn scene_add_shape(scene: &mut Scene, shape: &'static Shape) -> i32 {
    if scene.shapes.len() >= MAX_SHAPES_IN_SCENE {
        eprintln!("Error: Scene is full");
        return -1;
    }

    scene.shapes.push(shape);
    0
}

pub fn scene_remove_shape(scene: &mut Scene, index: i32) -> i32 {
    if index < 0 || index as usize >= scene.shapes.len() {
        return -1;
    }

    scene.shapes.remove(index as usize);
    0
}

pub fn scene_print(scene: Option<&Scene>) {
    match scene {
        Some(scene) => {
            println!("\n=== Scene: {} ===", scene.name);
            println!("Contains {} shape(s)\n", scene.shapes.len());
            for (i, shape) in scene.shapes.iter().enumerate() {
                println!("Shape #{}:", i + 1);
                shape_print(Some(shape));
                println!();
            }
        }
        None => println!("(null scene)"),
    }
}

pub fn scene_equals(s1: Option<&Scene>, s2: Option<&Scene>) -> i32 {
    let (Some(s1), Some(s2)) = (s1, s2) else {
        return 0;
    };

    if s1.shapes.len() != s2.shapes.len() {
        return 0;
    }

    let mut matched = vec![false; MAX_SHAPES_IN_SCENE];

    for shape1 in &s1.shapes {
        let mut found = false;
        for (j, shape2) in s2.shapes.iter().enumerate() {
            if !matched[j] && shape_equals(Some(shape1), Some(shape2)) == 1 {
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
    let (Some(scene), Some(filename)) = (scene, filename) else {
        return -1;
    };

    let mut file = match File::create(filename) {
        Ok(file) => file,
        Err(_) => {
            eprintln!("Error: Could not open file '{}' for writing", filename);
            return -1;
        }
    };

    if writeln!(file, "{}", scene.name).is_err() {
        return -1;
    }
    if writeln!(file, "{}", scene.shapes.len()).is_err() {
        return -1;
    }
    for shape in &scene.shapes {
        if writeln!(file, "{}", shape.type_ as i32).is_err() {
            return -1;
        }
    }

    println!("Scene saved to '{}'", filename);
    0
}

pub fn scene_load(filename: Option<&str>) -> Option<Box<Scene>> {
    let filename = filename?;
    let file = match File::open(filename) {
        Ok(file) => file,
        Err(_) => {
            eprintln!("Error: Could not open file '{}' for reading", filename);
            return None;
        }
    };

    let mut reader = BufReader::new(file);
    let mut name = String::new();
    if reader.read_line(&mut name).ok()? == 0 {
        return None;
    }
    while name.ends_with('\n') || name.ends_with('\r') {
        name.pop();
    }

    let mut scene = scene_create(Some(&name))?;

    let mut count_line = String::new();
    if reader.read_line(&mut count_line).ok()? == 0 {
        return None;
    }
    let shape_count: usize = count_line.trim().parse().ok()?;

    for _ in 0..shape_count {
        let mut type_line = String::new();
        if reader.read_line(&mut type_line).ok()? == 0 {
            return None;
        }
        let type_value: i32 = match type_line.trim().parse() {
            Ok(v) => v,
            Err(_) => return None,
        };
        if let Some(shape_type) = ShapeType::from_i32(type_value) {
            if let Some(shape) = shape_get(shape_type) {
                let _ = scene_add_shape(&mut scene, shape);
            }
        }
    }

    println!("Scene loaded from '{}'", filename);
    Some(scene)
}

pub fn scene_list_shapes(scene: Option<&Scene>) {
    match scene {
        Some(scene) => {
            println!("\nScene: {}", scene.name);
            println!("Shapes ({}):", scene.shapes.len());
            for (i, shape) in scene.shapes.iter().enumerate() {
                println!("  {}. {} (ptr: {:p})", i + 1, shape.name, *shape);
            }
        }
        None => println!("(null scene)"),
    }
}
