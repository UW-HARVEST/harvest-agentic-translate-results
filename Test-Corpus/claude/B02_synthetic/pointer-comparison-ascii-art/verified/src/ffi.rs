// ffi.rs - C-ABI compatible FFI layer.
//
// This module exposes `extern "C"` symbols that match the C library's public API
// (see c_src/include/scene.h and c_src/include/shape.h). The structs use the
// exact same memory layout as the C types so callers can read fields like
// `scene->shape_count` directly through the pointer.

use std::ffi::{c_char, c_int, CStr};
use std::os::raw::c_void;
use std::ptr;

// -----------------------------------------------------------------------------
// Constants matching the C #define values
// -----------------------------------------------------------------------------

pub const MAX_SHAPES_IN_SCENE: usize = 50;
pub const MAX_SCENE_NAME: usize = 64;
pub const MAX_SHAPE_WIDTH: usize = 80;
pub const MAX_SHAPE_HEIGHT: usize = 30;
pub const MAX_SHAPE_NAME: usize = 32;

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

// -----------------------------------------------------------------------------
// C-ABI compatible types
// -----------------------------------------------------------------------------

/// shape_t in C: { shape_type_t type; char name[32]; char art[30][80]; int width; int height; }
#[repr(C)]
pub struct shape_t {
    pub type_: c_int,
    pub name: [c_char; MAX_SHAPE_NAME],
    pub art: [[c_char; MAX_SHAPE_WIDTH]; MAX_SHAPE_HEIGHT],
    pub width: c_int,
    pub height: c_int,
}

/// scene_t in C: { char name[64]; shape_t* shapes[50]; int shape_count; }
#[repr(C)]
pub struct scene_t {
    pub name: [c_char; MAX_SCENE_NAME],
    pub shapes: [*mut shape_t; MAX_SHAPES_IN_SCENE],
    pub shape_count: c_int,
}

// -----------------------------------------------------------------------------
// Singleton storage for shape_manager (mirrors C's static shapes[SHAPE_COUNT])
// -----------------------------------------------------------------------------

static mut SHAPES_PTRS: [*mut shape_t; SHAPE_COUNT as usize] =
    [ptr::null_mut(); SHAPE_COUNT as usize];

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

/// Copy a string into a fixed-size C buffer, null-padded. Truncates at N-1 bytes
/// and writes a null terminator. Mimics strncpy + explicit null-terminate.
fn cstr_copy_strncpy(dest: &mut [c_char], src: &[u8]) {
    let n = dest.len();
    if n == 0 {
        return;
    }
    // Zero out
    for c in dest.iter_mut() {
        *c = 0;
    }
    let copy_len = std::cmp::min(src.len(), n - 1);
    for i in 0..copy_len {
        dest[i] = src[i] as c_char;
    }
    // Null terminator already 0 from zeroing
}

/// strcpy: copy null-terminated string from src into dest. dest must be large enough.
/// Stops at the first 0 byte in src (and copies it).
fn cstr_strcpy(dest: &mut [c_char], src: &[u8]) {
    // Zero out dest first to mimic that strcpy in init_*() writes into uninitialized
    // memory, but the rest stays uninitialized in C. To get reproducible behavior,
    // we zero everything beyond the copied length so two implementations compare equal.
    for c in dest.iter_mut() {
        *c = 0;
    }
    for (i, &b) in src.iter().enumerate() {
        if i >= dest.len() {
            break;
        }
        dest[i] = b as c_char;
        if b == 0 {
            return;
        }
    }
}

// -----------------------------------------------------------------------------
// shape_manager / shape API
// -----------------------------------------------------------------------------

unsafe fn init_tree(s: &mut shape_t) {
    s.type_ = SHAPE_TREE;
    cstr_strcpy(&mut s.name, b"Tree\0");
    s.height = 7;
    s.width = 11;
    cstr_strcpy(&mut s.art[0], b"    /\\    \0");
    cstr_strcpy(&mut s.art[1], b"   /  \\   \0");
    cstr_strcpy(&mut s.art[2], b"  /____\\  \0");
    cstr_strcpy(&mut s.art[3], b"  /    \\  \0");
    cstr_strcpy(&mut s.art[4], b" /______\\ \0");
    cstr_strcpy(&mut s.art[5], b"    ||    \0");
    cstr_strcpy(&mut s.art[6], b"    ||    \0");
}

