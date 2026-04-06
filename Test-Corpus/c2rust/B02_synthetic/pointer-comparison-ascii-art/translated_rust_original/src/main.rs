#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]
#![feature(raw_ref_op)]
#[allow(unused_imports)]
use ::driver;
extern "C" {
    static mut stdin: *mut _IO_FILE;
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn scanf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn sscanf(
        __s: *const ::core::ffi::c_char,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn getchar() -> ::core::ffi::c_int;
    fn fgets(
        __s: *mut ::core::ffi::c_char,
        __n: ::core::ffi::c_int,
        __stream: *mut FILE,
    ) -> *mut ::core::ffi::c_char;
    fn strcspn(
        __s: *const ::core::ffi::c_char,
        __reject: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_ulong;
    fn scene_create(name: *const ::core::ffi::c_char) -> *mut scene_t;
    fn scene_destroy(scene: *mut scene_t);
    fn scene_add_shape(scene: *mut scene_t, shape: *mut shape_t) -> ::core::ffi::c_int;
    fn scene_remove_shape(scene: *mut scene_t, index: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn scene_print(scene: *const scene_t);
    fn scene_equals(s1: *const scene_t, s2: *const scene_t) -> ::core::ffi::c_int;
    fn scene_save(
        scene: *const scene_t,
        filename: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn scene_load(filename: *const ::core::ffi::c_char) -> *mut scene_t;
    fn scene_list_shapes(scene: *const scene_t);
    fn shape_manager_init();
    fn shape_manager_cleanup();
    fn shape_get(type_0: shape_type_t) -> *mut shape_t;
    fn shape_print(shape: *const shape_t);
    fn shape_equals(s1: *const shape_t, s2: *const shape_t) -> ::core::ffi::c_int;
    fn shape_type_name(type_0: shape_type_t) -> *const ::core::ffi::c_char;
}
pub type size_t = usize;
pub type __off_t = ::core::ffi::c_long;
pub type __off64_t = ::core::ffi::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: ::core::ffi::c_int,
    pub _IO_read_ptr: *mut ::core::ffi::c_char,
    pub _IO_read_end: *mut ::core::ffi::c_char,
    pub _IO_read_base: *mut ::core::ffi::c_char,
    pub _IO_write_base: *mut ::core::ffi::c_char,
    pub _IO_write_ptr: *mut ::core::ffi::c_char,
    pub _IO_write_end: *mut ::core::ffi::c_char,
    pub _IO_buf_base: *mut ::core::ffi::c_char,
    pub _IO_buf_end: *mut ::core::ffi::c_char,
    pub _IO_save_base: *mut ::core::ffi::c_char,
    pub _IO_backup_base: *mut ::core::ffi::c_char,
    pub _IO_save_end: *mut ::core::ffi::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: ::core::ffi::c_int,
    pub _flags2: ::core::ffi::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: ::core::ffi::c_ushort,
    pub _vtable_offset: ::core::ffi::c_schar,
    pub _shortbuf: [::core::ffi::c_char; 1],
    pub _lock: *mut ::core::ffi::c_void,
    pub _offset: __off64_t,
    pub __pad1: *mut ::core::ffi::c_void,
    pub __pad2: *mut ::core::ffi::c_void,
    pub __pad3: *mut ::core::ffi::c_void,
    pub __pad4: *mut ::core::ffi::c_void,
    pub __pad5: size_t,
    pub _mode: ::core::ffi::c_int,
    pub _unused2: [::core::ffi::c_char; 20],
}
pub type _IO_lock_t = ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_marker {
    pub _next: *mut _IO_marker,
    pub _sbuf: *mut _IO_FILE,
    pub _pos: ::core::ffi::c_int,
}
pub type FILE = _IO_FILE;
pub type shape_type_t = ::core::ffi::c_uint;
pub const SHAPE_COUNT: shape_type_t = 10;
pub const SHAPE_RAINBOW: shape_type_t = 9;
pub const SHAPE_HEART: shape_type_t = 8;
pub const SHAPE_STAR: shape_type_t = 7;
pub const SHAPE_CAR: shape_type_t = 6;
pub const SHAPE_FLOWER: shape_type_t = 5;
pub const SHAPE_CLOUD: shape_type_t = 4;
pub const SHAPE_SUN: shape_type_t = 3;
pub const SHAPE_HOUSE: shape_type_t = 2;
pub const SHAPE_TRACTOR: shape_type_t = 1;
pub const SHAPE_TREE: shape_type_t = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct shape_t {
    pub type_0: shape_type_t,
    pub name: [::core::ffi::c_char; 32],
    pub art: [[::core::ffi::c_char; 80]; 30],
    pub width: ::core::ffi::c_int,
    pub height: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct scene_t {
    pub name: [::core::ffi::c_char; 64],
    pub shapes: [*mut shape_t; 50],
    pub shape_count: ::core::ffi::c_int,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const MAX_SCENE_NAME: ::core::ffi::c_int = 64 as ::core::ffi::c_int;
pub const MAX_SCENES: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
static mut scenes: [*mut scene_t; 10] = [
    ::core::ptr::null::<scene_t>() as *mut scene_t,
    ::core::ptr::null::<scene_t>() as *mut scene_t,
    ::core::ptr::null::<scene_t>() as *mut scene_t,
    ::core::ptr::null::<scene_t>() as *mut scene_t,
    ::core::ptr::null::<scene_t>() as *mut scene_t,
    ::core::ptr::null::<scene_t>() as *mut scene_t,
    ::core::ptr::null::<scene_t>() as *mut scene_t,
    ::core::ptr::null::<scene_t>() as *mut scene_t,
    ::core::ptr::null::<scene_t>() as *mut scene_t,
    ::core::ptr::null::<scene_t>() as *mut scene_t,
];
static mut scene_count: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn print_menu() {
    printf(b"\n\0" as *const u8 as *const ::core::ffi::c_char);
    printf(
        b"=========================================\n\0" as *const u8 as *const ::core::ffi::c_char,
    );
    printf(b"  ASCII ART DRAWING APPLICATION\n\0" as *const u8 as *const ::core::ffi::c_char);
    printf(
        b"=========================================\n\0" as *const u8 as *const ::core::ffi::c_char,
    );
    printf(b"1. View all available shapes\n\0" as *const u8 as *const ::core::ffi::c_char);
    printf(b"2. Create new scene\n\0" as *const u8 as *const ::core::ffi::c_char);
    printf(b"3. Add shape to scene\n\0" as *const u8 as *const ::core::ffi::c_char);
    printf(b"4. Remove shape from scene\n\0" as *const u8 as *const ::core::ffi::c_char);
    printf(b"5. View scene\n\0" as *const u8 as *const ::core::ffi::c_char);
    printf(b"6. List all scenes\n\0" as *const u8 as *const ::core::ffi::c_char);
    printf(b"7. Save scene\n\0" as *const u8 as *const ::core::ffi::c_char);
    printf(b"8. Load scene\n\0" as *const u8 as *const ::core::ffi::c_char);
    printf(b"9. Compare two shapes\n\0" as *const u8 as *const ::core::ffi::c_char);
    printf(b"10. Compare two scenes\n\0" as *const u8 as *const ::core::ffi::c_char);
    printf(b"11. Delete scene\n\0" as *const u8 as *const ::core::ffi::c_char);
    printf(b"12. Exit\n\0" as *const u8 as *const ::core::ffi::c_char);
    printf(
        b"=========================================\n\0" as *const u8 as *const ::core::ffi::c_char,
    );
    printf(b"Choice: \0" as *const u8 as *const ::core::ffi::c_char);
}
#[no_mangle]
pub unsafe extern "C" fn view_all_shapes() {
    printf(b"\n=== Available Shapes ===\n\0" as *const u8 as *const ::core::ffi::c_char);
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < SHAPE_COUNT as ::core::ffi::c_int {
        printf(
            b"\n%d. \0" as *const u8 as *const ::core::ffi::c_char,
            i + 1 as ::core::ffi::c_int,
        );
        shape_print(shape_get(i as shape_type_t));
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn create_new_scene() {
    if scene_count >= MAX_SCENES {
        printf(b"Error: Maximum scenes reached\n\0" as *const u8 as *const ::core::ffi::c_char);
        return;
    }
    let mut name: [::core::ffi::c_char; 64] = [0; 64];
    printf(b"Enter scene name: \0" as *const u8 as *const ::core::ffi::c_char);
    if fgets(
        &raw mut name as *mut ::core::ffi::c_char,
        MAX_SCENE_NAME,
        stdin as *mut FILE,
    )
    .is_null()
    {
        return;
    }
    name[strcspn(
        &raw mut name as *mut ::core::ffi::c_char,
        b"\n\0" as *const u8 as *const ::core::ffi::c_char,
    ) as usize] = 0 as ::core::ffi::c_char;
    scenes[scene_count as usize] = scene_create(&raw mut name as *mut ::core::ffi::c_char);
    if !scenes[scene_count as usize].is_null() {
        printf(
            b"Scene '%s' created (index %d)\n\0" as *const u8 as *const ::core::ffi::c_char,
            &raw mut name as *mut ::core::ffi::c_char,
            scene_count,
        );
        scene_count += 1;
    } else {
        printf(b"Error creating scene\n\0" as *const u8 as *const ::core::ffi::c_char);
    };
}
#[no_mangle]
pub unsafe extern "C" fn add_shape_to_scene() {
    if scene_count == 0 as ::core::ffi::c_int {
        printf(
            b"No scenes available. Create a scene first.\n\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        return;
    }
    printf(
        b"Select scene (0-%d): \0" as *const u8 as *const ::core::ffi::c_char,
        scene_count - 1 as ::core::ffi::c_int,
    );
    let mut scene_idx: ::core::ffi::c_int = 0;
    if scanf(
        b"%d\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut scene_idx,
    ) != 1 as ::core::ffi::c_int
    {
        printf(b"Invalid input\n\0" as *const u8 as *const ::core::ffi::c_char);
        while getchar() != '\n' as i32 {}
        return;
    }
    while getchar() != '\n' as i32 {}
    if scene_idx < 0 as ::core::ffi::c_int || scene_idx >= scene_count {
        printf(b"Invalid scene index\n\0" as *const u8 as *const ::core::ffi::c_char);
        return;
    }
    printf(b"\nSelect shape to add:\n\0" as *const u8 as *const ::core::ffi::c_char);
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < SHAPE_COUNT as ::core::ffi::c_int {
        printf(
            b"%d. %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            i,
            shape_type_name(i as shape_type_t),
        );
        i += 1;
    }
    printf(b"Choice: \0" as *const u8 as *const ::core::ffi::c_char);
    let mut shape_type: ::core::ffi::c_int = 0;
    if scanf(
        b"%d\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut shape_type,
    ) != 1 as ::core::ffi::c_int
    {
        printf(b"Invalid input\n\0" as *const u8 as *const ::core::ffi::c_char);
        while getchar() != '\n' as i32 {}
        return;
    }
    while getchar() != '\n' as i32 {}
    if shape_type < 0 as ::core::ffi::c_int || shape_type >= SHAPE_COUNT as ::core::ffi::c_int {
        printf(b"Invalid shape type\n\0" as *const u8 as *const ::core::ffi::c_char);
        return;
    }
    let mut shape: *mut shape_t = shape_get(shape_type as shape_type_t);
    if scene_add_shape(scenes[scene_idx as usize], shape) == 0 as ::core::ffi::c_int {
        printf(
            b"Shape '%s' added to scene (reusing singleton at %p)\n\0" as *const u8
                as *const ::core::ffi::c_char,
            &raw mut (*shape).name as *mut ::core::ffi::c_char,
            shape as *mut ::core::ffi::c_void,
        );
    } else {
        printf(b"Error adding shape\n\0" as *const u8 as *const ::core::ffi::c_char);
    };
}
#[no_mangle]
pub unsafe extern "C" fn remove_shape_from_scene() {
    if scene_count == 0 as ::core::ffi::c_int {
        printf(b"No scenes available\n\0" as *const u8 as *const ::core::ffi::c_char);
        return;
    }
    printf(
        b"Select scene (0-%d): \0" as *const u8 as *const ::core::ffi::c_char,
        scene_count - 1 as ::core::ffi::c_int,
    );
    let mut scene_idx: ::core::ffi::c_int = 0;
    if scanf(
        b"%d\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut scene_idx,
    ) != 1 as ::core::ffi::c_int
    {
        printf(b"Invalid input\n\0" as *const u8 as *const ::core::ffi::c_char);
        while getchar() != '\n' as i32 {}
        return;
    }
    while getchar() != '\n' as i32 {}
    if scene_idx < 0 as ::core::ffi::c_int || scene_idx >= scene_count {
        printf(b"Invalid scene index\n\0" as *const u8 as *const ::core::ffi::c_char);
        return;
    }
    scene_list_shapes(scenes[scene_idx as usize]);
    if (*scenes[scene_idx as usize]).shape_count == 0 as ::core::ffi::c_int {
        printf(b"Scene is empty\n\0" as *const u8 as *const ::core::ffi::c_char);
        return;
    }
    printf(
        b"Select shape to remove (1-%d): \0" as *const u8 as *const ::core::ffi::c_char,
        (*scenes[scene_idx as usize]).shape_count,
    );
    let mut shape_idx: ::core::ffi::c_int = 0;
    if scanf(
        b"%d\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut shape_idx,
    ) != 1 as ::core::ffi::c_int
    {
        printf(b"Invalid input\n\0" as *const u8 as *const ::core::ffi::c_char);
        while getchar() != '\n' as i32 {}
        return;
    }
    while getchar() != '\n' as i32 {}
    if scene_remove_shape(
        scenes[scene_idx as usize],
        shape_idx - 1 as ::core::ffi::c_int,
    ) == 0 as ::core::ffi::c_int
    {
        printf(b"Shape removed\n\0" as *const u8 as *const ::core::ffi::c_char);
    } else {
        printf(b"Error removing shape\n\0" as *const u8 as *const ::core::ffi::c_char);
    };
}
#[no_mangle]
pub unsafe extern "C" fn view_scene() {
    if scene_count == 0 as ::core::ffi::c_int {
        printf(b"No scenes available\n\0" as *const u8 as *const ::core::ffi::c_char);
        return;
    }
    printf(
        b"Select scene (0-%d): \0" as *const u8 as *const ::core::ffi::c_char,
        scene_count - 1 as ::core::ffi::c_int,
    );
    let mut scene_idx: ::core::ffi::c_int = 0;
    if scanf(
        b"%d\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut scene_idx,
    ) != 1 as ::core::ffi::c_int
    {
        printf(b"Invalid input\n\0" as *const u8 as *const ::core::ffi::c_char);
        while getchar() != '\n' as i32 {}
        return;
    }
    while getchar() != '\n' as i32 {}
    if scene_idx < 0 as ::core::ffi::c_int || scene_idx >= scene_count {
        printf(b"Invalid scene index\n\0" as *const u8 as *const ::core::ffi::c_char);
        return;
    }
    scene_print(scenes[scene_idx as usize]);
}
#[no_mangle]
pub unsafe extern "C" fn list_all_scenes() {
    printf(b"\n=== All Scenes ===\n\0" as *const u8 as *const ::core::ffi::c_char);
    if scene_count == 0 as ::core::ffi::c_int {
        printf(b"No scenes created yet\n\0" as *const u8 as *const ::core::ffi::c_char);
        return;
    }
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < scene_count {
        printf(
            b"%d. %s (%d shapes)\n\0" as *const u8 as *const ::core::ffi::c_char,
            i,
            &raw mut (**(&raw mut scenes as *mut *mut scene_t).offset(i as isize)).name
                as *mut ::core::ffi::c_char,
            (*scenes[i as usize]).shape_count,
        );
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn save_scene_to_file() {
    if scene_count == 0 as ::core::ffi::c_int {
        printf(b"No scenes available\n\0" as *const u8 as *const ::core::ffi::c_char);
        return;
    }
    printf(
        b"Select scene (0-%d): \0" as *const u8 as *const ::core::ffi::c_char,
        scene_count - 1 as ::core::ffi::c_int,
    );
    let mut scene_idx: ::core::ffi::c_int = 0;
    if scanf(
        b"%d\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut scene_idx,
    ) != 1 as ::core::ffi::c_int
    {
        printf(b"Invalid input\n\0" as *const u8 as *const ::core::ffi::c_char);
        while getchar() != '\n' as i32 {}
        return;
    }
    while getchar() != '\n' as i32 {}
    if scene_idx < 0 as ::core::ffi::c_int || scene_idx >= scene_count {
        printf(b"Invalid scene index\n\0" as *const u8 as *const ::core::ffi::c_char);
        return;
    }
    let mut filename: [::core::ffi::c_char; 256] = [0; 256];
    printf(b"Enter filename: \0" as *const u8 as *const ::core::ffi::c_char);
    if fgets(
        &raw mut filename as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 256]>() as ::core::ffi::c_int,
        stdin as *mut FILE,
    )
    .is_null()
    {
        return;
    }
    filename[strcspn(
        &raw mut filename as *mut ::core::ffi::c_char,
        b"\n\0" as *const u8 as *const ::core::ffi::c_char,
    ) as usize] = 0 as ::core::ffi::c_char;
    scene_save(
        scenes[scene_idx as usize],
        &raw mut filename as *mut ::core::ffi::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn load_scene_from_file() {
    if scene_count >= MAX_SCENES {
        printf(b"Error: Maximum scenes reached\n\0" as *const u8 as *const ::core::ffi::c_char);
        return;
    }
    let mut filename: [::core::ffi::c_char; 256] = [0; 256];
    printf(b"Enter filename: \0" as *const u8 as *const ::core::ffi::c_char);
    if fgets(
        &raw mut filename as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 256]>() as ::core::ffi::c_int,
        stdin as *mut FILE,
    )
    .is_null()
    {
        return;
    }
    filename[strcspn(
        &raw mut filename as *mut ::core::ffi::c_char,
        b"\n\0" as *const u8 as *const ::core::ffi::c_char,
    ) as usize] = 0 as ::core::ffi::c_char;
    let mut scene: *mut scene_t = scene_load(&raw mut filename as *mut ::core::ffi::c_char);
    if !scene.is_null() {
        let fresh0 = scene_count;
        scene_count = scene_count + 1;
        scenes[fresh0 as usize] = scene;
        printf(
            b"Scene loaded (index %d)\n\0" as *const u8 as *const ::core::ffi::c_char,
            scene_count - 1 as ::core::ffi::c_int,
        );
    }
}
#[no_mangle]
pub unsafe extern "C" fn compare_shapes() {
    printf(
        b"\nSelect first shape (0-%d):\n\0" as *const u8 as *const ::core::ffi::c_char,
        SHAPE_COUNT as ::core::ffi::c_int - 1 as ::core::ffi::c_int,
    );
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < SHAPE_COUNT as ::core::ffi::c_int {
        printf(
            b"%d. %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            i,
            shape_type_name(i as shape_type_t),
        );
        i += 1;
    }
    printf(b"Choice: \0" as *const u8 as *const ::core::ffi::c_char);
    let mut type1: ::core::ffi::c_int = 0;
    if scanf(
        b"%d\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut type1,
    ) != 1 as ::core::ffi::c_int
    {
        printf(b"Invalid input\n\0" as *const u8 as *const ::core::ffi::c_char);
        while getchar() != '\n' as i32 {}
        return;
    }
    while getchar() != '\n' as i32 {}
    printf(
        b"\nSelect second shape (0-%d): \0" as *const u8 as *const ::core::ffi::c_char,
        SHAPE_COUNT as ::core::ffi::c_int - 1 as ::core::ffi::c_int,
    );
    let mut type2: ::core::ffi::c_int = 0;
    if scanf(
        b"%d\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut type2,
    ) != 1 as ::core::ffi::c_int
    {
        printf(b"Invalid input\n\0" as *const u8 as *const ::core::ffi::c_char);
        while getchar() != '\n' as i32 {}
        return;
    }
    while getchar() != '\n' as i32 {}
    if type1 < 0 as ::core::ffi::c_int
        || type1 >= SHAPE_COUNT as ::core::ffi::c_int
        || type2 < 0 as ::core::ffi::c_int
        || type2 >= SHAPE_COUNT as ::core::ffi::c_int
    {
        printf(b"Invalid shape type\n\0" as *const u8 as *const ::core::ffi::c_char);
        return;
    }
    let mut s1: *mut shape_t = shape_get(type1 as shape_type_t);
    let mut s2: *mut shape_t = shape_get(type2 as shape_type_t);
    printf(
        b"\nShape 1: %s (ptr: %p)\n\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut (*s1).name as *mut ::core::ffi::c_char,
        s1 as *mut ::core::ffi::c_void,
    );
    printf(
        b"Shape 2: %s (ptr: %p)\n\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut (*s2).name as *mut ::core::ffi::c_char,
        s2 as *mut ::core::ffi::c_void,
    );
    printf(
        b"Comparison of pointers: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        (s1 == s2) as ::core::ffi::c_int,
    );
    if shape_equals(s1, s2) != 0 {
        printf(
            b"Result: Shapes are EQUAL (same instance)\n\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    } else {
        printf(
            b"Result: Shapes are NOT EQUAL (different instances)\n\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    };
}
#[no_mangle]
pub unsafe extern "C" fn compare_scenes() {
    if scene_count < 2 as ::core::ffi::c_int {
        printf(b"Need at least 2 scenes to compare\n\0" as *const u8 as *const ::core::ffi::c_char);
        return;
    }
    printf(
        b"Select first scene (0-%d): \0" as *const u8 as *const ::core::ffi::c_char,
        scene_count - 1 as ::core::ffi::c_int,
    );
    let mut idx1: ::core::ffi::c_int = 0;
    if scanf(
        b"%d\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut idx1,
    ) != 1 as ::core::ffi::c_int
    {
        printf(b"Invalid input\n\0" as *const u8 as *const ::core::ffi::c_char);
        while getchar() != '\n' as i32 {}
        return;
    }
    while getchar() != '\n' as i32 {}
    printf(
        b"Select second scene (0-%d): \0" as *const u8 as *const ::core::ffi::c_char,
        scene_count - 1 as ::core::ffi::c_int,
    );
    let mut idx2: ::core::ffi::c_int = 0;
    if scanf(
        b"%d\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut idx2,
    ) != 1 as ::core::ffi::c_int
    {
        printf(b"Invalid input\n\0" as *const u8 as *const ::core::ffi::c_char);
        while getchar() != '\n' as i32 {}
        return;
    }
    while getchar() != '\n' as i32 {}
    if idx1 < 0 as ::core::ffi::c_int
        || idx1 >= scene_count
        || idx2 < 0 as ::core::ffi::c_int
        || idx2 >= scene_count
    {
        printf(b"Invalid scene index\n\0" as *const u8 as *const ::core::ffi::c_char);
        return;
    }
    let mut sc1: *mut scene_t = scenes[idx1 as usize];
    let mut sc2: *mut scene_t = scenes[idx2 as usize];
    printf(
        b"\nScene 1: %s (%d shapes)\n\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut (*sc1).name as *mut ::core::ffi::c_char,
        (*sc1).shape_count,
    );
    scene_list_shapes(sc1);
    printf(
        b"\nScene 2: %s (%d shapes)\n\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut (*sc2).name as *mut ::core::ffi::c_char,
        (*sc2).shape_count,
    );
    scene_list_shapes(sc2);
    if scene_equals(sc1, sc2) != 0 {
        printf(
            b"\nResult: Scenes are EQUAL (1:1 correspondence)\n\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    } else {
        printf(b"\nResult: Scenes are NOT EQUAL\n\0" as *const u8 as *const ::core::ffi::c_char);
    };
}
#[no_mangle]
pub unsafe extern "C" fn delete_scene() {
    if scene_count == 0 as ::core::ffi::c_int {
        printf(b"No scenes available\n\0" as *const u8 as *const ::core::ffi::c_char);
        return;
    }
    printf(
        b"Select scene to delete (0-%d): \0" as *const u8 as *const ::core::ffi::c_char,
        scene_count - 1 as ::core::ffi::c_int,
    );
    let mut scene_idx: ::core::ffi::c_int = 0;
    if scanf(
        b"%d\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut scene_idx,
    ) != 1 as ::core::ffi::c_int
    {
        printf(b"Invalid input\n\0" as *const u8 as *const ::core::ffi::c_char);
        while getchar() != '\n' as i32 {}
        return;
    }
    while getchar() != '\n' as i32 {}
    if scene_idx < 0 as ::core::ffi::c_int || scene_idx >= scene_count {
        printf(b"Invalid scene index\n\0" as *const u8 as *const ::core::ffi::c_char);
        return;
    }
    scene_destroy(scenes[scene_idx as usize]);
    let mut i: ::core::ffi::c_int = scene_idx;
    while i < scene_count - 1 as ::core::ffi::c_int {
        scenes[i as usize] = scenes[(i + 1 as ::core::ffi::c_int) as usize];
        i += 1;
    }
    scene_count -= 1;
    printf(b"Scene deleted\n\0" as *const u8 as *const ::core::ffi::c_char);
}
unsafe fn main_0() -> ::core::ffi::c_int {
    printf(
        b"\xE2\x95\x94\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x97\n\0"
            as *const u8 as *const ::core::ffi::c_char,
    );
    printf(
        b"\xE2\x95\x91  ASCII ART DRAWING APPLICATION        \xE2\x95\x91\n\0" as *const u8
            as *const ::core::ffi::c_char,
    );
    printf(
        b"\xE2\x95\x91  Child-Friendly Shape Editor           \xE2\x95\x91\n\0" as *const u8
            as *const ::core::ffi::c_char,
    );
    printf(
        b"\xE2\x95\x9A\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x9D\n\0"
            as *const u8 as *const ::core::ffi::c_char,
    );
    shape_manager_init();
    let mut input: [::core::ffi::c_char; 256] = [0; 256];
    let mut choice: ::core::ffi::c_int = 0;
    loop {
        print_menu();
        if fgets(
            &raw mut input as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 256]>() as ::core::ffi::c_int,
            stdin as *mut FILE,
        )
        .is_null()
        {
            break;
        }
        if sscanf(
            &raw mut input as *mut ::core::ffi::c_char,
            b"%d\0" as *const u8 as *const ::core::ffi::c_char,
            &raw mut choice,
        ) != 1 as ::core::ffi::c_int
        {
            printf(b"Invalid input\n\0" as *const u8 as *const ::core::ffi::c_char);
        } else {
            match choice {
                1 => {
                    view_all_shapes();
                }
                2 => {
                    create_new_scene();
                }
                3 => {
                    add_shape_to_scene();
                }
                4 => {
                    remove_shape_from_scene();
                }
                5 => {
                    view_scene();
                }
                6 => {
                    list_all_scenes();
                }
                7 => {
                    save_scene_to_file();
                }
                8 => {
                    load_scene_from_file();
                }
                9 => {
                    compare_shapes();
                }
                10 => {
                    compare_scenes();
                }
                11 => {
                    delete_scene();
                }
                12 => {
                    printf(
                        b"\nCleaning up and exiting...\n\0" as *const u8
                            as *const ::core::ffi::c_char,
                    );
                    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                    while i < scene_count {
                        scene_destroy(scenes[i as usize]);
                        i += 1;
                    }
                    shape_manager_cleanup();
                    printf(b"Goodbye!\n\0" as *const u8 as *const ::core::ffi::c_char);
                    return 0 as ::core::ffi::c_int;
                }
                _ => {
                    printf(b"Invalid choice\n\0" as *const u8 as *const ::core::ffi::c_char);
                }
            }
        }
    }
    let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i_0 < scene_count {
        scene_destroy(scenes[i_0 as usize]);
        i_0 += 1;
    }
    shape_manager_cleanup();
    return 0 as ::core::ffi::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
