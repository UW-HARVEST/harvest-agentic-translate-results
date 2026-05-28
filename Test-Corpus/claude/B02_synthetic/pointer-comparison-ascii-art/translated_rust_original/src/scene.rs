// scene.rs - Rust translation of scene.c
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

use crate::shape::{shape_equals, shape_get_by_index, shape_print, Shape};
use crate::{cprint, ceprint};

pub const MAX_SHAPES_IN_SCENE: usize = 50;
pub const MAX_SCENE_NAME: usize = 64;

pub struct Scene {
    pub name: String,
    pub shapes: Vec<*mut Shape>,
    pub shape_count: i32,
}

// Truncate a string to at most (max_bytes - 1) bytes, mimicking strncpy
// followed by NUL-termination.
fn truncate_to(name: &str, max_bytes: usize) -> String {
    let bytes = name.as_bytes();
    let take = bytes.len().min(max_bytes - 1);
    String::from_utf8_lossy(&bytes[..take]).into_owned()
}

pub fn scene_create(name: Option<&str>) -> *mut Scene {
    let scene_name = match name {
        Some(n) => truncate_to(n, MAX_SCENE_NAME),
        None => "Untitled Scene".to_string(),
    };
    let scene = Scene {
        name: scene_name,
        shapes: vec![std::ptr::null_mut(); MAX_SHAPES_IN_SCENE],
        shape_count: 0,
    };
    Box::into_raw(Box::new(scene))
}

pub fn scene_destroy(scene: *mut Scene) {
    if !scene.is_null() {
        unsafe {
            let _ = Box::from_raw(scene);
        }
    }
}

pub fn scene_add_shape(scene: *mut Scene, shape: *mut Shape) -> i32 {
    if scene.is_null() || shape.is_null() {
        return -1;
    }
    unsafe {
        let s = &mut *scene;
        if (s.shape_count as usize) >= MAX_SHAPES_IN_SCENE {
            ceprint!("Error: Scene is full\n");
            return -1;
        }
        let idx = s.shape_count as usize;
        s.shapes[idx] = shape;
        s.shape_count += 1;
        0
    }
}

pub fn scene_remove_shape(scene: *mut Scene, index: i32) -> i32 {
    if scene.is_null() {
        return -1;
    }
    unsafe {
        let s = &mut *scene;
        if index < 0 || index >= s.shape_count {
            return -1;
        }
        let mut i = index as usize;
        let count = s.shape_count as usize;
        while i < count - 1 {
            s.shapes[i] = s.shapes[i + 1];
            i += 1;
        }
        s.shape_count -= 1;
        0
    }
}

pub fn scene_print(scene: *const Scene) {
    if scene.is_null() {
        cprint!("(null scene)\n");
        return;
    }
    unsafe {
        let s = &*scene;
        cprint!("\n=== Scene: {} ===\n", s.name);
        cprint!("Contains {} shape(s)\n\n", s.shape_count);
        for i in 0..s.shape_count as usize {
            cprint!("Shape #{}:\n", i + 1);
            shape_print(s.shapes[i] as *const Shape);
            cprint!("\n");
        }
    }
}

pub fn scene_equals(s1: *const Scene, s2: *const Scene) -> i32 {
    if s1.is_null() || s2.is_null() {
        return 0;
    }
    unsafe {
        let a = &*s1;
        let b = &*s2;
        if a.shape_count != b.shape_count {
            return 0;
        }
        let mut matched = vec![0i32; MAX_SHAPES_IN_SCENE];
        for i in 0..a.shape_count as usize {
            let mut found = 0i32;
            for j in 0..b.shape_count as usize {
                if matched[j] == 0
                    && shape_equals(a.shapes[i] as *const Shape, b.shapes[j] as *const Shape) != 0
                {
                    matched[j] = 1;
                    found = 1;
                    break;
                }
            }
            if found == 0 {
                return 0;
            }
        }
        1
    }
}