unsafe fn init_tractor(s: &mut shape_t) {
    s.type_ = SHAPE_TRACTOR;
    cstr_strcpy(&mut s.name, b"Tractor\0");
    s.height = 6;
    s.width = 20;
    cstr_strcpy(&mut s.art[0], b"      ________     \0");
    cstr_strcpy(&mut s.art[1], b"     |        |___ \0");
    cstr_strcpy(&mut s.art[2], b"     |  []  []|   |\0");
    cstr_strcpy(&mut s.art[3], b"  ___|________|___|\0");
    cstr_strcpy(&mut s.art[4], b" /  o        o   \\\0");
    cstr_strcpy(&mut s.art[5], b"|___|        |___| \0");
}

unsafe fn init_house(s: &mut shape_t) {
    s.type_ = SHAPE_HOUSE;
    cstr_strcpy(&mut s.name, b"House\0");
    s.height = 7;
    s.width = 13;
    cstr_strcpy(&mut s.art[0], b"     /\\     \0");
    cstr_strcpy(&mut s.art[1], b"    /  \\    \0");
    cstr_strcpy(&mut s.art[2], b"   /____\\   \0");
    cstr_strcpy(&mut s.art[3], b"   |    |   \0");
    cstr_strcpy(&mut s.art[4], b"   | [] |   \0");
    cstr_strcpy(&mut s.art[5], b"   |    |   \0");
    cstr_strcpy(&mut s.art[6], b"   |____|   \0");
}

unsafe fn init_sun(s: &mut shape_t) {
    s.type_ = SHAPE_SUN;
    cstr_strcpy(&mut s.name, b"Sun\0");
    s.height = 7;
    s.width = 11;
    cstr_strcpy(&mut s.art[0], b"  \\  |  / \0");
    cstr_strcpy(&mut s.art[1], b"   \\ | /  \0");
    cstr_strcpy(&mut s.art[2], b"--- (@) ---\0");
    cstr_strcpy(&mut s.art[3], b"   / | \\  \0");
    cstr_strcpy(&mut s.art[4], b"  /  |  \\ \0");
    cstr_strcpy(&mut s.art[5], b"          \0");
    cstr_strcpy(&mut s.art[6], b"          \0");
}

unsafe fn init_cloud(s: &mut shape_t) {
    s.type_ = SHAPE_CLOUD;
    cstr_strcpy(&mut s.name, b"Cloud\0");
    s.height = 4;
    s.width = 16;
    cstr_strcpy(&mut s.art[0], b"   _____       \0");
    cstr_strcpy(&mut s.art[1], b"  /     \\_    \0");
    cstr_strcpy(&mut s.art[2], b" /  ___  _\\  \0");
    cstr_strcpy(&mut s.art[3], b"(__/   \\_)   \0");
}

unsafe fn init_flower(s: &mut shape_t) {
    s.type_ = SHAPE_FLOWER;
    cstr_strcpy(&mut s.name, b"Flower\0");
    s.height = 7;
    s.width = 9;
    cstr_strcpy(&mut s.art[0], b"  \\|/  \0");
    cstr_strcpy(&mut s.art[1], b" -(@)- \0");
    cstr_strcpy(&mut s.art[2], b"  /|\\  \0");
    cstr_strcpy(&mut s.art[3], b"   |   \0");
    cstr_strcpy(&mut s.art[4], b"   |   \0");
    cstr_strcpy(&mut s.art[5], b"  / \\  \0");
    cstr_strcpy(&mut s.art[6], b" /   \\ \0");
}

unsafe fn init_car(s: &mut shape_t) {
    s.type_ = SHAPE_CAR;
    cstr_strcpy(&mut s.name, b"Car\0");
    s.height = 4;
    s.width = 16;
    cstr_strcpy(&mut s.art[0], b"  ____       \0");
    cstr_strcpy(&mut s.art[1], b" /|_||_\\____ \0");
    cstr_strcpy(&mut s.art[2], b"( o     o  ) \0");
    cstr_strcpy(&mut s.art[3], b" -----------  \0");
}

unsafe fn init_star(s: &mut shape_t) {
    s.type_ = SHAPE_STAR;
    cstr_strcpy(&mut s.name, b"Star\0");
    s.height = 5;
    s.width = 9;
    cstr_strcpy(&mut s.art[0], b"    *    \0");
    cstr_strcpy(&mut s.art[1], b"   ***   \0");
    cstr_strcpy(&mut s.art[2], b"  *****  \0");
    cstr_strcpy(&mut s.art[3], b" ******* \0");
    cstr_strcpy(&mut s.art[4], b"*********\0");
}

