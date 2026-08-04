use std::fs::File;
use std::io::{BufReader, Write};

use crate::input::{ByteReader, StreamReader};
use crate::shape::{shape_equals, shape_print, Shape, ShapeManager, ShapeType};

pub const MAX_SHAPES_IN_SCENE: usize = 50;
pub const MAX_SCENE_NAME: usize = 64;

pub struct Scene {
    pub name: String,
    pub shapes: Vec<ShapeType>,
}

impl Scene {
    pub fn create(name: Option<&str>) -> Self {
        let scene_name = match name {
            Some(name) => truncate_bytes(name, MAX_SCENE_NAME - 1),
            None => String::from("Untitled Scene"),
        };

        Self {
            name: scene_name,
            shapes: Vec::new(),
        }
    }

    pub fn add_shape(&mut self, shape: Option<&Shape>) -> i32 {
        if shape.is_none() {
            return -1;
        }

        if self.shapes.len() >= MAX_SHAPES_IN_SCENE {
            eprintln!("Error: Scene is full");
            return -1;
        }

        self.shapes.push(shape.unwrap().shape_type);
        0
    }

    pub fn remove_shape(&mut self, index: i32) -> i32 {
        if index < 0 || index as usize >= self.shapes.len() {
            return -1;
        }

        self.shapes.remove(index as usize);
        0
    }

    pub fn print(&self, shapes: &ShapeManager) {
        println!("\n=== Scene: {} ===", self.name);
        println!("Contains {} shape(s)\n", self.shapes.len());

        for (index, shape_type) in self.shapes.iter().enumerate() {
            println!("Shape #{}:", index + 1);
            shape_print(Some(shapes.get(*shape_type)));
            println!();
        }
    }

    pub fn equals(&self, other: &Self, shapes: &ShapeManager) -> bool {
        if self.shapes.len() != other.shapes.len() {
            return false;
        }

        let mut matched = [false; MAX_SHAPES_IN_SCENE];
        for shape_type in &self.shapes {
            let mut found = false;
            for (index, other_type) in other.shapes.iter().enumerate() {
                if !matched[index]
                    && shape_equals(shapes.get(*shape_type), shapes.get(*other_type))
                {
                    matched[index] = true;
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

    pub fn save(&self, filename: &str) -> i32 {
        let mut file = match File::create(filename) {
            Ok(file) => file,
            Err(_) => {
                eprintln!("Error: Could not open file '{}' for writing", filename);
                return -1;
            }
        };

        let _ = writeln!(file, "{}", self.name);
        let _ = writeln!(file, "{}", self.shapes.len());
        for shape_type in &self.shapes {
            let _ = writeln!(file, "{}", *shape_type as i32);
        }

        println!("Scene saved to '{}'", filename);
        0
    }

    pub fn load(filename: &str, shapes: &ShapeManager) -> Option<Self> {
        let file = match File::open(filename) {
            Ok(file) => file,
            Err(_) => {
                eprintln!("Error: Could not open file '{}' for reading", filename);
                return None;
            }
        };

        let mut reader = StreamReader::new(BufReader::new(file));
        let mut name = reader.fgets(MAX_SCENE_NAME)?;
        strip_first_newline(&mut name);

        let mut scene = Self::create(Some(&name));

        let shape_count = match fscanf_d_newline(&mut reader) {
            Some(value) => value,
            None => return None,
        };

        for _ in 0..shape_count {
            let shape_type = match fscanf_d_newline(&mut reader) {
                Some(value) => value,
                None => return None,
            };

            let shape = shapes.get_by_i32(shape_type);
            let _ = scene.add_shape(shape);
        }

        println!("Scene loaded from '{}'", filename);
        Some(scene)
    }

    pub fn list_shapes(&self, shapes: &ShapeManager) {
        println!("\nScene: {}", self.name);
        println!("Shapes ({}):", self.shapes.len());

        for (index, shape_type) in self.shapes.iter().enumerate() {
            let shape = shapes.get(*shape_type);
            println!(
                "  {}. {} (ptr: {:p})",
                index + 1,
                shape.name,
                shape as *const Shape
            );
        }
    }
}

pub fn truncate_bytes(text: &str, max_len: usize) -> String {
    let bytes = text.as_bytes();
    let end = bytes.len().min(max_len);
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

pub fn strip_first_newline(text: &mut String) {
    if let Some(index) = text.find('\n') {
        text.truncate(index);
    }
}

fn fscanf_d_newline<R: std::io::Read>(reader: &mut StreamReader<R>) -> Option<i32> {
    let value = reader.scanf_d()?;
    while matches!(reader.peek(), Some(byte) if byte.is_ascii_whitespace()) {
        reader.getchar();
    }
    Some(value)
}
