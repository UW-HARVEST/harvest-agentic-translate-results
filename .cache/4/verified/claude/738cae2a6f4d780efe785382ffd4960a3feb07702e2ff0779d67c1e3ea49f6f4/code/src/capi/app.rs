//! C-ABI translation of `c_src/src/main.c` — the application level functions
//! (all of which the C shared object exports, including `main` itself).
#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_void};
use std::ptr::{self, addr_of_mut};

use super::cstdio::{c_stdin};

use super::scene::{
    scene_add_shape, scene_create, scene_destroy, scene_list_shapes, scene_load, scene_name_ptr,
    scene_print, scene_remove_shape, scene_save, scene_t, MAX_SCENE_NAME,
};
use super::shape::{
    shape_get, shape_manager_cleanup, shape_manager_init, shape_name_ptr, shape_print, shape_t,
    shape_type_name, SHAPE_COUNT,
};

const MAX_SCENES: usize = 10;

// static scene_t *scenes[MAX_SCENES] = {NULL};
// static int scene_count = 0;
static mut SCENES: [*mut scene_t; MAX_SCENES] = [ptr::null_mut(); MAX_SCENES];
static mut SCENE_COUNT: c_int = 0;

#[inline]
unsafe fn scenes_slot(i: usize) -> *mut *mut scene_t {
    (addr_of_mut!(SCENES) as *mut *mut scene_t).add(i)
}

#[inline]
unsafe fn scene_count_get() -> c_int {
    ptr::read(addr_of_mut!(SCENE_COUNT))
}

#[inline]
unsafe fn scene_count_set(v: c_int) {
    ptr::write(addr_of_mut!(SCENE_COUNT), v);
}

/// `while (getchar() != '\n');`
///
/// NOTE: reproduces the original loop exactly.  At EOF `getchar()` returns
/// `EOF` for ever, so the loop never terminates - just like the C program.
#[inline]
unsafe fn discard_line() {
    while libc::getchar() != '\n' as c_int {}
}

/// `void print_menu(void)`
#[no_mangle]
pub unsafe extern "C" fn print_menu() {
    libc::printf(c"\n".as_ptr());
    libc::printf(c"=========================================\n".as_ptr());
    libc::printf(c"  ASCII ART DRAWING APPLICATION\n".as_ptr());
    libc::printf(c"=========================================\n".as_ptr());
    libc::printf(c"1. View all available shapes\n".as_ptr());
    libc::printf(c"2. Create new scene\n".as_ptr());
    libc::printf(c"3. Add shape to scene\n".as_ptr());
    libc::printf(c"4. Remove shape from scene\n".as_ptr());
    libc::printf(c"5. View scene\n".as_ptr());
    libc::printf(c"6. List all scenes\n".as_ptr());
    libc::printf(c"7. Save scene\n".as_ptr());
    libc::printf(c"8. Load scene\n".as_ptr());
    libc::printf(c"9. Compare two shapes\n".as_ptr());
    libc::printf(c"10. Compare two scenes\n".as_ptr());
    libc::printf(c"11. Delete scene\n".as_ptr());
    libc::printf(c"12. Exit\n".as_ptr());
    libc::printf(c"=========================================\n".as_ptr());
    libc::printf(c"Choice: ".as_ptr());
}

/// `void view_all_shapes(void)`
#[no_mangle]
pub unsafe extern "C" fn view_all_shapes() {
    libc::printf(c"\n=== Available Shapes ===\n".as_ptr());
    let mut i: c_int = 0;
    while i < SHAPE_COUNT {
        libc::printf(c"\n%d. ".as_ptr(), i + 1);
        shape_print(shape_get(i));
        i += 1;
    }
}

