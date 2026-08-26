//! C-ABI translation of `c_src/src/scene.c` (and `scene.h`).
#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_void};
use std::ptr::{self, addr_of_mut};

use super::cstdio::{c_stderr};

use super::shape::{
    shape_get, shape_name_ptr, shape_print, shape_t, strcpy_lit,
};

// ---------------------------------------------------------------------------
// scene.h
// ---------------------------------------------------------------------------

pub const MAX_SHAPES_IN_SCENE: usize = 50;
pub const MAX_SCENE_NAME: usize = 64;

/// `typedef struct { char name[64]; shape_t *shapes[50]; int shape_count; } scene_t;`
#[repr(C)]
pub struct scene_t {
    pub name: [c_char; MAX_SCENE_NAME],
    pub shapes: [*mut shape_t; MAX_SHAPES_IN_SCENE],
    pub shape_count: c_int,
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// `&scene->name[0]`
#[inline]
pub(crate) unsafe fn scene_name_ptr(scene: *mut scene_t) -> *mut c_char {
    addr_of_mut!((*scene).name) as *mut c_char
}

/// `&scene->shapes[i]`
#[inline]
pub(crate) unsafe fn scene_shape_slot(scene: *mut scene_t, i: usize) -> *mut *mut shape_t {
    (addr_of_mut!((*scene).shapes) as *mut *mut shape_t).add(i)
}

// ---------------------------------------------------------------------------
// public API
// ---------------------------------------------------------------------------

/// `scene_t* scene_create(const char *name)`
#[no_mangle]
pub unsafe extern "C" fn scene_create(name: *const c_char) -> *mut scene_t {
    let scene = libc::malloc(std::mem::size_of::<scene_t>()) as *mut scene_t;
    if scene.is_null() {
        return ptr::null_mut();
    }

    if !name.is_null() {
        libc::strncpy(scene_name_ptr(scene), name, MAX_SCENE_NAME - 1);
        *scene_name_ptr(scene).add(MAX_SCENE_NAME - 1) = 0;
    } else {
        strcpy_lit(scene_name_ptr(scene), b"Untitled Scene");
    }

    (*scene).shape_count = 0;

    scene
}

/// `void scene_destroy(scene_t *scene)`
#[no_mangle]
pub unsafe extern "C" fn scene_destroy(scene: *mut scene_t) {
    if !scene.is_null() {
        // Note: We don't free the shapes themselves
        // because they are singletons managed by shape_manager
        libc::free(scene as *mut c_void);
    }
}

/// `int scene_add_shape(scene_t *scene, shape_t *shape)`
#[no_mangle]
pub unsafe extern "C" fn scene_add_shape(scene: *mut scene_t, shape: *mut shape_t) -> c_int {
    if scene.is_null() || shape.is_null() {
        return -1;
    }

    if (*scene).shape_count as usize >= MAX_SHAPES_IN_SCENE {
        libc::fprintf(c_stderr(), c"Error: Scene is full\n".as_ptr());
        return -1;
    }

    let count = (*scene).shape_count;
    *scene_shape_slot(scene, count as usize) = shape;
    (*scene).shape_count = count + 1;
    0
}

/// `int scene_remove_shape(scene_t *scene, int index)`
#[no_mangle]
pub unsafe extern "C" fn scene_remove_shape(scene: *mut scene_t, index: c_int) -> c_int {
    if scene.is_null() || index < 0 || index >= (*scene).shape_count {
        return -1;
    }

    // Shift remaining shapes
    let mut i: c_int = index;
    while i < (*scene).shape_count - 1 {
        *scene_shape_slot(scene, i as usize) = *scene_shape_slot(scene, (i + 1) as usize);
        i += 1;
    }

    (*scene).shape_count -= 1;
    0
}

/// `void scene_print(const scene_t *scene)`
#[no_mangle]
pub unsafe extern "C" fn scene_print(scene: *const scene_t) {
    if scene.is_null() {
        libc::printf(c"(null scene)\n".as_ptr());
        return;
    }

    let scene = scene as *mut scene_t;
    libc::printf(c"\n=== Scene: %s ===\n".as_ptr(), scene_name_ptr(scene));
    libc::printf(
        c"Contains %d shape(s)\n\n".as_ptr(),
        (*scene).shape_count,
    );

    let mut i: c_int = 0;
    while i < (*scene).shape_count {
        libc::printf(c"Shape #%d:\n".as_ptr(), i + 1);
        shape_print(*scene_shape_slot(scene, i as usize));
        libc::printf(c"\n".as_ptr());
        i += 1;
    }
}

/// `int scene_equals(const scene_t *s1, const scene_t *s2)`
#[no_mangle]
pub unsafe extern "C" fn scene_equals(s1: *const scene_t, s2: *const scene_t) -> c_int {
    if s1.is_null() || s2.is_null() {
        return 0;
    }

    let s1 = s1 as *mut scene_t;
    let s2 = s2 as *mut scene_t;

    // Scenes are equal if there's a 1:1 correspondence
    if (*s1).shape_count != (*s2).shape_count {
        return 0;
    }

    // For each shape in s1, find a matching shape in s2
    let mut matched: [c_int; MAX_SHAPES_IN_SCENE] = [0; MAX_SHAPES_IN_SCENE];

    let mut i: c_int = 0;
    while i < (*s1).shape_count {
        let mut found: c_int = 0;
        let mut j: c_int = 0;
        while j < (*s2).shape_count {
            if matched[j as usize] == 0
                && super::shape::shape_equals(
                    *scene_shape_slot(s1, i as usize),
                    *scene_shape_slot(s2, j as usize),
                ) != 0
            {
                matched[j as usize] = 1;
                found = 1;
                break;
            }
            j += 1;
        }
        if found == 0 {
            return 0;
        }
        i += 1;
    }

    1
}

/// `int scene_save(const scene_t *scene, const char *filename)`
#[no_mangle]
pub unsafe extern "C" fn scene_save(scene: *const scene_t, filename: *const c_char) -> c_int {
    if scene.is_null() || filename.is_null() {
        return -1;
    }

    let scene = scene as *mut scene_t;
    let file = libc::fopen(filename, c"w".as_ptr());
    if file.is_null() {
        libc::fprintf(
            c_stderr(),
            c"Error: Could not open file '%s' for writing\n".as_ptr(),
            filename,
        );
        return -1;
    }

    // Write scene name
    libc::fprintf(file, c"%s\n".as_ptr(), scene_name_ptr(scene));

    // Write shape count
    libc::fprintf(file, c"%d\n".as_ptr(), (*scene).shape_count);

    // Write shape types (not the shapes themselves, just their types)
    let mut i: c_int = 0;
    while i < (*scene).shape_count {
        let shape = *scene_shape_slot(scene, i as usize);
        libc::fprintf(file, c"%d\n".as_ptr(), (*shape).type_);
        i += 1;
    }

    libc::fclose(file);
    libc::printf(c"Scene saved to '%s'\n".as_ptr(), filename);
    0
}

/// `scene_t* scene_load(const char *filename)`
#[no_mangle]
pub unsafe extern "C" fn scene_load(filename: *const c_char) -> *mut scene_t {
    if filename.is_null() {
        return ptr::null_mut();
    }

    let file = libc::fopen(filename, c"r".as_ptr());
    if file.is_null() {
        libc::fprintf(
            c_stderr(),
            c"Error: Could not open file '%s' for reading\n".as_ptr(),
            filename,
        );
        return ptr::null_mut();
    }

    let mut name: [c_char; MAX_SCENE_NAME] = [0; MAX_SCENE_NAME];
    if libc::fgets(name.as_mut_ptr(), MAX_SCENE_NAME as c_int, file).is_null() {
        libc::fclose(file);
        return ptr::null_mut();
    }

    // Remove newline
    let cut = libc::strcspn(name.as_ptr(), c"\n".as_ptr());
    name[cut] = 0;

    let scene = scene_create(name.as_ptr());
    if scene.is_null() {
        libc::fclose(file);
        return ptr::null_mut();
    }

    let mut shape_count: c_int = 0;
    if libc::fscanf(file, c"%d\n".as_ptr(), &mut shape_count as *mut c_int) != 1 {
        scene_destroy(scene);
        libc::fclose(file);
        return ptr::null_mut();
    }

    let mut i: c_int = 0;
    while i < shape_count {
        let mut type_: c_int = 0;
        if libc::fscanf(file, c"%d\n".as_ptr(), &mut type_ as *mut c_int) != 1 {
            scene_destroy(scene);
            libc::fclose(file);
            return ptr::null_mut();
        }

        let shape = shape_get(type_);
        if !shape.is_null() {
            scene_add_shape(scene, shape);
        }
        i += 1;
    }

    libc::fclose(file);
    libc::printf(c"Scene loaded from '%s'\n".as_ptr(), filename);
    scene
}

/// `void scene_list_shapes(const scene_t *scene)`
#[no_mangle]
pub unsafe extern "C" fn scene_list_shapes(scene: *const scene_t) {
    if scene.is_null() {
        libc::printf(c"(null scene)\n".as_ptr());
        return;
    }

    let scene = scene as *mut scene_t;
    libc::printf(c"\nScene: %s\n".as_ptr(), scene_name_ptr(scene));
    libc::printf(c"Shapes (%d):\n".as_ptr(), (*scene).shape_count);

    let mut i: c_int = 0;
    while i < (*scene).shape_count {
        let shape = *scene_shape_slot(scene, i as usize);
        libc::printf(
            c"  %d. %s (ptr: %p)\n".as_ptr(),
            i + 1,
            shape_name_ptr(shape),
            shape as *const c_void,
        );
        i += 1;
    }
}
