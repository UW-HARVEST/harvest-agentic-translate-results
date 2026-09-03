// Translation of scene.c / scene.h

use crate::cio::{self, In, Out, Scan};
use crate::shape::{shape_print, ShapeManager};

pub const MAX_SHAPES_IN_SCENE: usize = 50;
pub const MAX_SCENE_NAME: usize = 64;

pub struct Scene {
    pub name: [u8; MAX_SCENE_NAME],
    /// The C code stores `shape_t *`; every stored pointer is one of the
    /// singletons, so the shape type index identifies it uniquely and pointer
    /// comparison is index comparison.
    pub shapes: [i32; MAX_SHAPES_IN_SCENE],
    pub shape_count: i32,
}

impl Scene {
    /// scene_create(name)
    pub fn create(name: Option<&[u8]>) -> Box<Scene> {
        let mut scene = Box::new(Scene {
            name: [0u8; MAX_SCENE_NAME],
            shapes: [0i32; MAX_SHAPES_IN_SCENE],
            shape_count: 0,
        });

        match name {
            Some(n) => {
                // strncpy(scene->name, name, MAX_SCENE_NAME - 1); then NUL at
                // the last byte.
                let src = cio::c_str_bytes(n);
                let take = src.len().min(MAX_SCENE_NAME - 1);
                scene.name[..take].copy_from_slice(&src[..take]);
                scene.name[MAX_SCENE_NAME - 1] = 0;
            }
            None => {
                let d = b"Untitled Scene";
                scene.name[..d.len()].copy_from_slice(d);
                scene.name[d.len()] = 0;
            }
        }

        scene.shape_count = 0;
        scene
    }

    pub fn name_str(&self) -> &[u8] {
        cio::c_str_bytes(&self.name)
    }
}

/// scene_add_shape(): returns 0 on success, -1 on failure.
pub fn scene_add_shape(scene: &mut Scene, shape_type: Option<i32>) -> i32 {
    let shape_type = match shape_type {
        None => return -1,
        Some(t) => t,
    };

    if scene.shape_count as usize >= MAX_SHAPES_IN_SCENE {
        cio::err_put("Error: Scene is full\n");
        return -1;
    }

    scene.shapes[scene.shape_count as usize] = shape_type;
    scene.shape_count += 1;
    0
}

/// scene_remove_shape()
pub fn scene_remove_shape(scene: &mut Scene, index: i32) -> i32 {
    if index < 0 || index >= scene.shape_count {
        return -1;
    }

    let mut i = index;
    while i < scene.shape_count - 1 {
        scene.shapes[i as usize] = scene.shapes[(i + 1) as usize];
        i += 1;
    }

    scene.shape_count -= 1;
    0
}

/// scene_print()
pub fn scene_print(out: &mut Out, mgr: &ShapeManager, scene: &Scene) {
    out.put("\n=== Scene: ");
    out.put_bytes(scene.name_str());
    out.put(" ===\n");
    out.put(&format!("Contains {} shape(s)\n\n", scene.shape_count));

    for i in 0..scene.shape_count {
        out.put(&format!("Shape #{}:\n", i + 1));
        shape_print(out, mgr.get(scene.shapes[i as usize]));
        out.put("\n");
    }
}

/// scene_equals()
pub fn scene_equals(s1: &Scene, s2: &Scene) -> i32 {
    if s1.shape_count != s2.shape_count {
        return 0;
    }

    let mut matched = [false; MAX_SHAPES_IN_SCENE];

    for i in 0..s1.shape_count as usize {
        let mut found = false;
        for j in 0..s2.shape_count as usize {
            if !matched[j] && s1.shapes[i] == s2.shapes[j] {
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

/// scene_save()
pub fn scene_save(out: &mut Out, scene: &Scene, filename: &[u8]) -> i32 {
    use std::io::Write;

    let path = crate::os_path(filename);
    let file = std::fs::File::create(&path);
    let mut file = match file {
        Err(_) => {
            let mut msg = Vec::new();
            msg.extend_from_slice(b"Error: Could not open file '");
            msg.extend_from_slice(filename);
            msg.extend_from_slice(b"' for writing\n");
            cio::err_put_bytes(&msg);
            return -1;
        }
        Ok(f) => f,
    };

    let mut body: Vec<u8> = Vec::new();
    body.extend_from_slice(scene.name_str());
    body.push(b'\n');
    body.extend_from_slice(scene.shape_count.to_string().as_bytes());
    body.push(b'\n');
    for i in 0..scene.shape_count as usize {
        body.extend_from_slice(scene.shapes[i].to_string().as_bytes());
        body.push(b'\n');
    }
    let _ = file.write_all(&body);
    drop(file);

    out.put("Scene saved to '");
    out.put_bytes(filename);
    out.put("'\n");
    0
}

/// scene_load(): None models the NULL return.
pub fn scene_load(out: &mut Out, mgr: &ShapeManager, filename: &[u8]) -> Option<Box<Scene>> {
    use std::io::Read;

    let path = crate::os_path(filename);
    let mut file = match std::fs::File::open(&path) {
        Err(_) => {
            let mut msg = Vec::new();
            msg.extend_from_slice(b"Error: Could not open file '");
            msg.extend_from_slice(filename);
            msg.extend_from_slice(b"' for reading\n");
            cio::err_put_bytes(&msg);
            return None;
        }
        Ok(f) => f,
    };

    let mut body: Vec<u8> = Vec::new();
    if file.read_to_end(&mut body).is_err() {
        // The stream is open but unreadable (a directory, for instance); the
        // first fgets() then fails and scene_load returns NULL.
        return None;
    }
    let mut inp = In::from_bytes(body);

    let name = inp.fgets(MAX_SCENE_NAME)?;
    let name = cio::trim_at_newline(&name);

    let mut scene = Scene::create(Some(&name));

    let shape_count = match inp.scan_int() {
        Scan::Fail => return None,
        Scan::Val(v) => v,
    };

    for _ in 0..shape_count {
        let t = match inp.scan_int() {
            Scan::Fail => return None,
            Scan::Val(v) => v,
        };

        if mgr.get(t).is_some() {
            scene_add_shape(&mut scene, Some(t));
        }
    }

    out.put("Scene loaded from '");
    out.put_bytes(filename);
    out.put("'\n");
    Some(scene)
}

/// scene_list_shapes()
pub fn scene_list_shapes(out: &mut Out, mgr: &ShapeManager, scene: &Scene) {
    out.put("\nScene: ");
    out.put_bytes(scene.name_str());
    out.put("\n");
    out.put(&format!("Shapes ({}):\n", scene.shape_count));

    for i in 0..scene.shape_count as usize {
        let t = scene.shapes[i];
        out.put(&format!("  {}. ", i + 1));
        match mgr.get(t) {
            Some(s) => out.put_bytes(cio::c_str_bytes(&s.name)),
            None => {}
        }
        out.put(&format!(" (ptr: {})\n", cio::fmt_ptr(mgr.ptr_of(t))));
    }
}
