use std::fs::File;
use std::io::{self, BufRead, Write};
use std::path::Path;

use crate::shape::{Shape, ShapeManager, ShapeType};

const MAX_SHAPES_IN_SCENE: usize = 50;
const MAX_SCENE_NAME: usize = 64;

pub struct Scene {
    name: String,
    shapes: Vec<&'static Shape>,
}

impl Scene {
    pub fn new(name: &str) -> Self {
        let name = if name.is_empty() {
            "Untitled Scene".to_string()
        } else {
            name.chars().take(MAX_SCENE_NAME).collect()
        };
        
        Scene {
            name,
            shapes: Vec::new(),
        }
    }
    
    pub fn name(&self) -> &str {
        &self.name
    }
    
    pub fn shape_count(&self) -> usize {
        self.shapes.len()
    }
    
    pub fn add_shape(&mut self, shape: &'static Shape) -> Result<(), ()> {
        if self.shapes.len() >= MAX_SHAPES_IN_SCENE {
            eprintln!("Error: Scene is full");
            return Err(());
        }
        
        self.shapes.push(shape);
        Ok(())
    }
    
    pub fn remove_shape(&mut self, index: usize) -> Result<(), ()> {
        if index >= self.shapes.len() {
            return Err(());
        }
        
        self.shapes.remove(index);
        Ok(())
    }
    
    pub fn print(&self) {
        println!("\n=== Scene: {} ===", self.name);
        println!("Contains {} shape(s)\n", self.shapes.len());
        
        for (i, shape) in self.shapes.iter().enumerate() {
            println!("Shape #{}:", i + 1);
            shape.print();
            println!();
        }
    }
    
    pub fn equals(s1: &Scene, s2: &Scene) -> bool {
        if s1.shapes.len() != s2.shapes.len() {
            return false;
        }
        
        let mut matched = vec![false; s2.shapes.len()];
        
        for shape1 in &s1.shapes {
            let mut found = false;
            for (j, shape2) in s2.shapes.iter().enumerate() {
                if !matched[j] && ShapeManager::equals(shape1, shape2) {
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
    
    pub fn save(&self, filename: &str) -> Result<(), io::Error> {
        let path = Path::new(filename);
        let mut file = File::create(path)?;
        
        writeln!(file, "{}", self.name)?;
        writeln!(file, "{}", self.shapes.len())?;
        
        for shape in &self.shapes {
            writeln!(file, "{}", shape.shape_type() as usize)?;
        }
        
        println!("Scene saved to '{}'", filename);
        Ok(())
    }
    
    pub fn load(filename: &str) -> Option<Self> {
        let path = Path::new(filename);
        let file = File::open(path).ok()?;
        let reader = io::BufReader::new(file);
        let mut lines = reader.lines();
        
        let name = lines.next()?.ok()?;
        let name = name.trim().to_string();
        
        let shape_count: usize = lines.next()?.ok()?.parse().ok()?;
        
        let mut scene = Scene::new(&name);
        
        for _ in 0..shape_count {
            let type_line = lines.next()?.ok()?;
            let shape_type: usize = type_line.trim().parse().ok()?;
            
            if let Some(shape) = ShapeManager::get(ShapeType::from_usize(shape_type)) {
                let _ = scene.add_shape(shape);
            }
        }
        
        println!("Scene loaded from '{}'", filename);
        Some(scene)
    }
    
    pub fn list_shapes(&self) {
        println!("\nScene: {}", self.name);
        println!("Shapes ({}):", self.shapes.len());
        
        for (i, shape) in self.shapes.iter().enumerate() {
            println!("  {}. {} (ptr: {:p})", i + 1, shape.name(), *shape);
        }
    }
}