/// `void create_new_scene(void)`
#[no_mangle]
pub unsafe extern "C" fn create_new_scene() {
    if scene_count_get() as usize >= MAX_SCENES {
        libc::printf(c"Error: Maximum scenes reached\n".as_ptr());
        return;
    }

    let mut name: [c_char; MAX_SCENE_NAME] = [0; MAX_SCENE_NAME];
    libc::printf(c"Enter scene name: ".as_ptr());
    if libc::fgets(name.as_mut_ptr(), MAX_SCENE_NAME as c_int, c_stdin()).is_null() {
        return;
    }
    let cut = libc::strcspn(name.as_ptr(), c"\n".as_ptr());
    name[cut] = 0;

    let count = scene_count_get();
    *scenes_slot(count as usize) = scene_create(name.as_ptr());
    if !(*scenes_slot(count as usize)).is_null() {
        libc::printf(
            c"Scene '%s' created (index %d)\n".as_ptr(),
            name.as_ptr(),
            count,
        );
        scene_count_set(count + 1);
    } else {
        libc::printf(c"Error creating scene\n".as_ptr());
    }
}

/// `void add_shape_to_scene(void)`
#[no_mangle]
pub unsafe extern "C" fn add_shape_to_scene() {
    if scene_count_get() == 0 {
        libc::printf(c"No scenes available. Create a scene first.\n".as_ptr());
        return;
    }

    libc::printf(c"Select scene (0-%d): ".as_ptr(), scene_count_get() - 1);
    let mut scene_idx: c_int = 0;
    if libc::scanf(c"%d".as_ptr(), &mut scene_idx as *mut c_int) != 1 {
        libc::printf(c"Invalid input\n".as_ptr());
        discard_line();
        return;
    }
    discard_line();

    if scene_idx < 0 || scene_idx >= scene_count_get() {
        libc::printf(c"Invalid scene index\n".as_ptr());
        return;
    }

    libc::printf(c"\nSelect shape to add:\n".as_ptr());
    let mut i: c_int = 0;
    while i < SHAPE_COUNT {
        libc::printf(c"%d. %s\n".as_ptr(), i, shape_type_name(i));
        i += 1;
    }
    libc::printf(c"Choice: ".as_ptr());

    let mut shape_type: c_int = 0;
    if libc::scanf(c"%d".as_ptr(), &mut shape_type as *mut c_int) != 1 {
        libc::printf(c"Invalid input\n".as_ptr());
        discard_line();
        return;
    }
    discard_line();

    if shape_type < 0 || shape_type >= SHAPE_COUNT {
        libc::printf(c"Invalid shape type\n".as_ptr());
        return;
    }

    let shape: *mut shape_t = shape_get(shape_type);
    if scene_add_shape(*scenes_slot(scene_idx as usize), shape) == 0 {
        libc::printf(
            c"Shape '%s' added to scene (reusing singleton at %p)\n".as_ptr(),
            shape_name_ptr(shape),
            shape as *const c_void,
        );
    } else {
        libc::printf(c"Error adding shape\n".as_ptr());
    }
}

/// `void remove_shape_from_scene(void)`
#[no_mangle]
pub unsafe extern "C" fn remove_shape_from_scene() {
    if scene_count_get() == 0 {
        libc::printf(c"No scenes available\n".as_ptr());
        return;
    }

    libc::printf(c"Select scene (0-%d): ".as_ptr(), scene_count_get() - 1);
    let mut scene_idx: c_int = 0;
    if libc::scanf(c"%d".as_ptr(), &mut scene_idx as *mut c_int) != 1 {
        libc::printf(c"Invalid input\n".as_ptr());
        discard_line();
        return;
    }
    discard_line();

    if scene_idx < 0 || scene_idx >= scene_count_get() {
        libc::printf(c"Invalid scene index\n".as_ptr());
        return;
    }

    scene_list_shapes(*scenes_slot(scene_idx as usize));

    if (**scenes_slot(scene_idx as usize)).shape_count == 0 {
        libc::printf(c"Scene is empty\n".as_ptr());
        return;
    }

    libc::printf(
        c"Select shape to remove (1-%d): ".as_ptr(),
        (**scenes_slot(scene_idx as usize)).shape_count,
    );
    let mut shape_idx: c_int = 0;
    if libc::scanf(c"%d".as_ptr(), &mut shape_idx as *mut c_int) != 1 {
        libc::printf(c"Invalid input\n".as_ptr());
        discard_line();
        return;
    }
    discard_line();

    if scene_remove_shape(
        *scenes_slot(scene_idx as usize),
        shape_idx.wrapping_sub(1),
    ) == 0
    {
        libc::printf(c"Shape removed\n".as_ptr());
    } else {
        libc::printf(c"Error removing shape\n".as_ptr());
    }
}

