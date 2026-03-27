use crate::shape::*;
use std::io::{self, BufRead, Write};
use std::fs::File;

pub const MAX_SHAPES_IN_SCENE: usize = 50;
pub const MAX_SCENE_NAME: usize = 64;

pub struct Scene {
    pub name: String,
    pub shapes: [*mut Shape; MAX_SHAPES_IN_SCENE],
    pub shape_count: i32,
}

pub fn scene_create(name: &str) -> *mut Scene {
    let scene = Box::new(Scene {
        name: if name.is_empty() {
            "Untitled Scene".into()
        } else {
            let mut n = name.to_string();
            n.truncate(MAX_SCENE_NAME - 1);
            n
        },
        shapes: [std::ptr::null_mut(); MAX_SHAPES_IN_SCENE],
        shape_count: 0,
    });
    Box::into_raw(scene)
}

pub fn scene_destroy(scene: *mut Scene) {
    if !scene.is_null() {
        unsafe { drop(Box::from_raw(scene)); }
    }
}

pub fn scene_add_shape(scene: *mut Scene, shape: *mut Shape) -> i32 {
    if scene.is_null() || shape.is_null() {
        return -1;
    }
    let s = unsafe { &mut *scene };
    if s.shape_count >= MAX_SHAPES_IN_SCENE as i32 {
        eprint!("Error: Scene is full\n");
        return -1;
    }
    s.shapes[s.shape_count as usize] = shape;
    s.shape_count += 1;
    0
}

pub fn scene_remove_shape(scene: *mut Scene, index: i32) -> i32 {
    if scene.is_null() {
        return -1;
    }
    let s = unsafe { &mut *scene };
    if index < 0 || index >= s.shape_count {
        return -1;
    }
    for i in index as usize..(s.shape_count - 1) as usize {
        s.shapes[i] = s.shapes[i + 1];
    }
    s.shape_count -= 1;
    0
}

pub fn scene_print(scene: *const Scene) {
    if scene.is_null() {
        print!("(null scene)\n");
        return;
    }
    let s = unsafe { &*scene };
    print!("\n=== Scene: {} ===\n", s.name);
    print!("Contains {} shape(s)\n\n", s.shape_count);
    for i in 0..s.shape_count as usize {
        print!("Shape #{}:\n", i + 1);
        shape_print(s.shapes[i]);
        print!("\n");
    }
}

