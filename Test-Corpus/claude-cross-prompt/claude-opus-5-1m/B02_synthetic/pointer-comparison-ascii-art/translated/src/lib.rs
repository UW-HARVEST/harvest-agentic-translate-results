// Translation of C ASCII art library (shape.c + scene.c) to Rust.
// Preserves byte-identical output by using libc's printf/fprintf/fopen, etc.

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::ptr;

// ============================================================
// Constants (matching the C #defines)
// ============================================================

pub const MAX_SHAPE_WIDTH: usize = 80;
pub const MAX_SHAPE_HEIGHT: usize = 30;
pub const MAX_SHAPE_NAME: usize = 32;

pub const MAX_SHAPES_IN_SCENE: usize = 50;
pub const MAX_SCENE_NAME: usize = 64;

// shape_type_t enum values
pub const SHAPE_TREE: c_int = 0;
pub const SHAPE_TRACTOR: c_int = 1;
pub const SHAPE_HOUSE: c_int = 2;
pub const SHAPE_SUN: c_int = 3;
pub const SHAPE_CLOUD: c_int = 4;
pub const SHAPE_FLOWER: c_int = 5;
pub const SHAPE_CAR: c_int = 6;
pub const SHAPE_STAR: c_int = 7;
pub const SHAPE_HEART: c_int = 8;
pub const SHAPE_RAINBOW: c_int = 9;
pub const SHAPE_COUNT: c_int = 10;
const SHAPE_COUNT_USIZE: usize = SHAPE_COUNT as usize;

pub type shape_type_t = c_int;

// ============================================================
// Structs (must mirror C layout)
// ============================================================

#[repr(C)]
pub struct shape_t {
    pub type_: c_int, // 'type' is a Rust keyword; layout is identical
    pub name: [c_char; MAX_SHAPE_NAME],
    pub art: [[c_char; MAX_SHAPE_WIDTH]; MAX_SHAPE_HEIGHT],
    pub width: c_int,
    pub height: c_int,
}

#[repr(C)]
pub struct scene_t {
    pub name: [c_char; MAX_SCENE_NAME],
    pub shapes: [*mut shape_t; MAX_SHAPES_IN_SCENE],
    pub shape_count: c_int,
}

// ============================================================
// Singleton shape storage
// ============================================================

static mut SHAPES: [*mut shape_t; SHAPE_COUNT_USIZE] =
    [ptr::null_mut(); SHAPE_COUNT_USIZE];

// ============================================================
// Helpers
// ============================================================

/// Copy a byte string `src` (no embedded NUL) into a C buffer at `dst`,
/// writing a NUL terminator. Mirrors strcpy semantics for ASCII art lines.
unsafe fn cstrcpy(dst: *mut c_char, src: &[u8]) {
    for (i, &b) in src.iter().enumerate() {
        unsafe { *dst.add(i) = b as c_char };
    }
    unsafe { *dst.add(src.len()) = 0 };
}

// Format strings for libc::printf / libc::fprintf
// All end with a trailing NUL byte.
const FMT_NULL_SHAPE: &[u8] = b"(null shape)\n\0";
const FMT_NAME_COLON: &[u8] = b"%s:\n\0";
const FMT_S: &[u8] = b"%s\n\0";
const FMT_NULL_SCENE: &[u8] = b"(null scene)\n\0";
const FMT_SCENE_HEADER: &[u8] = b"\n=== Scene: %s ===\n\0";
const FMT_CONTAINS: &[u8] = b"Contains %d shape(s)\n\n\0";
const FMT_SHAPE_NUM: &[u8] = b"Shape #%d:\n\0";
const FMT_NL: &[u8] = b"\n\0";
const FMT_ERR_SCENE_FULL: &[u8] = b"Error: Scene is full\n\0";
const FMT_ERR_OPEN_WRITE: &[u8] = b"Error: Could not open file '%s' for writing\n\0";
const FMT_ERR_OPEN_READ: &[u8] = b"Error: Could not open file '%s' for reading\n\0";
const FMT_FILE_NAME_NL: &[u8] = b"%s\n\0";
const FMT_DECIMAL_NL: &[u8] = b"%d\n\0";
const FMT_SAVED_TO: &[u8] = b"Scene saved to '%s'\n\0";
const FMT_LOADED_FROM: &[u8] = b"Scene loaded from '%s'\n\0";
const FMT_SCENE_NAME_LINE: &[u8] = b"\nScene: %s\n\0";
const FMT_SHAPES_COUNT: &[u8] = b"Shapes (%d):\n\0";
const FMT_LIST_ENTRY: &[u8] = b"  %d. %s (ptr: %p)\n\0";
const FMT_ALLOC_FAIL: &[u8] = b"Error: Failed to allocate shape\n\0";