/// `void view_scene(void)`
#[no_mangle]
pub unsafe extern "C" fn view_scene() {
    if scene_count_get() == 0 {
        libc::printf(c"No scenes available\n".as_ptr());
        return;
    }

    libc::printf(c"Select scene (0-%d): ".as_ptr(), scene_count_get() - 1);
    let mut scene_idx: c_int = 0;
    if libc::scanf(c"%d".as_ptr(), &mut scene_idx as *mut c_int) != 1 {
        libc::printf(c"Invalid input\n".as_ptr());
        discard_line();
        return;
    }
    discard_line();

    if scene_idx < 0 || scene_idx >= scene_count_get() {
        libc::printf(c"Invalid scene index\n".as_ptr());
        return;
    }

    scene_print(*scenes_slot(scene_idx as usize));
}

/// `void list_all_scenes(void)`
#[no_mangle]
pub unsafe extern "C" fn list_all_scenes() {
    libc::printf(c"\n=== All Scenes ===\n".as_ptr());
    if scene_count_get() == 0 {
        libc::printf(c"No scenes created yet\n".as_ptr());
        return;
    }

    let mut i: c_int = 0;
    while i < scene_count_get() {
        let scene = *scenes_slot(i as usize);
        libc::printf(
            c"%d. %s (%d shapes)\n".as_ptr(),
            i,
            scene_name_ptr(scene),
            (*scene).shape_count,
        );
        i += 1;
    }
}

/// `void save_scene_to_file(void)`
#[no_mangle]
pub unsafe extern "C" fn save_scene_to_file() {
    if scene_count_get() == 0 {
        libc::printf(c"No scenes available\n".as_ptr());
        return;
    }

    libc::printf(c"Select scene (0-%d): ".as_ptr(), scene_count_get() - 1);
    let mut scene_idx: c_int = 0;
    if libc::scanf(c"%d".as_ptr(), &mut scene_idx as *mut c_int) != 1 {
        libc::printf(c"Invalid input\n".as_ptr());
        discard_line();
        return;
    }
    discard_line();

    if scene_idx < 0 || scene_idx >= scene_count_get() {
        libc::printf(c"Invalid scene index\n".as_ptr());
        return;
    }

    let mut filename: [c_char; 256] = [0; 256];
    libc::printf(c"Enter filename: ".as_ptr());
    if libc::fgets(filename.as_mut_ptr(), 256, c_stdin()).is_null() {
        return;
    }
    let cut = libc::strcspn(filename.as_ptr(), c"\n".as_ptr());
    filename[cut] = 0;

    scene_save(*scenes_slot(scene_idx as usize), filename.as_ptr());
}

/// `void load_scene_from_file(void)`
#[no_mangle]
pub unsafe extern "C" fn load_scene_from_file() {
    if scene_count_get() as usize >= MAX_SCENES {
        libc::printf(c"Error: Maximum scenes reached\n".as_ptr());
        return;
    }

    let mut filename: [c_char; 256] = [0; 256];
    libc::printf(c"Enter filename: ".as_ptr());
    if libc::fgets(filename.as_mut_ptr(), 256, c_stdin()).is_null() {
        return;
    }
    let cut = libc::strcspn(filename.as_ptr(), c"\n".as_ptr());
    filename[cut] = 0;

    let scene = scene_load(filename.as_ptr());
    if !scene.is_null() {
        let count = scene_count_get();
        *scenes_slot(count as usize) = scene;
        scene_count_set(count + 1);
        libc::printf(c"Scene loaded (index %d)\n".as_ptr(), scene_count_get() - 1);
    }
}

