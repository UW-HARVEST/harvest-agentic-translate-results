use crate::shape::{shape_equals, shape_get, shape_print, Shape, ShapeType};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

pub struct Scene {
    pub name: String,
    pub shapes: Vec<&'static Shape>,
}

impl Scene {
    pub fn new(name: &str) -> Self {
        Self {
            name: if name.is_empty() {
                "Untitled Scene".to_string()
            } else {
                name.to_string()
            },
            shapes: Vec::new(),
        }
    }

    pub fn add_shape(&mut self, shape: &'static Shape) -> Result<(), &'static str> {
        if self.shapes.len() >= 50 {
            eprintln!("Error: Scene is full");
            Err("Scene is full")
        } else {
            self.shapes.push(shape);
            Ok(())
        }
    }

    pub fn remove_shape(&mut self, index: usize) -> Result<(), &'static str> {
        if index < self.shapes.len() {
            self.shapes.remove(index);
            Ok(())
        } else {
            Err("Invalid index")
        }
    }

    pub fn print(&self) {
        println!("\n=== Scene: {} ===", self.name);
        println!("Contains {} shape(s)\n", self.shapes.len());

        for (i, shape) in self.shapes.iter().enumerate() {
            println!("Shape #{}:", i + 1);
            shape_print(Some(*shape));
            println!();
        }
    }

    pub fn equals(&self, other: &Scene) -> bool {
        if self.shapes.len() != other.shapes.len() {
            return false;
        }

        let mut matched = vec![false; other.shapes.len()];

        for s1 in &self.shapes {
            let mut found = false;
            for (j, s2) in other.shapes.iter().enumerate() {
                if !matched[j] && shape_equals(Some(*s1), Some(*s2)) {
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

    pub fn save(&self, filename: &str) -> std::io::Result<()> {
        let mut file = File::create(filename)?;
        writeln!(file, "{}", self.name)?;
        writeln!(file, "{}", self.shapes.len())?;
        for shape in &self.shapes {
            writeln!(file, "{}", shape.shape_type as usize)?;
        }
        println!("Scene saved to '{}'", filename);
        Ok(())
    }

    pub fn load(filename: &str) -> std::io::Result<Self> {
        let file = File::open(filename)?;
        let mut lines = BufReader::new(file).lines();

        let name = lines
            .next()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Missing name"))??;
        let mut scene = Scene::new(&name);

        let count_str = lines
            .next()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Missing count"))??;
        let count: usize = count_str
            .parse()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid count"))?;

        for _ in 0..count {
            let type_str = lines.next().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "Missing shape type")
            })??;
            let type_val: usize = type_str.parse().map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid shape type")
            })?;

            if let Some(shape_type) = ShapeType::from_usize(type_val) {
                if let Some(shape) = shape_get(shape_type) {
                    let _ = scene.add_shape(shape);
                }
            }
        }

        println!("Scene loaded from '{}'", filename);
        Ok(scene)
    }

    pub fn list_shapes(&self) {
        println!("\nScene: {}", self.name);
        println!("Shapes ({}):", self.shapes.len());
        for (i, shape) in self.shapes.iter().enumerate() {
            println!("  {}. {} (ptr: {:p})", i + 1, shape.name, *shape);
        }
    }
}