const DEFAULT_SCENE_NAME: &[u8] = b"Untitled Scene\0";

// ============================================================
// Shape initialization (mirrors C init_* functions)
// ============================================================

unsafe fn set_shape_name(shape: *mut shape_t, src: &[u8]) {
    let name_ptr = unsafe { (*shape).name.as_mut_ptr() };
    unsafe { cstrcpy(name_ptr, src) };
}

unsafe fn set_art_line(shape: *mut shape_t, line: usize, src: &[u8]) {
    let line_ptr = unsafe { (*shape).art[line].as_mut_ptr() };
    unsafe { cstrcpy(line_ptr, src) };
}

unsafe fn init_tree(shape: *mut shape_t) {
    unsafe {
        (*shape).type_ = SHAPE_TREE;
        set_shape_name(shape, b"Tree");
        (*shape).height = 7;
        (*shape).width = 11;

        set_art_line(shape, 0, b"    /\\    ");
        set_art_line(shape, 1, b"   /  \\   ");
        set_art_line(shape, 2, b"  /____\\  ");
        set_art_line(shape, 3, b"  /    \\  ");
        set_art_line(shape, 4, b" /______\\ ");
        set_art_line(shape, 5, b"    ||    ");
        set_art_line(shape, 6, b"    ||    ");
    }
}

unsafe fn init_tractor(shape: *mut shape_t) {
    unsafe {
        (*shape).type_ = SHAPE_TRACTOR;
        set_shape_name(shape, b"Tractor");
        (*shape).height = 6;
        (*shape).width = 20;

        set_art_line(shape, 0, b"      ________     ");
        set_art_line(shape, 1, b"     |        |___ ");
        set_art_line(shape, 2, b"     |  []  []|   |");
        set_art_line(shape, 3, b"  ___|________|___|");
        set_art_line(shape, 4, b" /  o        o   \\");
        set_art_line(shape, 5, b"|___|        |___| ");
    }
}

unsafe fn init_house(shape: *mut shape_t) {
    unsafe {
        (*shape).type_ = SHAPE_HOUSE;
        set_shape_name(shape, b"House");
        (*shape).height = 7;
        (*shape).width = 13;

        set_art_line(shape, 0, b"     /\\     ");
        set_art_line(shape, 1, b"    /  \\    ");
        set_art_line(shape, 2, b"   /____\\   ");
        set_art_line(shape, 3, b"   |    |   ");
        set_art_line(shape, 4, b"   | [] |   ");
        set_art_line(shape, 5, b"   |    |   ");
        set_art_line(shape, 6, b"   |____|   ");
    }
}

unsafe fn init_sun(shape: *mut shape_t) {
    unsafe {
        (*shape).type_ = SHAPE_SUN;
        set_shape_name(shape, b"Sun");
        (*shape).height = 7;
        (*shape).width = 11;

        set_art_line(shape, 0, b"  \\  |  / ");
        set_art_line(shape, 1, b"   \\ | /  ");
        set_art_line(shape, 2, b"--- (@) ---");
        set_art_line(shape, 3, b"   / | \\  ");
        set_art_line(shape, 4, b"  /  |  \\ ");
        set_art_line(shape, 5, b"          ");
        set_art_line(shape, 6, b"          ");
    }
}

unsafe fn init_cloud(shape: *mut shape_t) {
    unsafe {
        (*shape).type_ = SHAPE_CLOUD;
        set_shape_name(shape, b"Cloud");
        (*shape).height = 4;
        (*shape).width = 16;

        set_art_line(shape, 0, b"   _____       ");
        set_art_line(shape, 1, b"  /     \\_    ");
        set_art_line(shape, 2, b" /  ___  _\\  ");
        set_art_line(shape, 3, b"(__/   \\_)   ");
    }
}

unsafe fn init_flower(shape: *mut shape_t) {
    unsafe {
        (*shape).type_ = SHAPE_FLOWER;
        set_shape_name(shape, b"Flower");
        (*shape).height = 7;
        (*shape).width = 9;

        set_art_line(shape, 0, b"  \\|/  ");
        set_art_line(shape, 1, b" -(@)- ");
        set_art_line(shape, 2, b"  /|\\  ");
        set_art_line(shape, 3, b"   |   ");
        set_art_line(shape, 4, b"   |   ");
        set_art_line(shape, 5, b"  / \\  ");
        set_art_line(shape, 6, b" /   \\ ");
    }
}