/// `void compare_shapes(void)`
#[no_mangle]
pub unsafe extern "C" fn compare_shapes() {
    libc::printf(c"\nSelect first shape (0-%d):\n".as_ptr(), SHAPE_COUNT - 1);
    let mut i: c_int = 0;
    while i < SHAPE_COUNT {
        libc::printf(c"%d. %s\n".as_ptr(), i, shape_type_name(i));
        i += 1;
    }
    libc::printf(c"Choice: ".as_ptr());

    let mut type1: c_int = 0;
    if libc::scanf(c"%d".as_ptr(), &mut type1 as *mut c_int) != 1 {
        libc::printf(c"Invalid input\n".as_ptr());
        discard_line();
        return;
    }
    discard_line();

    libc::printf(c"\nSelect second shape (0-%d): ".as_ptr(), SHAPE_COUNT - 1);
    let mut type2: c_int = 0;
    if libc::scanf(c"%d".as_ptr(), &mut type2 as *mut c_int) != 1 {
        libc::printf(c"Invalid input\n".as_ptr());
        discard_line();
        return;
    }
    discard_line();

    if type1 < 0 || type1 >= SHAPE_COUNT || type2 < 0 || type2 >= SHAPE_COUNT {
        libc::printf(c"Invalid shape type\n".as_ptr());
        return;
    }

    let s1: *mut shape_t = shape_get(type1);
    let s2: *mut shape_t = shape_get(type2);

    libc::printf(
        c"\nShape 1: %s (ptr: %p)\n".as_ptr(),
        shape_name_ptr(s1),
        s1 as *const c_void,
    );
    libc::printf(
        c"Shape 2: %s (ptr: %p)\n".as_ptr(),
        shape_name_ptr(s2),
        s2 as *const c_void,
    );
    libc::printf(
        c"Comparison of pointers: %d\n".as_ptr(),
        if s1 == s2 { 1 } else { 0 } as c_int,
    );

    if super::shape::shape_equals(s1, s2) != 0 {
        libc::printf(c"Result: Shapes are EQUAL (same instance)\n".as_ptr());
    } else {
        libc::printf(c"Result: Shapes are NOT EQUAL (different instances)\n".as_ptr());
    }
}

/// `void compare_scenes(void)`
#[no_mangle]
pub unsafe extern "C" fn compare_scenes() {
    if scene_count_get() < 2 {
        libc::printf(c"Need at least 2 scenes to compare\n".as_ptr());
        return;
    }

    libc::printf(
        c"Select first scene (0-%d): ".as_ptr(),
        scene_count_get() - 1,
    );
    let mut idx1: c_int = 0;
    if libc::scanf(c"%d".as_ptr(), &mut idx1 as *mut c_int) != 1 {
        libc::printf(c"Invalid input\n".as_ptr());
        discard_line();
        return;
    }
    discard_line();

    libc::printf(
        c"Select second scene (0-%d): ".as_ptr(),
        scene_count_get() - 1,
    );
    let mut idx2: c_int = 0;
    if libc::scanf(c"%d".as_ptr(), &mut idx2 as *mut c_int) != 1 {
        libc::printf(c"Invalid input\n".as_ptr());
        discard_line();
        return;
    }
    discard_line();

    if idx1 < 0 || idx1 >= scene_count_get() || idx2 < 0 || idx2 >= scene_count_get() {
        libc::printf(c"Invalid scene index\n".as_ptr());
        return;
    }

    let sc1 = *scenes_slot(idx1 as usize);
    let sc2 = *scenes_slot(idx2 as usize);

    libc::printf(
        c"\nScene 1: %s (%d shapes)\n".as_ptr(),
        scene_name_ptr(sc1),
        (*sc1).shape_count,
    );
    scene_list_shapes(sc1);

    libc::printf(
        c"\nScene 2: %s (%d shapes)\n".as_ptr(),
        scene_name_ptr(sc2),
        (*sc2).shape_count,
    );
    scene_list_shapes(sc2);

    if super::scene::scene_equals(sc1, sc2) != 0 {
        libc::printf(c"\nResult: Scenes are EQUAL (1:1 correspondence)\n".as_ptr());
    } else {
        libc::printf(c"\nResult: Scenes are NOT EQUAL\n".as_ptr());
    }
}

