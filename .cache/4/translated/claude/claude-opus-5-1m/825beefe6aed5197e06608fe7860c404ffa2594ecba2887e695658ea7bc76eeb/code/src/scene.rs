//! Translation of `scene.c` / `scene.h`.

use std::fs::File;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use crate::cio::{self, CReader, Out};
use crate::p;
use crate::shape::{fmt_ptr, shape_equals, ShapeManager, ShapeRef};

pub const MAX_SHAPES_IN_SCENE: usize = 50;
pub const MAX_SCENE_NAME: usize = 64;

pub struct Scene {
    /// `char name[MAX_SCENE_NAME]`, stored as the raw bytes of the C string.
    pub name: Vec<u8>,
    /// `shape_t *shapes[MAX_SHAPES_IN_SCENE]` + `shape_count`.
    pub shapes: Vec<ShapeRef>,
}

impl Scene {
    pub fn shape_count(&self) -> i32 {
        self.shapes.len() as i32
    }
}

/// `scene_create`
pub fn scene_create(name: Option<&[u8]>) -> Option<Scene> {
    let name = match name {
        Some(n) => cio::truncate_to_buffer(n, MAX_SCENE_NAME),
        None => b"Untitled Scene".to_vec(),
    };
    Some(Scene {
        name,
        shapes: Vec::new(),
    })
}

/// `scene_add_shape`
pub fn scene_add_shape(scene: &mut Scene, shape: Option<ShapeRef>) -> i32 {
    let shape = match shape {
        Some(s) => s,
        None => return -1,
    };

    if scene.shapes.len() >= MAX_SHAPES_IN_SCENE {
        cio::err_str("Error: Scene is full\n");
        return -1;
    }

    scene.shapes.push(shape);
    0
}

/// `scene_remove_shape`
pub fn scene_remove_shape(scene: &mut Scene, index: i32) -> i32 {
    if index < 0 || index >= scene.shape_count() {
        return -1;
    }

    scene.shapes.remove(index as usize);
    0
}

/// `scene_print`
pub fn scene_print(out: &mut Out, mgr: &ShapeManager, scene: &Scene) {
    out.s("\n=== Scene: ");
    out.b(&scene.name);
    out.s(" ===\n");
    p!(out, "Contains {} shape(s)\n\n", scene.shape_count());

    for i in 0..scene.shape_count() {
        p!(out, "Shape #{}:\n", i + 1);
        mgr.print(out, Some(scene.shapes[i as usize]));
        out.s("\n");
    }
}

/// `scene_equals`
pub fn scene_equals(s1: &Scene, s2: &Scene) -> i32 {
    if s1.shape_count() != s2.shape_count() {
        return 0;
    }

    let mut matched = [0i32; MAX_SHAPES_IN_SCENE];

    for i in 0..s1.shape_count() as usize {
        let mut found = 0;
        for j in 0..s2.shape_count() as usize {
            if matched[j] == 0 && shape_equals(s1.shapes[i], s2.shapes[j]) != 0 {
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

/// `scene_save`
pub fn scene_save(out: &mut Out, mgr: &ShapeManager, scene: &Scene, filename: &[u8]) -> i32 {
    let path = Path::new(std::ffi::OsStr::from_bytes(filename));
    let file = match File::create(path) {
        Ok(f) => f,
        Err(_) => {
            cio::err_bytes(&[
                b"Error: Could not open file '",
                filename,
                b"' for writing\n",
            ]);
            return -1;
        }
    };
    let mut file = std::io::BufWriter::new(file);

    // Write scene name
    let _ = file.write_all(&scene.name);
    let _ = file.write_all(b"\n");

    // Write shape count
    let _ = file.write_all(format!("{}\n", scene.shape_count()).as_bytes());

    // Write shape types (not the shapes themselves, just their types)
    for i in 0..scene.shape_count() as usize {
        let kind = mgr.shape(scene.shapes[i]).kind;
        let _ = file.write_all(format!("{}\n", kind).as_bytes());
    }

    let _ = file.flush();
    drop(file);

    out.s("Scene saved to '");
    out.b(filename);
    out.s("'\n");
    0
}

/// `scene_load`
pub fn scene_load(out: &mut Out, mgr: &ShapeManager, filename: &[u8]) -> Option<Scene> {
    let path = Path::new(std::ffi::OsStr::from_bytes(filename));
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => {
            cio::err_bytes(&[
                b"Error: Could not open file '",
                filename,
                b"' for reading\n",
            ]);
            return None;
        }
    };
    let mut file = CReader::new(file);

    let name = match file.fgets(MAX_SCENE_NAME) {
        Some(line) => line,
        None => return None,
    };

    // Remove newline
    let name = cio::strip_newline(&name);

    let mut scene = match scene_create(Some(&name)) {
        Some(s) => s,
        None => return None,
    };

    let shape_count = match file.scan_int() {
        Some(v) => {
            file.skip_format_space();
            v
        }
        None => return None,
    };

    for _ in 0..shape_count {
        let shape_type = match file.scan_int() {
            Some(v) => {
                file.skip_format_space();
                v
            }
            None => return None,
        };

        let shape = mgr.get(shape_type);
        if shape.is_some() {
            scene_add_shape(&mut scene, shape);
        }
    }

    out.s("Scene loaded from '");
    out.b(filename);
    out.s("'\n");
    Some(scene)
}

/// `scene_list_shapes`
pub fn scene_list_shapes(out: &mut Out, mgr: &ShapeManager, scene: &Scene) {
    out.s("\nScene: ");
    out.b(&scene.name);
    out.s("\n");
    p!(out, "Shapes ({}):\n", scene.shape_count());

    for i in 0..scene.shape_count() as usize {
        let r = scene.shapes[i];
        p!(
            out,
            "  {}. {} (ptr: {})\n",
            i + 1,
            mgr.name(r),
            fmt_ptr(mgr.addr(r))
        );
    }
}