unsafe fn init_car(shape: *mut shape_t) {
    unsafe {
        (*shape).type_ = SHAPE_CAR;
        set_shape_name(shape, b"Car");
        (*shape).height = 4;
        (*shape).width = 16;

        set_art_line(shape, 0, b"  ____       ");
        set_art_line(shape, 1, b" /|_||_\\____ ");
        set_art_line(shape, 2, b"( o     o  ) ");
        set_art_line(shape, 3, b" -----------  ");
    }
}

unsafe fn init_star(shape: *mut shape_t) {
    unsafe {
        (*shape).type_ = SHAPE_STAR;
        set_shape_name(shape, b"Star");
        (*shape).height = 5;
        (*shape).width = 9;

        set_art_line(shape, 0, b"    *    ");
        set_art_line(shape, 1, b"   ***   ");
        set_art_line(shape, 2, b"  *****  ");
        set_art_line(shape, 3, b" ******* ");
        set_art_line(shape, 4, b"*********");
    }
}

unsafe fn init_heart(shape: *mut shape_t) {
    unsafe {
        (*shape).type_ = SHAPE_HEART;
        set_shape_name(shape, b"Heart");
        (*shape).height = 6;
        (*shape).width = 11;

        set_art_line(shape, 0, b" *** ***  ");
        set_art_line(shape, 1, b"*********  ");
        set_art_line(shape, 2, b"*********  ");
        set_art_line(shape, 3, b" ******* ");
        set_art_line(shape, 4, b"  *****  ");
        set_art_line(shape, 5, b"   ***   ");
    }
}

unsafe fn init_rainbow(shape: *mut shape_t) {
    unsafe {
        (*shape).type_ = SHAPE_RAINBOW;
        set_shape_name(shape, b"Rainbow");
        (*shape).height = 5;
        (*shape).width = 21;

        set_art_line(shape, 0, b"      _______      ");
        set_art_line(shape, 1, b"    /         \\    ");
        set_art_line(shape, 2, b"   /           \\   ");
        set_art_line(shape, 3, b"  /             \\  ");
        set_art_line(shape, 4, b" /               \\ ");
    }
}

