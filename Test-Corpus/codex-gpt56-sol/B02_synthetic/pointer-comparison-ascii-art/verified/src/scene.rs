use crate::input::fscanf_i32;
use crate::shape::{self, ShapeManager};
use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::unix::ffi::OsStrExt;

const MAX_SHAPES_IN_SCENE: usize = 50;
const MAX_SCENE_NAME: usize = 64;

pub struct Scene {
    pub name: Vec<u8>,
    pub shapes: Vec<i32>,
}

impl Scene {
    pub fn new(name: &[u8]) -> Self {
        Self {
            name: name[..name.len().min(MAX_SCENE_NAME - 1)].to_vec(),
            shapes: Vec::new(),
        }
    }

    pub fn add_shape<W: Write>(&mut self, type_id: i32, err: &mut W) -> i32 {
        if self.shapes.len() >= MAX_SHAPES_IN_SCENE {
            let _ = err.write_all(b"Error: Scene is full\n");
            return -1;
        }
        self.shapes.push(type_id);
        0
    }

    pub fn remove_shape(&mut self, index: i32) -> i32 {
        let Ok(index) = usize::try_from(index) else {
            return -1;
        };
        if index >= self.shapes.len() {
            return -1;
        }
        self.shapes.remove(index);
        0
    }
}

pub fn print<W: Write>(out: &mut W, scene: &Scene, manager: &ShapeManager) -> io::Result<()> {
    out.write_all(b"\n=== Scene: ")?;
    out.write_all(&scene.name)?;
    out.write_all(b" ===\nContains ")?;
    write!(out, "{}", scene.shapes.len())?;
    out.write_all(b" shape(s)\n\n")?;

    for (index, &type_id) in scene.shapes.iter().enumerate() {
        writeln!(out, "Shape #{}:", index + 1)?;
        shape::print(out, manager.get(type_id))?;
        out.write_all(b"\n")?;
    }
    Ok(())
}

pub fn list_shapes<W: Write>(out: &mut W, scene: &Scene, manager: &ShapeManager) -> io::Result<()> {
    out.write_all(b"\nScene: ")?;
    out.write_all(&scene.name)?;
    out.write_all(b"\nShapes (")?;
    write!(out, "{}", scene.shapes.len())?;
    out.write_all(b"):\n")?;

    for (index, &type_id) in scene.shapes.iter().enumerate() {
        write!(out, "  {}. ", index + 1)?;
        out.write_all(manager.get(type_id).unwrap().name)?;
        write!(out, " (ptr: {:p})\n", manager.ptr(type_id))?;
    }
    Ok(())
}

pub fn equals(first: &Scene, second: &Scene) -> bool {
    if first.shapes.len() != second.shapes.len() {
        return false;
    }

    let mut matched = [false; MAX_SHAPES_IN_SCENE];
    for first_type in &first.shapes {
        let mut found = false;
        for (index, second_type) in second.shapes.iter().enumerate() {
            if !matched[index] && first_type == second_type {
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

pub fn save<O: Write, E: Write>(
    scene: &Scene,
    filename: &[u8],
    manager: &ShapeManager,
    out: &mut O,
    err: &mut E,
) -> i32 {
    let path = OsStr::from_bytes(filename);
    let mut file = match File::create(path) {
        Ok(file) => file,
        Err(_) => {
            let _ = err.write_all(b"Error: Could not open file '");
            let _ = err.write_all(filename);
            let _ = err.write_all(b"' for writing\n");
            return -1;
        }
    };

    let _ = file.write_all(&scene.name);
    let _ = file.write_all(b"\n");
    let _ = writeln!(file, "{}", scene.shapes.len());
    for &type_id in &scene.shapes {
        let _ = writeln!(file, "{}", manager.get(type_id).unwrap().type_id);
    }

    let _ = out.write_all(b"Scene saved to '");
    let _ = out.write_all(filename);
    let _ = out.write_all(b"'\n");
    0
}

pub fn load<O: Write, E: Write>(
    filename: &[u8],
    manager: &ShapeManager,
    out: &mut O,
    err: &mut E,
) -> Option<Scene> {
    let path = OsStr::from_bytes(filename);
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(_) => {
            let _ = err.write_all(b"Error: Could not open file '");
            let _ = err.write_all(filename);
            let _ = err.write_all(b"' for reading\n");
            return None;
        }
    };

    let mut data = Vec::new();
    if file.read_to_end(&mut data).is_err() {
        return None;
    }

    let mut position = 0;
    let name_line = file_fgets(&data, &mut position, MAX_SCENE_NAME)?;
    let name_end = name_line
        .iter()
        .position(|&byte| byte == 0 || byte == b'\n')
        .unwrap_or(name_line.len());
    let mut scene = Scene::new(&name_line[..name_end]);

    let shape_count = fscanf_i32(&data, &mut position)?;
    for _ in 0..shape_count {
        let type_id = fscanf_i32(&data, &mut position)?;
        if manager.get(type_id).is_some() {
            scene.add_shape(type_id, err);
        }
    }

    let _ = out.write_all(b"Scene loaded from '");
    let _ = out.write_all(filename);
    let _ = out.write_all(b"'\n");
    Some(scene)
}

fn file_fgets(data: &[u8], position: &mut usize, size: usize) -> Option<Vec<u8>> {
    if *position >= data.len() || size <= 1 {
        return None;
    }

    let start = *position;
    let limit = (start + size - 1).min(data.len());
    while *position < limit {
        let byte = data[*position];
        *position += 1;
        if byte == b'\n' {
            break;
        }
    }
    Some(data[start..*position].to_vec())
}