pub fn scene_save(scene: *const Scene, filename: &str) -> i32 {
    if scene.is_null() {
        return -1;
    }
    unsafe {
        let s = &*scene;
        let mut file = match File::create(filename) {
            Ok(f) => f,
            Err(_) => {
                ceprint!("Error: Could not open file '{}' for writing\n", filename);
                return -1;
            }
        };
        let _ = writeln!(file, "{}", s.name);
        let _ = writeln!(file, "{}", s.shape_count);
        for i in 0..s.shape_count as usize {
            let sh = &*s.shapes[i];
            let _ = writeln!(file, "{}", sh.shape_type.as_i32());
        }
        drop(file);
        cprint!("Scene saved to '{}'\n", filename);
        0
    }
}

pub fn scene_load(filename: &str) -> *mut Scene {
    let file = match File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            ceprint!("Error: Could not open file '{}' for reading\n", filename);
            return std::ptr::null_mut();
        }
    };
    let mut reader = BufReader::new(file);

    // Read scene name (fgets-style)
    let mut name_line = String::new();
    match reader.read_line(&mut name_line) {
        Ok(0) => return std::ptr::null_mut(),
        Ok(_) => {}
        Err(_) => return std::ptr::null_mut(),
    }
    if name_line.len() > MAX_SCENE_NAME - 1 {
        name_line.truncate(MAX_SCENE_NAME - 1);
    }
    if name_line.ends_with('\n') {
        name_line.pop();
    }

    let scene = scene_create(Some(&name_line));
    if scene.is_null() {
        return std::ptr::null_mut();
    }

    let shape_count = match read_int(&mut reader) {
        Some(v) => v,
        None => {
            scene_destroy(scene);
            return std::ptr::null_mut();
        }
    };

    for _ in 0..shape_count {
        let t = match read_int(&mut reader) {
            Some(v) => v,
            None => {
                scene_destroy(scene);
                return std::ptr::null_mut();
            }
        };
        let shape_ptr = shape_get_by_index(t);
        if !shape_ptr.is_null() {
            scene_add_shape(scene, shape_ptr);
        }
    }

    cprint!("Scene loaded from '{}'\n", filename);
    scene
}

// Read an integer using fscanf-like semantics: skip whitespace, then parse digits.
fn read_int<R: BufRead>(reader: &mut R) -> Option<i32> {
    use std::io::Read;
    let mut buf = [0u8; 1];
    let mut sign: i32 = 1;
    let mut digits = String::new();
    let mut last_byte;

    // Skip whitespace
    loop {
        let n = reader.read(&mut buf).ok()?;
        if n == 0 {
            return None;
        }
        if !buf[0].is_ascii_whitespace() {
            last_byte = buf[0];
            break;
        }
    }

    if last_byte == b'-' {
        sign = -1;
    } else if last_byte == b'+' {
        // skip
    } else if last_byte.is_ascii_digit() {
        digits.push(last_byte as char);
    } else {
        return None;
    }

    loop {
        let n = match reader.read(&mut buf) {
            Ok(n) => n,
            Err(_) => break,
        };
        if n == 0 {
            break;
        }
        if buf[0].is_ascii_digit() {
            digits.push(buf[0] as char);
        } else {
            break;
        }
    }

    if digits.is_empty() {
        return None;
    }
    digits.parse::<i32>().ok().map(|v| v * sign)
}

pub fn scene_list_shapes(scene: *const Scene) {
    if scene.is_null() {
        cprint!("(null scene)\n");
        return;
    }
    unsafe {
        let s = &*scene;
        cprint!("\nScene: {}\n", s.name);
        cprint!("Shapes ({}):\n", s.shape_count);
        for i in 0..s.shape_count as usize {
            let sh = &*s.shapes[i];
            cprint!(
                "  {}. {} (ptr: {})\n",
                i + 1,
                sh.name,
                crate::util::format_ptr(s.shapes[i] as *const u8)
            );
        }
    }
}
