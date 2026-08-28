//! Translation of scene.c / scene.h

use crate::cio::{self, ByteSrc, COut};
use crate::shape::{shape_equals, shape_get, shape_print, shape_ptr, Shape};

pub const MAX_SHAPES_IN_SCENE: usize = 50;
pub const MAX_SCENE_NAME: usize = 64;

pub struct Scene {
    pub name: Vec<u8>,
    pub shapes: Vec<&'static Shape>,
}

impl Scene {
    pub fn shape_count(&self) -> i32 {
        self.shapes.len() as i32
    }
}

/// Create a new empty scene.  malloc never fails here, so `scene_create`
/// always succeeds (the C caller's NULL branch is therefore dead).
pub fn scene_create(name: Option<&[u8]>) -> Scene {
    let name = match name {
        Some(n) => {
            // strncpy(scene->name, name, MAX_SCENE_NAME - 1) + explicit NUL
            let n = cio::c_str(n);
            let take = if n.len() > MAX_SCENE_NAME - 1 {
                MAX_SCENE_NAME - 1
            } else {
                n.len()
            };
            n[..take].to_vec()
        }
        None => b"Untitled Scene".to_vec(),
    };

    Scene {
        name,
        shapes: Vec::new(),
    }
}

/// Add a shape to the scene
pub fn scene_add_shape(scene: &mut Scene, shape: Option<&'static Shape>) -> i32 {
    let shape = match shape {
        None => return -1,
        Some(s) => s,
    };

    if scene.shapes.len() >= MAX_SHAPES_IN_SCENE {
        cio::err(b"Error: Scene is full\n");
        return -1;
    }

    scene.shapes.push(shape);
    0
}

/// Remove a shape at index
pub fn scene_remove_shape(scene: &mut Scene, index: i32) -> i32 {
    if index < 0 || index >= scene.shape_count() {
        return -1;
    }

    scene.shapes.remove(index as usize);
    0
}

/// Print the scene
pub fn scene_print(out: &mut COut, scene: &Scene) {
    out.puts("\n=== Scene: ");
    out.put(&scene.name);
    out.puts(" ===\n");
    out.puts(&format!("Contains {} shape(s)\n\n", scene.shape_count()));

    for i in 0..scene.shapes.len() {
        out.puts(&format!("Shape #{}:\n", i + 1));
        shape_print(out, Some(scene.shapes[i]));
        out.puts("\n");
    }
}

/// Compare two scenes for equality (1:1 correspondence)
pub fn scene_equals(s1: &Scene, s2: &Scene) -> i32 {
    // Scenes are equal if there's a 1:1 correspondence
    if s1.shape_count() != s2.shape_count() {
        return 0;
    }

    // For each shape in s1, find a matching shape in s2
    let mut matched = [0i32; MAX_SHAPES_IN_SCENE];

    for i in 0..s1.shapes.len() {
        let mut found = 0;
        for j in 0..s2.shapes.len() {
            if matched[j] == 0 && shape_equals(Some(s1.shapes[i]), Some(s2.shapes[j])) != 0 {
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

/// Save scene to file
pub fn scene_save(out: &mut COut, scene: &Scene, filename: &[u8]) -> i32 {
    let mut data: Vec<u8> = Vec::new();

    // Write scene name
    data.extend_from_slice(&scene.name);
    data.push(b'\n');

    // Write shape count
    data.extend_from_slice(format!("{}\n", scene.shape_count()).as_bytes());

    // Write shape types (not the shapes themselves, just their types)
    for shape in &scene.shapes {
        data.extend_from_slice(format!("{}\n", shape.stype).as_bytes());
    }

    match write_file(filename, &data) {
        Ok(()) => {}
        Err(()) => {
            let mut msg: Vec<u8> = Vec::new();
            msg.extend_from_slice(b"Error: Could not open file '");
            msg.extend_from_slice(filename);
            msg.extend_from_slice(b"' for writing\n");
            cio::err(&msg);
            return -1;
        }
    }

    out.puts("Scene saved to '");
    out.put(filename);
    out.puts("'\n");
    0
}

/// Load scene from file
pub fn scene_load(out: &mut COut, filename: &[u8]) -> Option<Scene> {
    let contents = match read_file(filename) {
        Ok(c) => c,
        Err(()) => {
            let mut msg: Vec<u8> = Vec::new();
            msg.extend_from_slice(b"Error: Could not open file '");
            msg.extend_from_slice(filename);
            msg.extend_from_slice(b"' for reading\n");
            cio::err(&msg);
            return None;
        }
    };

    let mut file = ByteSrc::new(&contents);

    let name = file.fgets(MAX_SCENE_NAME)?;

    // Remove newline
    let name = cio::strip_newline(&name);

    let mut scene = scene_create(Some(&name));

    let shape_count = file.fscanf_int_nl()?;

    for _ in 0..shape_count {
        let stype = file.fscanf_int_nl()?;

        let shape = shape_get(stype);
        if shape.is_some() {
            scene_add_shape(&mut scene, shape);
        }
    }

    out.puts("Scene loaded from '");
    out.put(filename);
    out.puts("'\n");
    Some(scene)
}

/// List all shapes in scene
pub fn scene_list_shapes(out: &mut COut, scene: &Scene) {
    out.puts("\nScene: ");
    out.put(&scene.name);
    out.puts("\n");
    out.puts(&format!("Shapes ({}):\n", scene.shape_count()));

    for i in 0..scene.shapes.len() {
        out.puts(&format!("  {}. ", i + 1));
        out.put(&scene.shapes[i].name);
        out.puts(&format!(" (ptr: {})\n", shape_ptr(scene.shapes[i])));
    }
}

#[cfg(unix)]
fn os_name(filename: &[u8]) -> std::ffi::OsString {
    use std::os::unix::ffi::OsStringExt;
    std::ffi::OsString::from_vec(filename.to_vec())
}

#[cfg(not(unix))]
fn os_name(filename: &[u8]) -> std::ffi::OsString {
    std::ffi::OsString::from(String::from_utf8_lossy(filename).into_owned())
}

/// fopen(filename, "w") + fprintf + fclose
fn write_file(filename: &[u8], data: &[u8]) -> Result<(), ()> {
    use std::io::Write;
    if filename.is_empty() {
        return Err(());
    }
    let mut f = std::fs::File::create(os_name(filename)).map_err(|_| ())?;
    let _ = f.write_all(data);
    Ok(())
}

/// fopen(filename, "r") + read everything + fclose
///
/// Only a failure to *open* the file is an error: opening a directory succeeds
/// in C and the subsequent read fails, which the C code sees as an immediate
/// end of file.
fn read_file(filename: &[u8]) -> Result<Vec<u8>, ()> {
    use std::io::Read;
    if filename.is_empty() {
        return Err(());
    }
    let mut f = std::fs::File::open(os_name(filename)).map_err(|_| ())?;
    let mut buf: Vec<u8> = Vec::new();
    let _ = f.read_to_end(&mut buf);
    Ok(buf)
}
