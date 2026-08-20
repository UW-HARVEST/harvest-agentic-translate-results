//! C-ABI translation of `c_src/src/shape.c` (and `shape.h`).
//!
//! This module reproduces the C implementation exactly at the ABI level: the
//! same struct layout, the same singleton `malloc` pattern, the same libc stdio
//! calls (so that the produced byte stream, including `%p` formatting and
//! stream buffering, is identical) and the same return values.
#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int};
use std::ptr::{self, addr_of_mut};

use super::cstdio::{c_stderr};

// ---------------------------------------------------------------------------
// shape.h
// ---------------------------------------------------------------------------

pub const MAX_SHAPE_WIDTH: usize = 80;
pub const MAX_SHAPE_HEIGHT: usize = 30;
pub const MAX_SHAPE_NAME: usize = 32;

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

/// `typedef struct { shape_type_t type; char name[32]; char art[30][80];
///                   int width; int height; } shape_t;`
#[repr(C)]
pub struct shape_t {
    pub type_: c_int,
    pub name: [c_char; MAX_SHAPE_NAME],
    pub art: [[c_char; MAX_SHAPE_WIDTH]; MAX_SHAPE_HEIGHT],
    pub width: c_int,
    pub height: c_int,
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// `&shape->name[0]` without ever forming a Rust reference to (possibly
/// uninitialised) `malloc`ed memory.
#[inline]
pub(crate) unsafe fn shape_name_ptr(shape: *mut shape_t) -> *mut c_char {
    addr_of_mut!((*shape).name) as *mut c_char
}

/// `&shape->art[row][0]`
#[inline]
pub(crate) unsafe fn shape_art_row(shape: *mut shape_t, row: usize) -> *mut c_char {
    (addr_of_mut!((*shape).art) as *mut c_char).add(row * MAX_SHAPE_WIDTH)
}

/// `strcpy(dst, "literal")` for a NUL-free literal.
#[inline]
pub(crate) unsafe fn strcpy_lit(dst: *mut c_char, src: &[u8]) {
    ptr::copy_nonoverlapping(src.as_ptr() as *const c_char, dst, src.len());
    *dst.add(src.len()) = 0;
}

// ---------------------------------------------------------------------------
// static shape_t *shapes[SHAPE_COUNT] = {NULL};
// ---------------------------------------------------------------------------

static mut SHAPES: [*mut shape_t; SHAPE_COUNT as usize] = [ptr::null_mut(); SHAPE_COUNT as usize];

#[inline]
unsafe fn shapes_slot(i: usize) -> *mut *mut shape_t {
    (addr_of_mut!(SHAPES) as *mut *mut shape_t).add(i)
}

unsafe fn init_tree(shape: *mut shape_t) {
    (*shape).type_ = SHAPE_TREE;
    strcpy_lit(shape_name_ptr(shape), b"Tree");
    (*shape).height = 7;
    (*shape).width = 11;
    strcpy_lit(shape_art_row(shape, 0), b"    /\\    ");
    strcpy_lit(shape_art_row(shape, 1), b"   /  \\   ");
    strcpy_lit(shape_art_row(shape, 2), b"  /____\\  ");
    strcpy_lit(shape_art_row(shape, 3), b"  /    \\  ");
    strcpy_lit(shape_art_row(shape, 4), b" /______\\ ");
    strcpy_lit(shape_art_row(shape, 5), b"    ||    ");
    strcpy_lit(shape_art_row(shape, 6), b"    ||    ");
}

unsafe fn init_tractor(shape: *mut shape_t) {
    (*shape).type_ = SHAPE_TRACTOR;
    strcpy_lit(shape_name_ptr(shape), b"Tractor");
    (*shape).height = 6;
    (*shape).width = 20;
    strcpy_lit(shape_art_row(shape, 0), b"      ________     ");
    strcpy_lit(shape_art_row(shape, 1), b"     |        |___ ");
    strcpy_lit(shape_art_row(shape, 2), b"     |  []  []|   |");
    strcpy_lit(shape_art_row(shape, 3), b"  ___|________|___|");
    strcpy_lit(shape_art_row(shape, 4), b" /  o        o   \\");
    strcpy_lit(shape_art_row(shape, 5), b"|___|        |___| ");
}

unsafe fn init_house(shape: *mut shape_t) {
    (*shape).type_ = SHAPE_HOUSE;
    strcpy_lit(shape_name_ptr(shape), b"House");
    (*shape).height = 7;
    (*shape).width = 13;
    strcpy_lit(shape_art_row(shape, 0), b"     /\\     ");
    strcpy_lit(shape_art_row(shape, 1), b"    /  \\    ");
    strcpy_lit(shape_art_row(shape, 2), b"   /____\\   ");
    strcpy_lit(shape_art_row(shape, 3), b"   |    |   ");
    strcpy_lit(shape_art_row(shape, 4), b"   | [] |   ");
    strcpy_lit(shape_art_row(shape, 5), b"   |    |   ");
    strcpy_lit(shape_art_row(shape, 6), b"   |____|   ");
}

unsafe fn init_sun(shape: *mut shape_t) {
    (*shape).type_ = SHAPE_SUN;
    strcpy_lit(shape_name_ptr(shape), b"Sun");
    (*shape).height = 7;
    (*shape).width = 11;
    strcpy_lit(shape_art_row(shape, 0), b"  \\  |  / ");
    strcpy_lit(shape_art_row(shape, 1), b"   \\ | /  ");
    strcpy_lit(shape_art_row(shape, 2), b"--- (@) ---");
    strcpy_lit(shape_art_row(shape, 3), b"   / | \\  ");
    strcpy_lit(shape_art_row(shape, 4), b"  /  |  \\ ");
    strcpy_lit(shape_art_row(shape, 5), b"          ");
    strcpy_lit(shape_art_row(shape, 6), b"          ");
}

unsafe fn init_cloud(shape: *mut shape_t) {
    (*shape).type_ = SHAPE_CLOUD;
    strcpy_lit(shape_name_ptr(shape), b"Cloud");
    (*shape).height = 4;
    (*shape).width = 16;
    strcpy_lit(shape_art_row(shape, 0), b"   _____       ");
    strcpy_lit(shape_art_row(shape, 1), b"  /     \\_    ");
    strcpy_lit(shape_art_row(shape, 2), b" /  ___  _\\  ");
    strcpy_lit(shape_art_row(shape, 3), b"(__/   \\_)   ");
}

unsafe fn init_flower(shape: *mut shape_t) {
    (*shape).type_ = SHAPE_FLOWER;
    strcpy_lit(shape_name_ptr(shape), b"Flower");
    (*shape).height = 7;
    (*shape).width = 9;
    strcpy_lit(shape_art_row(shape, 0), b"  \\|/  ");
    strcpy_lit(shape_art_row(shape, 1), b" -(@)- ");
    strcpy_lit(shape_art_row(shape, 2), b"  /|\\  ");
    strcpy_lit(shape_art_row(shape, 3), b"   |   ");
    strcpy_lit(shape_art_row(shape, 4), b"   |   ");
    strcpy_lit(shape_art_row(shape, 5), b"  / \\  ");
    strcpy_lit(shape_art_row(shape, 6), b" /   \\ ");
}

unsafe fn init_car(shape: *mut shape_t) {
    (*shape).type_ = SHAPE_CAR;
    strcpy_lit(shape_name_ptr(shape), b"Car");
    (*shape).height = 4;
    (*shape).width = 16;
    strcpy_lit(shape_art_row(shape, 0), b"  ____       ");
    strcpy_lit(shape_art_row(shape, 1), b" /|_||_\\____ ");
    strcpy_lit(shape_art_row(shape, 2), b"( o     o  ) ");
    strcpy_lit(shape_art_row(shape, 3), b" -----------  ");
}

unsafe fn init_star(shape: *mut shape_t) {
    (*shape).type_ = SHAPE_STAR;
    strcpy_lit(shape_name_ptr(shape), b"Star");
    (*shape).height = 5;
    (*shape).width = 9;
    strcpy_lit(shape_art_row(shape, 0), b"    *    ");
    strcpy_lit(shape_art_row(shape, 1), b"   ***   ");
    strcpy_lit(shape_art_row(shape, 2), b"  *****  ");
    strcpy_lit(shape_art_row(shape, 3), b" ******* ");
    strcpy_lit(shape_art_row(shape, 4), b"*********");
}

unsafe fn init_heart(shape: *mut shape_t) {
    (*shape).type_ = SHAPE_HEART;
    strcpy_lit(shape_name_ptr(shape), b"Heart");
    (*shape).height = 6;
    (*shape).width = 11;
    strcpy_lit(shape_art_row(shape, 0), b" *** ***  ");
    strcpy_lit(shape_art_row(shape, 1), b"*********  ");
    strcpy_lit(shape_art_row(shape, 2), b"*********  ");
    strcpy_lit(shape_art_row(shape, 3), b" ******* ");
    strcpy_lit(shape_art_row(shape, 4), b"  *****  ");
    strcpy_lit(shape_art_row(shape, 5), b"   ***   ");
}

unsafe fn init_rainbow(shape: *mut shape_t) {
    (*shape).type_ = SHAPE_RAINBOW;
    strcpy_lit(shape_name_ptr(shape), b"Rainbow");
    (*shape).height = 5;
    (*shape).width = 21;
    strcpy_lit(shape_art_row(shape, 0), b"      _______      ");
    strcpy_lit(shape_art_row(shape, 1), b"    /         \\    ");
    strcpy_lit(shape_art_row(shape, 2), b"   /           \\   ");
    strcpy_lit(shape_art_row(shape, 3), b"  /             \\  ");
    strcpy_lit(shape_art_row(shape, 4), b" /               \\ ");
}

// ---------------------------------------------------------------------------
// public API
// ---------------------------------------------------------------------------

/// `void shape_manager_init(void)`
#[no_mangle]
pub unsafe extern "C" fn shape_manager_init() {
    // Allocate each shape once (singleton pattern)
    let mut i: c_int = 0;
    while i < SHAPE_COUNT {
        let p = libc::malloc(std::mem::size_of::<shape_t>()) as *mut shape_t;
        *shapes_slot(i as usize) = p;
        if p.is_null() {
            libc::fprintf(
                c_stderr(),
                c"Error: Failed to allocate shape\n".as_ptr(),
            );
            libc::exit(1);
        }
        i += 1;
    }

    // Initialize each shape
    init_tree(*shapes_slot(SHAPE_TREE as usize));
    init_tractor(*shapes_slot(SHAPE_TRACTOR as usize));
    init_house(*shapes_slot(SHAPE_HOUSE as usize));
    init_sun(*shapes_slot(SHAPE_SUN as usize));
    init_cloud(*shapes_slot(SHAPE_CLOUD as usize));
    init_flower(*shapes_slot(SHAPE_FLOWER as usize));
    init_car(*shapes_slot(SHAPE_CAR as usize));
    init_star(*shapes_slot(SHAPE_STAR as usize));
    init_heart(*shapes_slot(SHAPE_HEART as usize));
    init_rainbow(*shapes_slot(SHAPE_RAINBOW as usize));
}

/// `void shape_manager_cleanup(void)`
#[no_mangle]
pub unsafe extern "C" fn shape_manager_cleanup() {
    let mut i: c_int = 0;
    while i < SHAPE_COUNT {
        libc::free(*shapes_slot(i as usize) as *mut libc::c_void);
        *shapes_slot(i as usize) = ptr::null_mut();
        i += 1;
    }
}

/// `shape_t* shape_get(shape_type_t type)`
#[no_mangle]
pub unsafe extern "C" fn shape_get(type_: c_int) -> *mut shape_t {
    if type_ < 0 || type_ >= SHAPE_COUNT {
        return ptr::null_mut();
    }
    *shapes_slot(type_ as usize)
}

/// `void shape_print(const shape_t *shape)`
#[no_mangle]
pub unsafe extern "C" fn shape_print(shape: *const shape_t) {
    if shape.is_null() {
        libc::printf(c"(null shape)\n".as_ptr());
        return;
    }

    let shape = shape as *mut shape_t;
    libc::printf(c"%s:\n".as_ptr(), shape_name_ptr(shape));
    let mut i: c_int = 0;
    while i < (*shape).height {
        libc::printf(c"%s\n".as_ptr(), shape_art_row(shape, i as usize));
        i += 1;
    }
}

/// `int shape_equals(const shape_t *s1, const shape_t *s2)`
#[no_mangle]
pub unsafe extern "C" fn shape_equals(s1: *const shape_t, s2: *const shape_t) -> c_int {
    if s1 == s2 {
        1
    } else {
        0
    }
}

/// `const char* shape_type_name(shape_type_t type)`
#[no_mangle]
pub unsafe extern "C" fn shape_type_name(type_: c_int) -> *const c_char {
    match type_ {
        SHAPE_TREE => c"Tree".as_ptr(),
        SHAPE_TRACTOR => c"Tractor".as_ptr(),
        SHAPE_HOUSE => c"House".as_ptr(),
        SHAPE_SUN => c"Sun".as_ptr(),
        SHAPE_CLOUD => c"Cloud".as_ptr(),
        SHAPE_FLOWER => c"Flower".as_ptr(),
        SHAPE_CAR => c"Car".as_ptr(),
        SHAPE_STAR => c"Star".as_ptr(),
        SHAPE_HEART => c"Heart".as_ptr(),
        SHAPE_RAINBOW => c"Rainbow".as_ptr(),
        _ => c"Unknown".as_ptr(),
    }
}