unsafe fn init_heart(s: &mut shape_t) {
    s.type_ = SHAPE_HEART;
    cstr_strcpy(&mut s.name, b"Heart\0");
    s.height = 6;
    s.width = 11;
    cstr_strcpy(&mut s.art[0], b" *** ***  \0");
    cstr_strcpy(&mut s.art[1], b"*********  \0");
    cstr_strcpy(&mut s.art[2], b"*********  \0");
    cstr_strcpy(&mut s.art[3], b" ******* \0");
    cstr_strcpy(&mut s.art[4], b"  *****  \0");
    cstr_strcpy(&mut s.art[5], b"   ***   \0");
}

unsafe fn init_rainbow(s: &mut shape_t) {
    s.type_ = SHAPE_RAINBOW;
    cstr_strcpy(&mut s.name, b"Rainbow\0");
    s.height = 5;
    s.width = 21;
    cstr_strcpy(&mut s.art[0], b"      _______      \0");
    cstr_strcpy(&mut s.art[1], b"    /         \\    \0");
    cstr_strcpy(&mut s.art[2], b"   /           \\   \0");
    cstr_strcpy(&mut s.art[3], b"  /             \\  \0");
    cstr_strcpy(&mut s.art[4], b" /               \\ \0");
}

#[no_mangle]
pub unsafe extern "C" fn shape_manager_init() {
    // Allocate and zero each shape (calloc-ish, mimics 'malloc' but ensures
    // deterministic initial bytes since C never explicitly clears art[][] beyond
    // the strcpy length).
    for i in 0..SHAPE_COUNT as usize {
        if !SHAPES_PTRS[i].is_null() {
            // Already initialised; free first to avoid leak
            libc_free(SHAPES_PTRS[i] as *mut c_void);
        }
        // Allocate and zero — using libc::malloc + memset, but use Box for simplicity.
        // To keep ABI-compat with C's free() we use libc::malloc.
        let p = libc_malloc(std::mem::size_of::<shape_t>()) as *mut shape_t;
        if p.is_null() {
            // mimic exit(1) on error
            std::process::exit(1);
        }
        // Zero out so we have deterministic memory for byte-comparable tests.
        std::ptr::write_bytes(p as *mut u8, 0, std::mem::size_of::<shape_t>());
        SHAPES_PTRS[i] = p;
    }

    init_tree(&mut *SHAPES_PTRS[SHAPE_TREE as usize]);
    init_tractor(&mut *SHAPES_PTRS[SHAPE_TRACTOR as usize]);
    init_house(&mut *SHAPES_PTRS[SHAPE_HOUSE as usize]);
    init_sun(&mut *SHAPES_PTRS[SHAPE_SUN as usize]);
    init_cloud(&mut *SHAPES_PTRS[SHAPE_CLOUD as usize]);
    init_flower(&mut *SHAPES_PTRS[SHAPE_FLOWER as usize]);
    init_car(&mut *SHAPES_PTRS[SHAPE_CAR as usize]);
    init_star(&mut *SHAPES_PTRS[SHAPE_STAR as usize]);
    init_heart(&mut *SHAPES_PTRS[SHAPE_HEART as usize]);
    init_rainbow(&mut *SHAPES_PTRS[SHAPE_RAINBOW as usize]);
}

