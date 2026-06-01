// Translation of scene.c / scene.h to Rust.

use std::fs::File;
use std::io::{BufRead, BufReader, Write};

use crate::shape::{shape_equals, shape_get, shape_print, Shape, ShapeType};

pub const MAX_SHAPES_IN_SCENE: usize = 50;
pub const MAX_SCENE_NAME: usize = 64;

pub struct Scene {
    pub name: String,
    pub shapes: Vec<&'static Shape>,
    pub shape_count: i32,
}

pub fn scene_create(name: Option<&str>) -> Scene {
    let name_str = match name {
        Some(n) => {
            // Mirror strncpy with MAX_SCENE_NAME-1 truncation
            let mut s = n.to_string();
            if s.len() > MAX_SCENE_NAME - 1 {
                // Truncate to MAX_SCENE_NAME-1 *bytes*, on a char boundary if possible.
                // For ASCII this matches C exactly.
                let mut cut = MAX_SCENE_NAME - 1;
                while !s.is_char_boundary(cut) && cut > 0 {
                    cut -= 1;
                }
                s.truncate(cut);
            }
            s
        }
        None => "Untitled Scene".to_string(),
    };

    Scene {
        name: name_str,
        shapes: Vec::with_capacity(MAX_SHAPES_IN_SCENE),
        shape_count: 0,
    }
}

pub fn scene_destroy(_scene: Scene) {
    // No-op: dropping the Scene frees memory; shapes are singletons.
}

pub fn scene_add_shape(scene: &mut Scene, shape: Option<&'static Shape>) -> Result<(), ()> {
    let shape = match shape {
        Some(s) => s,
        None => return Err(()),
    };

    if scene.shape_count as usize >= MAX_SHAPES_IN_SCENE {
        eprintln!("Error: Scene is full");
        return Err(());
    }

    scene.shapes.push(shape);
    scene.shape_count += 1;
    Ok(())
}

pub fn scene_remove_shape(scene: &mut Scene, index: i32) -> Result<(), ()> {
    if index < 0 || index >= scene.shape_count {
        return Err(());
    }

    scene.shapes.remove(index as usize);
    scene.shape_count -= 1;
    Ok(())
}

pub fn scene_print(scene: Option<&Scene>) {
    let scene = match scene {
        Some(s) => s,
        None => {
            crate::print("(null scene)\n");
            return;
        }
    };

    crate::print(&format!("\n=== Scene: {} ===\n", scene.name));
    crate::print(&format!("Contains {} shape(s)\n\n", scene.shape_count));

    for i in 0..scene.shape_count as usize {
        crate::print(&format!("Shape #{}:\n", i + 1));
        shape_print(Some(scene.shapes[i]));
        crate::print("\n");
    }
}

pub fn scene_equals(s1: Option<&Scene>, s2: Option<&Scene>) -> i32 {
    let s1 = match s1 { Some(x) => x, None => return 0 };
    let s2 = match s2 { Some(x) => x, None => return 0 };

    if s1.shape_count != s2.shape_count {
        return 0;
    }

    let mut matched = vec![false; MAX_SHAPES_IN_SCENE];

    for i in 0..s1.shape_count as usize {
        let mut found = false;
        for j in 0..s2.shape_count as usize {
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

pub fn scene_save(scene: &Scene, filename: &str) -> i32 {
    let file = match File::create(filename) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Error: Could not open file '{}' for writing", filename);
            return -1;
        }
    };
    let mut w = std::io::BufWriter::new(file);
    let _ = writeln!(w, "{}", scene.name);
    let _ = writeln!(w, "{}", scene.shape_count);
    for i in 0..scene.shape_count as usize {
        let _ = writeln!(w, "{}", scene.shapes[i].type_int());
    }
    let _ = w.flush();

    crate::print(&format!("Scene saved to '{}'\n", filename));
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

    let mut name_line = String::new();
    if reader.read_line(&mut name_line).ok()? == 0 {
        return None;
    }
    // fgets reads up to MAX_SCENE_NAME-1 chars + '\0'.  Truncate accordingly.
    if name_line.len() > MAX_SCENE_NAME - 1 {
        // Cut on char boundary
        let mut cut = MAX_SCENE_NAME - 1;
        while !name_line.is_char_boundary(cut) && cut > 0 {
            cut -= 1;
        }
        name_line.truncate(cut);
    }
    // Strip trailing newline (strcspn(name, "\n") = 0)
    let stripped: String = name_line.chars().take_while(|&c| c != '\n').collect();

    let mut scene = scene_create(Some(&stripped));

    // fscanf("%d\n", ...): read int, skip optional whitespace.
    let shape_count = match read_fscanf_int(&mut reader) {
        Some(v) => v,
        None => return None,
    };

    for _ in 0..shape_count {
        let t = match read_fscanf_int(&mut reader) {
            Some(v) => v,
            None => return None,
        };
        if t >= 0 && t < crate::shape::SHAPE_COUNT as i32 {
            let shape = shape_get(ShapeType::from_int(t));
            let _ = scene_add_shape(&mut scene, shape);
        }
    }

    crate::print(&format!("Scene loaded from '{}'\n", filename));
    Some(scene)
}

// Read an integer skipping leading whitespace (mirrors scanf("%d")).
fn read_fscanf_int(reader: &mut BufReader<File>) -> Option<i32> {
    use std::io::Read;
    let mut buf = [0u8; 1];
    // Skip whitespace
    loop {
        let n = reader.read(&mut buf).ok()?;
        if n == 0 { return None; }
        if !(buf[0] as char).is_ascii_whitespace() {
            break;
        }
    }
    let mut s = String::new();
    let sign_or_digit = buf[0];
    if sign_or_digit == b'-' || sign_or_digit == b'+' || sign_or_digit.is_ascii_digit() {
        s.push(sign_or_digit as char);
    } else {
        return None;
    }
    loop {
        let n = reader.read(&mut buf).ok()?;
        if n == 0 { break; }
        if buf[0].is_ascii_digit() {
            s.push(buf[0] as char);
        } else {
            break;
        }
    }
    s.parse::<i32>().ok()
}

pub fn scene_list_shapes(scene: Option<&Scene>) {
    let scene = match scene {
        Some(s) => s,
        None => {
            crate::print("(null scene)\n");
            return;
        }
    };

    crate::print(&format!("\nScene: {}\n", scene.name));
    crate::print(&format!("Shapes ({}):\n", scene.shape_count));

    for i in 0..scene.shape_count as usize {
        let s = scene.shapes[i];
        crate::print(&format!(
            "  {}. {} (ptr: {})\n",
            i + 1,
            s.name(),
            crate::ptr_format(s as *const _ as *const ())
        ));
    }
}
