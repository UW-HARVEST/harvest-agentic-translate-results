//! Translation of scene.c / scene.h

use std::io::{Read, Write};

use crate::cio::{ceprintf, cprintf, trim_at_newline, Arg, FileIn, Out};
use crate::shape::{shape_equals, shape_get, shape_name, shape_print, shape_ptr, shape_type};

pub const MAX_SHAPES_IN_SCENE: usize = 50;
pub const MAX_SCENE_NAME: usize = 64;

pub struct Scene {
    /// `char name[MAX_SCENE_NAME]`, holding the bytes up to the NUL terminator.
    pub name: Vec<u8>,
    /// `shape_t *shapes[MAX_SHAPES_IN_SCENE]`, as singleton indices.
    pub shapes: Vec<usize>,
}

impl Scene {
    pub fn shape_count(&self) -> i32 {
        self.shapes.len() as i32
    }
}

/// `scene_create()`
pub fn scene_create(name: Option<&[u8]>) -> Scene {
    let stored = match name {
        // strncpy(scene->name, name, 63) + explicit NUL terminator
        Some(n) => {
            let take = n.len().min(MAX_SCENE_NAME - 1);
            n[..take].to_vec()
        }
        None => b"Untitled Scene".to_vec(),
    };
    Scene {
        name: stored,
        shapes: Vec::new(),
    }
}

/// `scene_add_shape()`
pub fn scene_add_shape(scene: &mut Scene, shape: Option<usize>) -> i32 {
    let shape = match shape {
        None => return -1,
        Some(s) => s,
    };

    if scene.shapes.len() >= MAX_SHAPES_IN_SCENE {
        ceprintf(b"Error: Scene is full\n", &[]);
        return -1;
    }

    scene.shapes.push(shape);
    0
}

/// `scene_remove_shape()`
pub fn scene_remove_shape(scene: &mut Scene, index: i32) -> i32 {
    if index < 0 || index >= scene.shape_count() {
        return -1;
    }
    scene.shapes.remove(index as usize);
    0
}

/// `scene_print()`
pub fn scene_print(out: &mut Out, scene: &Scene) {
    cprintf(out, b"\n=== Scene: %s ===\n", &[Arg::S(&scene.name)]);
    cprintf(
        out,
        b"Contains %d shape(s)\n\n",
        &[Arg::D(scene.shape_count())],
    );

    let mut i = 0;
    while i < scene.shape_count() {
        cprintf(out, b"Shape #%d:\n", &[Arg::D(i + 1)]);
        shape_print(out, Some(scene.shapes[i as usize]));
        cprintf(out, b"\n", &[]);
        i += 1;
    }
}

/// `scene_equals()`: 1:1 correspondence based on pointer identity.
pub fn scene_equals(s1: &Scene, s2: &Scene) -> bool {
    if s1.shape_count() != s2.shape_count() {
        return false;
    }

    let mut matched = [false; MAX_SHAPES_IN_SCENE];

    for i in 0..s1.shapes.len() {
        let mut found = false;
        for j in 0..s2.shapes.len() {
            if !matched[j] && shape_equals(Some(s1.shapes[i]), Some(s2.shapes[j])) {
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

/// `scene_save()`
pub fn scene_save(out: &mut Out, scene: &Scene, filename: &[u8]) -> i32 {
    let path = os_path(filename);
    let file = std::fs::File::create(&path);
    let mut file = match file {
        Err(_) => {
            ceprintf(
                b"Error: Could not open file '%s' for writing\n",
                &[Arg::S(filename)],
            );
            return -1;
        }
        Ok(f) => f,
    };

    let mut contents: Vec<u8> = Vec::new();
    contents.extend_from_slice(&scene.name);
    contents.push(b'\n');
    contents.extend_from_slice(format!("{}\n", scene.shape_count()).as_bytes());
    for &s in &scene.shapes {
        contents.extend_from_slice(format!("{}\n", shape_type(s)).as_bytes());
    }
    let _ = file.write_all(&contents);
    let _ = file.flush();
    drop(file);

    cprintf(out, b"Scene saved to '%s'\n", &[Arg::S(filename)]);
    0
}

/// `scene_load()`
pub fn scene_load(out: &mut Out, filename: &[u8]) -> Option<Scene> {
    let path = os_path(filename);
    let mut handle = match std::fs::File::open(&path) {
        Err(_) => {
            ceprintf(
                b"Error: Could not open file '%s' for reading\n",
                &[Arg::S(filename)],
            );
            return None;
        }
        Ok(f) => f,
    };

    let mut data = Vec::new();
    if handle.read_to_end(&mut data).is_err() {
        data.clear();
    }
    let mut file = FileIn::new(data);

    // fgets(name, MAX_SCENE_NAME, file)
    let raw = file.fgets(MAX_SCENE_NAME)?;
    let name = trim_at_newline(&raw).to_vec();

    let mut scene = scene_create(Some(&name));

    let shape_count = file.scan_int()?;

    let mut i = 0;
    while i < shape_count {
        let type_ = match file.scan_int() {
            None => return None,
            Some(t) => t,
        };

        let shape = shape_get(type_);
        if shape.is_some() {
            scene_add_shape(&mut scene, shape);
        }
        i += 1;
    }

    cprintf(out, b"Scene loaded from '%s'\n", &[Arg::S(filename)]);
    Some(scene)
}

/// `scene_list_shapes()`
pub fn scene_list_shapes(out: &mut Out, scene: &Scene) {
    cprintf(out, b"\nScene: %s\n", &[Arg::S(&scene.name)]);
    cprintf(out, b"Shapes (%d):\n", &[Arg::D(scene.shape_count())]);

    let mut i = 0;
    while i < scene.shape_count() {
        let idx = scene.shapes[i as usize];
        cprintf(
            out,
            b"  %d. %s (ptr: %p)\n",
            &[
                Arg::D(i + 1),
                Arg::S(shape_name(idx)),
                Arg::P(shape_ptr(idx)),
            ],
        );
        i += 1;
    }
}

/// Turn raw filename bytes into a path without going through UTF-8 validation.
fn os_path(bytes: &[u8]) -> std::path::PathBuf {
    use std::os::unix::ffi::OsStrExt;
    std::path::PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
}