/// `void delete_scene(void)`
#[no_mangle]
pub unsafe extern "C" fn delete_scene() {
    if scene_count_get() == 0 {
        libc::printf(c"No scenes available\n".as_ptr());
        return;
    }

    libc::printf(
        c"Select scene to delete (0-%d): ".as_ptr(),
        scene_count_get() - 1,
    );
    let mut scene_idx: c_int = 0;
    if libc::scanf(c"%d".as_ptr(), &mut scene_idx as *mut c_int) != 1 {
        libc::printf(c"Invalid input\n".as_ptr());
        discard_line();
        return;
    }
    discard_line();

    if scene_idx < 0 || scene_idx >= scene_count_get() {
        libc::printf(c"Invalid scene index\n".as_ptr());
        return;
    }

    scene_destroy(*scenes_slot(scene_idx as usize));

    // Shift remaining scenes
    let mut i: c_int = scene_idx;
    while i < scene_count_get() - 1 {
        *scenes_slot(i as usize) = *scenes_slot((i + 1) as usize);
        i += 1;
    }

    scene_count_set(scene_count_get() - 1);
    libc::printf(c"Scene deleted\n".as_ptr());
}

/// `int main(void)`
#[export_name = "main"]
pub unsafe extern "C" fn main_c() -> c_int {
    print_banner();

    // Initialize shape manager (allocate all shapes once)
    shape_manager_init();

    let mut input: [c_char; 256] = [0; 256];
    let mut choice: c_int = 0;

    loop {
        print_menu();

        if libc::fgets(input.as_mut_ptr(), 256, c_stdin()).is_null() {
            break;
        }

        if libc::sscanf(input.as_ptr(), c"%d".as_ptr(), &mut choice as *mut c_int) != 1 {
            libc::printf(c"Invalid input\n".as_ptr());
            continue;
        }

        match choice {
            1 => view_all_shapes(),
            2 => create_new_scene(),
            3 => add_shape_to_scene(),
            4 => remove_shape_from_scene(),
            5 => view_scene(),
            6 => list_all_scenes(),
            7 => save_scene_to_file(),
            8 => load_scene_from_file(),
            9 => compare_shapes(),
            10 => compare_scenes(),
            11 => delete_scene(),
            12 => {
                libc::printf(c"\nCleaning up and exiting...\n".as_ptr());
                let mut i: c_int = 0;
                while i < scene_count_get() {
                    scene_destroy(*scenes_slot(i as usize));
                    i += 1;
                }
                shape_manager_cleanup();
                libc::printf(c"Goodbye!\n".as_ptr());
                return 0;
            }
            _ => {
                libc::printf(c"Invalid choice\n".as_ptr());
            }
        }
    }

    // Cleanup
    let mut i: c_int = 0;
    while i < scene_count_get() {
        scene_destroy(*scenes_slot(i as usize));
        i += 1;
    }
    shape_manager_cleanup();

    0
}
/// The banner `main` prints before anything else (exact bytes taken from
/// `main.c`).
unsafe fn print_banner() {
    libc::printf(c"╔════════════════════════════════════════╗\n".as_ptr());
    libc::printf(c"║  ASCII ART DRAWING APPLICATION        ║\n".as_ptr());
    libc::printf(c"║  Child-Friendly Shape Editor           ║\n".as_ptr());
    libc::printf(c"╚════════════════════════════════════════╝\n".as_ptr());
}