#[no_mangle]
pub unsafe extern "C" fn shape_manager_cleanup() {
    for i in 0..SHAPE_COUNT as usize {
        if !SHAPES_PTRS[i].is_null() {
            libc_free(SHAPES_PTRS[i] as *mut c_void);
            SHAPES_PTRS[i] = ptr::null_mut();
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn shape_get(type_: c_int) -> *mut shape_t {
    if type_ < 0 || type_ >= SHAPE_COUNT {
        return ptr::null_mut();
    }
    SHAPES_PTRS[type_ as usize]
}

#[no_mangle]
pub unsafe extern "C" fn shape_print(shape: *const shape_t) {
    if shape.is_null() {
        // printf("(null shape)\n")
        c_puts("(null shape)\n\0".as_ptr() as *const c_char);
        return;
    }
    // printf("%s:\n", shape->name);
    c_printf_named((*shape).name.as_ptr(), b":\n\0");
    for i in 0..(*shape).height as usize {
        c_printf_line((*shape).art[i].as_ptr());
    }
}

#[no_mangle]
pub unsafe extern "C" fn shape_equals(s1: *const shape_t, s2: *const shape_t) -> c_int {
    if s1 == s2 {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn shape_type_name(type_: c_int) -> *const c_char {
    static TREE: &[u8] = b"Tree\0";
    static TRACTOR: &[u8] = b"Tractor\0";
    static HOUSE: &[u8] = b"House\0";
    static SUN: &[u8] = b"Sun\0";
    static CLOUD: &[u8] = b"Cloud\0";
    static FLOWER: &[u8] = b"Flower\0";
    static CAR: &[u8] = b"Car\0";
    static STAR: &[u8] = b"Star\0";
    static HEART: &[u8] = b"Heart\0";
    static RAINBOW: &[u8] = b"Rainbow\0";
    static UNKNOWN: &[u8] = b"Unknown\0";
    let s: &[u8] = match type_ {
        SHAPE_TREE => TREE,
        SHAPE_TRACTOR => TRACTOR,
        SHAPE_HOUSE => HOUSE,
        SHAPE_SUN => SUN,
        SHAPE_CLOUD => CLOUD,
        SHAPE_FLOWER => FLOWER,
        SHAPE_CAR => CAR,
        SHAPE_STAR => STAR,
        SHAPE_HEART => HEART,
        SHAPE_RAINBOW => RAINBOW,
        _ => UNKNOWN,
    };
    s.as_ptr() as *const c_char
}

// -----------------------------------------------------------------------------
// scene API
// -----------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn scene_create(name: *const c_char) -> *mut scene_t {
    let p = libc_malloc(std::mem::size_of::<scene_t>()) as *mut scene_t;
    if p.is_null() {
        return ptr::null_mut();
    }
    // Zero the whole struct so the unused name-buffer bytes are deterministic.
    std::ptr::write_bytes(p as *mut u8, 0, std::mem::size_of::<scene_t>());
    let s = &mut *p;

    if !name.is_null() {
        // strncpy(s->name, name, MAX_SCENE_NAME-1); s->name[MAX_SCENE_NAME-1] = 0;
        let slice = CStr::from_ptr(name).to_bytes();
        cstr_copy_strncpy(&mut s.name, slice);
    } else {
        cstr_strcpy(&mut s.name, b"Untitled Scene\0");
    }

    s.shape_count = 0;
    p
}

#[no_mangle]
pub unsafe extern "C" fn scene_destroy(scene: *mut scene_t) {
    if !scene.is_null() {
        libc_free(scene as *mut c_void);
    }
}

#[no_mangle]
pub unsafe extern "C" fn scene_add_shape(scene: *mut scene_t, shape: *mut shape_t) -> c_int {
    if scene.is_null() || shape.is_null() {
        return -1;
    }
    let s = &mut *scene;
    if s.shape_count as usize >= MAX_SHAPES_IN_SCENE {
        c_fputs_stderr(b"Error: Scene is full\n\0");
        return -1;
    }
    s.shapes[s.shape_count as usize] = shape;
    s.shape_count += 1;
    0
}

#[no_mangle]
pub unsafe extern "C" fn scene_remove_shape(scene: *mut scene_t, index: c_int) -> c_int {
    if scene.is_null() {
        return -1;
    }
    let s = &mut *scene;
    if index < 0 || index >= s.shape_count {
        return -1;
    }
    let count = s.shape_count;
    let mut i = index;
    while i < count - 1 {
        s.shapes[i as usize] = s.shapes[(i + 1) as usize];
        i += 1;
    }
    s.shape_count -= 1;
    0
}

#[no_mangle]
pub unsafe extern "C" fn scene_print(scene: *const scene_t) {
    if scene.is_null() {
        c_puts("(null scene)\n\0".as_ptr() as *const c_char);
        return;
    }
    let s = &*scene;
    c_printf_scene_header(s.name.as_ptr());
    c_printf_shape_count(s.shape_count);
    for i in 0..s.shape_count as usize {
        c_printf_shape_n((i + 1) as c_int);
        shape_print(s.shapes[i] as *const shape_t);
        c_putchar(b'\n' as c_int);
    }
}

#[no_mangle]
pub unsafe extern "C" fn scene_equals(s1: *const scene_t, s2: *const scene_t) -> c_int {
    if s1.is_null() || s2.is_null() {
        return 0;
    }
    let a = &*s1;
    let b = &*s2;
    if a.shape_count != b.shape_count {
        return 0;
    }
    let mut matched = [false; MAX_SHAPES_IN_SCENE];
    for i in 0..a.shape_count as usize {
        let mut found = false;
        for j in 0..b.shape_count as usize {
            if !matched[j] && shape_equals(a.shapes[i], b.shapes[j]) != 0 {
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

#[no_mangle]
pub unsafe extern "C" fn scene_save(scene: *const scene_t, filename: *const c_char) -> c_int {
    if scene.is_null() || filename.is_null() {
        return -1;
    }
    // Use C's fopen/fprintf so output formatting matches exactly.
    let mode = b"w\0".as_ptr() as *const c_char;
    let f = c_fopen(filename, mode);
    if f.is_null() {
        c_fprintf_open_err_write(filename);
        return -1;
    }
    let s = &*scene;
    // fprintf(file, "%s\n", scene->name);
    c_fprintf_named_line(f, s.name.as_ptr());
    // fprintf(file, "%d\n", scene->shape_count);
    c_fprintf_int_line(f, s.shape_count);
    for i in 0..s.shape_count as usize {
        let sp = s.shapes[i];
        if !sp.is_null() {
            c_fprintf_int_line(f, (*sp).type_);
        }
    }
    c_fclose(f);
    c_printf_saved(filename);
    0
}

#[no_mangle]
pub unsafe extern "C" fn scene_load(filename: *const c_char) -> *mut scene_t {
    if filename.is_null() {
        return ptr::null_mut();
    }
    let mode = b"r\0".as_ptr() as *const c_char;
    let f = c_fopen(filename, mode);
    if f.is_null() {
        c_fprintf_open_err_read(filename);
        return ptr::null_mut();
    }

    let mut name: [c_char; MAX_SCENE_NAME] = [0; MAX_SCENE_NAME];
    if c_fgets(name.as_mut_ptr(), MAX_SCENE_NAME as c_int, f).is_null() {
        c_fclose(f);
        return ptr::null_mut();
    }

    // name[strcspn(name, "\n")] = 0;
    let cs = CStr::from_ptr(name.as_ptr()).to_bytes();
    if let Some(pos) = cs.iter().position(|&b| b == b'\n') {
        name[pos] = 0;
    }

    let scene = scene_create(name.as_ptr());
    if scene.is_null() {
        c_fclose(f);
        return ptr::null_mut();
    }

    let mut shape_count: c_int = 0;
    if c_fscanf_int(f, &mut shape_count) != 1 {
        scene_destroy(scene);
        c_fclose(f);
        return ptr::null_mut();
    }

    for _ in 0..shape_count {
        let mut t: c_int = 0;
        if c_fscanf_int(f, &mut t) != 1 {
            scene_destroy(scene);
            c_fclose(f);
            return ptr::null_mut();
        }
        let sp = shape_get(t);
        if !sp.is_null() {
            scene_add_shape(scene, sp);
        }
    }

    c_fclose(f);
    c_printf_loaded(filename);
    scene
}

#[no_mangle]
pub unsafe extern "C" fn scene_list_shapes(scene: *const scene_t) {
    if scene.is_null() {
        c_puts("(null scene)\n\0".as_ptr() as *const c_char);
        return;
    }
    let s = &*scene;
    c_printf_scene_label(s.name.as_ptr());
    c_printf_shapes_count(s.shape_count);
    for i in 0..s.shape_count as usize {
        let sp = s.shapes[i];
        if !sp.is_null() {
            c_printf_listed_shape((i + 1) as c_int, (*sp).name.as_ptr(), sp as *const c_void);
        }
    }
}

// -----------------------------------------------------------------------------
// libc bindings — used to ensure printf/fprintf output matches C exactly,
// and that allocation comes from the same allocator (libc malloc/free) as the
// caller code in C uses with scene_destroy.
// -----------------------------------------------------------------------------

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut c_void, fmt: *const c_char, ...) -> c_int;
    fn fputs(s: *const c_char, stream: *mut c_void) -> c_int;
    #[allow(dead_code)]
    fn puts(s: *const c_char) -> c_int;
    fn putchar(c: c_int) -> c_int;
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(stream: *mut c_void) -> c_int;
    fn fgets(buf: *mut c_char, n: c_int, stream: *mut c_void) -> *mut c_char;
    fn fscanf(stream: *mut c_void, fmt: *const c_char, ...) -> c_int;
    static stderr: *mut c_void;
}

unsafe fn libc_malloc(size: usize) -> *mut c_void {
    malloc(size)
}

unsafe fn libc_free(p: *mut c_void) {
    free(p)
}

unsafe fn c_printf_named(name: *const c_char, suffix: &[u8]) {
    // printf("%s:\n", name) but suffix can be ":\n" or ".\n" — we just feed exactly :\n
    // The caller passes ":\n\0" in the suffix bytes.
    // Build a format string like "%s<suffix>"
    let mut fmt = Vec::with_capacity(2 + suffix.len());
    fmt.extend_from_slice(b"%s");
    fmt.extend_from_slice(suffix);
    // Already null-terminated from b":\n\0"
    printf(fmt.as_ptr() as *const c_char, name);
}

unsafe fn c_printf_line(line: *const c_char) {
    // printf("%s\n", line)
    printf(b"%s\n\0".as_ptr() as *const c_char, line);
}

unsafe fn c_puts(s: *const c_char) {
    // C's puts() appends a newline. We use fputs(stdout) for control —
    // but original code uses printf("(null shape)\n"). We simulate via fputs
    // with the included \n.
    fputs(s, stdout_ptr());
}

unsafe fn stdout_ptr() -> *mut c_void {
    extern "C" {
        static stdout: *mut c_void;
    }
    stdout
}

unsafe fn c_fputs_stderr(s: &[u8]) {
    fputs(s.as_ptr() as *const c_char, stderr);
}

unsafe fn c_putchar(c: c_int) {
    putchar(c);
}

unsafe fn c_fopen(path: *const c_char, mode: *const c_char) -> *mut c_void {
    fopen(path, mode)
}

unsafe fn c_fclose(f: *mut c_void) {
    fclose(f);
}

unsafe fn c_fgets(buf: *mut c_char, n: c_int, f: *mut c_void) -> *mut c_char {
    fgets(buf, n, f)
}

unsafe fn c_fscanf_int(f: *mut c_void, out: *mut c_int) -> c_int {
    fscanf(f, b"%d\n\0".as_ptr() as *const c_char, out)
}

unsafe fn c_fprintf_named_line(f: *mut c_void, name: *const c_char) {
    fprintf(f, b"%s\n\0".as_ptr() as *const c_char, name);
}

unsafe fn c_fprintf_int_line(f: *mut c_void, n: c_int) {
    fprintf(f, b"%d\n\0".as_ptr() as *const c_char, n);
}

unsafe fn c_fprintf_open_err_write(filename: *const c_char) {
    fprintf(
        stderr,
        b"Error: Could not open file '%s' for writing\n\0".as_ptr() as *const c_char,
        filename,
    );
}

unsafe fn c_fprintf_open_err_read(filename: *const c_char) {
    fprintf(
        stderr,
        b"Error: Could not open file '%s' for reading\n\0".as_ptr() as *const c_char,
        filename,
    );
}

unsafe fn c_printf_scene_header(name: *const c_char) {
    printf(b"\n=== Scene: %s ===\n\0".as_ptr() as *const c_char, name);
}

unsafe fn c_printf_shape_count(n: c_int) {
    printf(b"Contains %d shape(s)\n\n\0".as_ptr() as *const c_char, n);
}

unsafe fn c_printf_shape_n(n: c_int) {
    printf(b"Shape #%d:\n\0".as_ptr() as *const c_char, n);
}

unsafe fn c_printf_scene_label(name: *const c_char) {
    printf(b"\nScene: %s\n\0".as_ptr() as *const c_char, name);
}

unsafe fn c_printf_shapes_count(n: c_int) {
    printf(b"Shapes (%d):\n\0".as_ptr() as *const c_char, n);
}

unsafe fn c_printf_listed_shape(idx: c_int, name: *const c_char, ptr: *const c_void) {
    printf(
        b"  %d. %s (ptr: %p)\n\0".as_ptr() as *const c_char,
        idx,
        name,
        ptr,
    );
}

unsafe fn c_printf_saved(filename: *const c_char) {
    printf(b"Scene saved to '%s'\n\0".as_ptr() as *const c_char, filename);
}

unsafe fn c_printf_loaded(filename: *const c_char) {
    printf(b"Scene loaded from '%s'\n\0".as_ptr() as *const c_char, filename);
}