// ============================================================
// Public C API: shape_*
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shape_manager_init() {
    // Allocate each shape once (singleton pattern)
    for i in 0..SHAPE_COUNT_USIZE {
        let p = unsafe { libc::malloc(std::mem::size_of::<shape_t>()) } as *mut shape_t;
        if p.is_null() {
            unsafe {
                libc::fprintf(
                    libc_stderr(),
                    FMT_ALLOC_FAIL.as_ptr() as *const c_char,
                );
                libc::exit(1);
            }
        }
        unsafe { SHAPES[i] = p };
    }

    unsafe {
        init_tree(SHAPES[SHAPE_TREE as usize]);
        init_tractor(SHAPES[SHAPE_TRACTOR as usize]);
        init_house(SHAPES[SHAPE_HOUSE as usize]);
        init_sun(SHAPES[SHAPE_SUN as usize]);
        init_cloud(SHAPES[SHAPE_CLOUD as usize]);
        init_flower(SHAPES[SHAPE_FLOWER as usize]);
        init_car(SHAPES[SHAPE_CAR as usize]);
        init_star(SHAPES[SHAPE_STAR as usize]);
        init_heart(SHAPES[SHAPE_HEART as usize]);
        init_rainbow(SHAPES[SHAPE_RAINBOW as usize]);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shape_manager_cleanup() {
    for i in 0..SHAPE_COUNT_USIZE {
        unsafe {
            libc::free(SHAPES[i] as *mut c_void);
            SHAPES[i] = ptr::null_mut();
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shape_get(type_: shape_type_t) -> *mut shape_t {
    if type_ < 0 || type_ >= SHAPE_COUNT {
        return ptr::null_mut();
    }
    unsafe { SHAPES[type_ as usize] }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shape_print(shape: *const shape_t) {
    if shape.is_null() {
        unsafe {
            libc::printf(FMT_NULL_SHAPE.as_ptr() as *const c_char);
        }
        return;
    }

    unsafe {
        libc::printf(
            FMT_NAME_COLON.as_ptr() as *const c_char,
            (*shape).name.as_ptr(),
        );
        let height = (*shape).height;
        for i in 0..height {
            libc::printf(
                FMT_S.as_ptr() as *const c_char,
                (*shape).art[i as usize].as_ptr(),
            );
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shape_equals(s1: *const shape_t, s2: *const shape_t) -> c_int {
    if s1 == s2 { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shape_type_name(type_: shape_type_t) -> *const c_char {
    let s: &[u8] = match type_ {
        x if x == SHAPE_TREE => b"Tree\0",
        x if x == SHAPE_TRACTOR => b"Tractor\0",
        x if x == SHAPE_HOUSE => b"House\0",
        x if x == SHAPE_SUN => b"Sun\0",
        x if x == SHAPE_CLOUD => b"Cloud\0",
        x if x == SHAPE_FLOWER => b"Flower\0",
        x if x == SHAPE_CAR => b"Car\0",
        x if x == SHAPE_STAR => b"Star\0",
        x if x == SHAPE_HEART => b"Heart\0",
        x if x == SHAPE_RAINBOW => b"Rainbow\0",
        _ => b"Unknown\0",
    };
    s.as_ptr() as *const c_char
}

// ============================================================
// Public C API: scene_*
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn scene_create(name: *const c_char) -> *mut scene_t {
    let scene = unsafe { libc::malloc(std::mem::size_of::<scene_t>()) } as *mut scene_t;
    if scene.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        if !name.is_null() {
            libc::strncpy(
                (*scene).name.as_mut_ptr(),
                name,
                MAX_SCENE_NAME - 1,
            );
            (*scene).name[MAX_SCENE_NAME - 1] = 0;
        } else {
            libc::strcpy(
                (*scene).name.as_mut_ptr(),
                DEFAULT_SCENE_NAME.as_ptr() as *const c_char,
            );
        }

        (*scene).shape_count = 0;
    }

    scene
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn scene_destroy(scene: *mut scene_t) {
    if !scene.is_null() {
        // Note: we don't free the singleton shapes
        unsafe { libc::free(scene as *mut c_void) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn scene_add_shape(
    scene: *mut scene_t,
    shape: *mut shape_t,
) -> c_int {
    if scene.is_null() || shape.is_null() {
        return -1;
    }

    unsafe {
        if (*scene).shape_count as usize >= MAX_SHAPES_IN_SCENE {
            libc::fprintf(
                libc_stderr(),
                FMT_ERR_SCENE_FULL.as_ptr() as *const c_char,
            );
            return -1;
        }

        let idx = (*scene).shape_count as usize;
        (*scene).shapes[idx] = shape;
        (*scene).shape_count += 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn scene_remove_shape(scene: *mut scene_t, index: c_int) -> c_int {
    if scene.is_null() {
        return -1;
    }
    let count = unsafe { (*scene).shape_count };
    if index < 0 || index >= count {
        return -1;
    }

    unsafe {
        let mut i = index;
        while i < count - 1 {
            (*scene).shapes[i as usize] = (*scene).shapes[(i + 1) as usize];
            i += 1;
        }
        (*scene).shape_count -= 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn scene_print(scene: *const scene_t) {
    if scene.is_null() {
        unsafe {
            libc::printf(FMT_NULL_SCENE.as_ptr() as *const c_char);
        }
        return;
    }

    unsafe {
        libc::printf(
            FMT_SCENE_HEADER.as_ptr() as *const c_char,
            (*scene).name.as_ptr(),
        );
        libc::printf(
            FMT_CONTAINS.as_ptr() as *const c_char,
            (*scene).shape_count,
        );

        let count = (*scene).shape_count;
        for i in 0..count {
            libc::printf(
                FMT_SHAPE_NUM.as_ptr() as *const c_char,
                i + 1,
            );
            shape_print((*scene).shapes[i as usize] as *const shape_t);
            libc::printf(FMT_NL.as_ptr() as *const c_char);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn scene_equals(s1: *const scene_t, s2: *const scene_t) -> c_int {
    if s1.is_null() || s2.is_null() {
        return 0;
    }

    unsafe {
        if (*s1).shape_count != (*s2).shape_count {
            return 0;
        }

        let mut matched = [0i32; MAX_SHAPES_IN_SCENE];
        let count = (*s1).shape_count;

        for i in 0..count {
            let mut found = 0;
            for j in 0..count {
                if matched[j as usize] == 0
                    && shape_equals(
                        (*s1).shapes[i as usize] as *const shape_t,
                        (*s2).shapes[j as usize] as *const shape_t,
                    ) != 0
                {
                    matched[j as usize] = 1;
                    found = 1;
                    break;
                }
            }
            if found == 0 {
                return 0;
            }
        }
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn scene_save(
    scene: *const scene_t,
    filename: *const c_char,
) -> c_int {
    if scene.is_null() || filename.is_null() {
        return -1;
    }

    let mode = b"w\0";
    let file = unsafe { libc::fopen(filename, mode.as_ptr() as *const c_char) };
    if file.is_null() {
        unsafe {
            libc::fprintf(
                libc_stderr(),
                FMT_ERR_OPEN_WRITE.as_ptr() as *const c_char,
                filename,
            );
        }
        return -1;
    }

    unsafe {
        libc::fprintf(
            file,
            FMT_FILE_NAME_NL.as_ptr() as *const c_char,
            (*scene).name.as_ptr(),
        );
        libc::fprintf(
            file,
            FMT_DECIMAL_NL.as_ptr() as *const c_char,
            (*scene).shape_count,
        );

        let count = (*scene).shape_count;
        for i in 0..count {
            let shape_ptr = (*scene).shapes[i as usize];
            libc::fprintf(
                file,
                FMT_DECIMAL_NL.as_ptr() as *const c_char,
                (*shape_ptr).type_,
            );
        }

        libc::fclose(file);
        libc::printf(
            FMT_SAVED_TO.as_ptr() as *const c_char,
            filename,
        );
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn scene_load(filename: *const c_char) -> *mut scene_t {
    if filename.is_null() {
        return ptr::null_mut();
    }

    let mode = b"r\0";
    let file = unsafe { libc::fopen(filename, mode.as_ptr() as *const c_char) };
    if file.is_null() {
        unsafe {
            libc::fprintf(
                libc_stderr(),
                FMT_ERR_OPEN_READ.as_ptr() as *const c_char,
                filename,
            );
        }
        return ptr::null_mut();
    }

    let mut name_buf: [c_char; MAX_SCENE_NAME] = [0; MAX_SCENE_NAME];

    unsafe {
        let r = libc::fgets(
            name_buf.as_mut_ptr(),
            MAX_SCENE_NAME as c_int,
            file,
        );
        if r.is_null() {
            libc::fclose(file);
            return ptr::null_mut();
        }

        // Remove newline: name[strcspn(name, "\n")] = 0
        let nl_pat = b"\n\0";
        let pos = libc::strcspn(
            name_buf.as_ptr(),
            nl_pat.as_ptr() as *const c_char,
        );
        name_buf[pos as usize] = 0;

        let scene = scene_create(name_buf.as_ptr());
        if scene.is_null() {
            libc::fclose(file);
            return ptr::null_mut();
        }

        let mut shape_count: c_int = 0;
        let fmt = b"%d\n\0";
        if libc::fscanf(file, fmt.as_ptr() as *const c_char, &mut shape_count) != 1 {
            scene_destroy(scene);
            libc::fclose(file);
            return ptr::null_mut();
        }

        for _i in 0..shape_count {
            let mut type_val: c_int = 0;
            if libc::fscanf(file, fmt.as_ptr() as *const c_char, &mut type_val) != 1 {
                scene_destroy(scene);
                libc::fclose(file);
                return ptr::null_mut();
            }

            let shape = shape_get(type_val as shape_type_t);
            if !shape.is_null() {
                scene_add_shape(scene, shape);
            }
        }

        libc::fclose(file);
        libc::printf(
            FMT_LOADED_FROM.as_ptr() as *const c_char,
            filename,
        );
        scene
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn scene_list_shapes(scene: *const scene_t) {
    if scene.is_null() {
        unsafe {
            libc::printf(FMT_NULL_SCENE.as_ptr() as *const c_char);
        }
        return;
    }

    unsafe {
        libc::printf(
            FMT_SCENE_NAME_LINE.as_ptr() as *const c_char,
            (*scene).name.as_ptr(),
        );
        libc::printf(
            FMT_SHAPES_COUNT.as_ptr() as *const c_char,
            (*scene).shape_count,
        );

        let count = (*scene).shape_count;
        for i in 0..count {
            let shape_ptr = (*scene).shapes[i as usize];
            libc::printf(
                FMT_LIST_ENTRY.as_ptr() as *const c_char,
                i + 1,
                (*shape_ptr).name.as_ptr(),
                shape_ptr as *const c_void,
            );
        }
    }
}

// ============================================================
// stderr access (libc crate exposes it as a function on some platforms)
// ============================================================

#[cfg(any(target_os = "linux", target_os = "android"))]
extern "C" {
    static mut stderr: *mut libc::FILE;
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn libc_stderr() -> *mut libc::FILE {
    unsafe { stderr }
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn libc_stderr() -> *mut libc::FILE {
    // Fallback for other platforms: use fdopen on fd 2
    // Cached so we don't open multiple times.
    use std::sync::OnceLock;
    static CACHED: OnceLock<usize> = OnceLock::new();
    let p = *CACHED.get_or_init(|| {
        let mode = b"w\0";
        unsafe {
            libc::fdopen(2, mode.as_ptr() as *const c_char) as usize
        }
    });
    p as *mut libc::FILE
}