pub fn scene_equals(s1: *const Scene, s2: *const Scene) -> bool {
    if s1.is_null() || s2.is_null() {
        return false;
    }
    let sc1 = unsafe { &*s1 };
    let sc2 = unsafe { &*s2 };
    if sc1.shape_count != sc2.shape_count {
        return false;
    }
    let mut matched = [false; MAX_SHAPES_IN_SCENE];
    for i in 0..sc1.shape_count as usize {
        let mut found = false;
        for j in 0..sc2.shape_count as usize {
            if !matched[j] && shape_equals(sc1.shapes[i], sc2.shapes[j]) {
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

pub fn scene_save(scene: *const Scene, filename: &str) -> i32 {
    if scene.is_null() || filename.is_empty() {
        return -1;
    }
    let s = unsafe { &*scene };
    let file = File::create(filename);
    if file.is_err() {
        eprint!("Error: Could not open file '{}' for writing\n", filename);
        return -1;
    }
    let mut file = file.unwrap();
    let _ = write!(file, "{}\n", s.name);
    let _ = write!(file, "{}\n", s.shape_count);
    for i in 0..s.shape_count as usize {
        let shape = unsafe { &*s.shapes[i] };
        let _ = write!(file, "{}\n", shape.shape_type);
    }
    print!("Scene saved to '{}'\n", filename);
    0
}

pub fn scene_load(filename: &str) -> *mut Scene {
    if filename.is_empty() {
        return std::ptr::null_mut();
    }
    let file = File::open(filename);
    if file.is_err() {
        eprint!("Error: Could not open file '{}' for reading\n", filename);
        return std::ptr::null_mut();
    }
    let mut reader = io::BufReader::new(file.unwrap());
    let mut name = String::new();
    if reader.read_line(&mut name).is_err() || name.is_empty() {
        return std::ptr::null_mut();
    }
    let name = name.trim_end_matches('\n').to_string();

    let mut count_line = String::new();
    if reader.read_line(&mut count_line).is_err() {
        return std::ptr::null_mut();
    }
    let shape_count: i32 = match count_line.trim().parse() {
        Ok(v) => v,
        Err(_) => return std::ptr::null_mut(),
    };

    let scene = scene_create(&name);
    if scene.is_null() {
        return std::ptr::null_mut();
    }

    for _ in 0..shape_count {
        let mut type_line = String::new();
        if reader.read_line(&mut type_line).is_err() {
            scene_destroy(scene);
            return std::ptr::null_mut();
        }
        let shape_type: usize = match type_line.trim().parse() {
            Ok(v) => v,
            Err(_) => {
                scene_destroy(scene);
                return std::ptr::null_mut();
            }
        };
        let shape = shape_get(shape_type);
        if !shape.is_null() {
            scene_add_shape(scene, shape);
        }
    }

    print!("Scene loaded from '{}'\n", filename);
    scene
}

pub fn scene_list_shapes(scene: *const Scene) {
    if scene.is_null() {
        print!("(null scene)\n");
        return;
    }
    let s = unsafe { &*scene };
    print!("\nScene: {}\n", s.name);
    print!("Shapes ({}):\n", s.shape_count);
    for i in 0..s.shape_count as usize {
        let shape = unsafe { &*s.shapes[i] };
        print!("  {}. {} (ptr: {:p})\n", i + 1, shape.name, s.shapes[i]);
    }
}

// --- C ABI exports with exact C symbol names ---
use std::os::raw::{c_char, c_int};

#[export_name = "scene_create"]
pub unsafe extern "C" fn _export_scene_create(name: *const c_char) -> *mut Scene {
    if name.is_null() {
        scene_create("")
    } else {
        let s = std::ffi::CStr::from_ptr(name).to_str().unwrap_or("");
        scene_create(s)
    }
}

#[export_name = "scene_destroy"]
pub unsafe extern "C" fn _export_scene_destroy(scene: *mut Scene) {
    scene_destroy(scene);
}

#[export_name = "scene_add_shape"]
pub unsafe extern "C" fn _export_scene_add_shape(scene: *mut Scene, shape: *mut Shape) -> c_int {
    scene_add_shape(scene, shape)
}

#[export_name = "scene_remove_shape"]
pub unsafe extern "C" fn _export_scene_remove_shape(scene: *mut Scene, index: c_int) -> c_int {
    scene_remove_shape(scene, index)
}

#[export_name = "scene_print"]
pub unsafe extern "C" fn _export_scene_print(scene: *const Scene) {
    scene_print(scene);
}

#[export_name = "scene_equals"]
pub unsafe extern "C" fn _export_scene_equals(s1: *const Scene, s2: *const Scene) -> c_int {
    if scene_equals(s1, s2) { 1 } else { 0 }
}

#[export_name = "scene_save"]
pub unsafe extern "C" fn _export_scene_save(scene: *const Scene, filename: *const c_char) -> c_int {
    if filename.is_null() {
        return -1;
    }
    let f = std::ffi::CStr::from_ptr(filename).to_str().unwrap_or("");
    scene_save(scene, f)
}

#[export_name = "scene_load"]
pub unsafe extern "C" fn _export_scene_load(filename: *const c_char) -> *mut Scene {
    if filename.is_null() {
        return std::ptr::null_mut();
    }
    let f = std::ffi::CStr::from_ptr(filename).to_str().unwrap_or("");
    scene_load(f)
}

#[export_name = "scene_list_shapes"]
pub unsafe extern "C" fn _export_scene_list_shapes(scene: *const Scene) {
    scene_list_shapes(scene);
}
