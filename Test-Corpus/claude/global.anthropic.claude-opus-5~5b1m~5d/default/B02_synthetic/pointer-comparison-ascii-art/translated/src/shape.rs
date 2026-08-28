//! Translation of shape.c / shape.h
//!
//! The C code allocates one `shape_t` per shape type with malloc and treats the
//! resulting pointers as identities (`shape_equals` is a pointer comparison and
//! the pointers themselves are printed with `%p`).  The translation keeps that
//! model: every shape is heap allocated exactly once and then referred to by a
//! `&'static Shape`, so pointer identity and the printed addresses behave the
//! same way.

use crate::cio::COut;
use std::sync::OnceLock;

pub const MAX_SHAPE_HEIGHT: usize = 30;

pub const SHAPE_TREE: i32 = 0;
pub const SHAPE_TRACTOR: i32 = 1;
pub const SHAPE_HOUSE: i32 = 2;
pub const SHAPE_SUN: i32 = 3;
pub const SHAPE_CLOUD: i32 = 4;
pub const SHAPE_FLOWER: i32 = 5;
pub const SHAPE_CAR: i32 = 6;
pub const SHAPE_STAR: i32 = 7;
pub const SHAPE_HEART: i32 = 8;
pub const SHAPE_RAINBOW: i32 = 9;
pub const SHAPE_COUNT: i32 = 10;

pub struct Shape {
    pub stype: i32,
    pub name: Vec<u8>,
    pub art: Vec<Vec<u8>>,
    pub width: i32,
    pub height: i32,
}

impl Shape {
    fn blank() -> Shape {
        Shape {
            stype: 0,
            name: Vec::new(),
            art: vec![Vec::new(); MAX_SHAPE_HEIGHT],
            width: 0,
            height: 0,
        }
    }
}

fn init_tree(shape: &mut Shape) {
    shape.stype = SHAPE_TREE;
    shape.name = b"Tree".to_vec();
    shape.height = 7;
    shape.width = 11;

    shape.art[0] = b"    /\\    ".to_vec();
    shape.art[1] = b"   /  \\   ".to_vec();
    shape.art[2] = b"  /____\\  ".to_vec();
    shape.art[3] = b"  /    \\  ".to_vec();
    shape.art[4] = b" /______\\ ".to_vec();
    shape.art[5] = b"    ||    ".to_vec();
    shape.art[6] = b"    ||    ".to_vec();
}

fn init_tractor(shape: &mut Shape) {
    shape.stype = SHAPE_TRACTOR;
    shape.name = b"Tractor".to_vec();
    shape.height = 6;
    shape.width = 20;

    shape.art[0] = b"      ________     ".to_vec();
    shape.art[1] = b"     |        |___ ".to_vec();
    shape.art[2] = b"     |  []  []|   |".to_vec();
    shape.art[3] = b"  ___|________|___|".to_vec();
    shape.art[4] = b" /  o        o   \\".to_vec();
    shape.art[5] = b"|___|        |___| ".to_vec();
}

fn init_house(shape: &mut Shape) {
    shape.stype = SHAPE_HOUSE;
    shape.name = b"House".to_vec();
    shape.height = 7;
    shape.width = 13;

    shape.art[0] = b"     /\\     ".to_vec();
    shape.art[1] = b"    /  \\    ".to_vec();
    shape.art[2] = b"   /____\\   ".to_vec();
    shape.art[3] = b"   |    |   ".to_vec();
    shape.art[4] = b"   | [] |   ".to_vec();
    shape.art[5] = b"   |    |   ".to_vec();
    shape.art[6] = b"   |____|   ".to_vec();
}

fn init_sun(shape: &mut Shape) {
    shape.stype = SHAPE_SUN;
    shape.name = b"Sun".to_vec();
    shape.height = 7;
    shape.width = 11;

    shape.art[0] = b"  \\  |  / ".to_vec();
    shape.art[1] = b"   \\ | /  ".to_vec();
    shape.art[2] = b"--- (@) ---".to_vec();
    shape.art[3] = b"   / | \\  ".to_vec();
    shape.art[4] = b"  /  |  \\ ".to_vec();
    shape.art[5] = b"          ".to_vec();
    shape.art[6] = b"          ".to_vec();
}

fn init_cloud(shape: &mut Shape) {
    shape.stype = SHAPE_CLOUD;
    shape.name = b"Cloud".to_vec();
    shape.height = 4;
    shape.width = 16;

    shape.art[0] = b"   _____       ".to_vec();
    shape.art[1] = b"  /     \\_    ".to_vec();
    shape.art[2] = b" /  ___  _\\  ".to_vec();
    shape.art[3] = b"(__/   \\_)   ".to_vec();
}

fn init_flower(shape: &mut Shape) {
    shape.stype = SHAPE_FLOWER;
    shape.name = b"Flower".to_vec();
    shape.height = 7;
    shape.width = 9;

    shape.art[0] = b"  \\|/  ".to_vec();
    shape.art[1] = b" -(@)- ".to_vec();
    shape.art[2] = b"  /|\\  ".to_vec();
    shape.art[3] = b"   |   ".to_vec();
    shape.art[4] = b"   |   ".to_vec();
    shape.art[5] = b"  / \\  ".to_vec();
    shape.art[6] = b" /   \\ ".to_vec();
}

fn init_car(shape: &mut Shape) {
    shape.stype = SHAPE_CAR;
    shape.name = b"Car".to_vec();
    shape.height = 4;
    shape.width = 16;

    shape.art[0] = b"  ____       ".to_vec();
    shape.art[1] = b" /|_||_\\____ ".to_vec();
    shape.art[2] = b"( o     o  ) ".to_vec();
    shape.art[3] = b" -----------  ".to_vec();
}

fn init_star(shape: &mut Shape) {
    shape.stype = SHAPE_STAR;
    shape.name = b"Star".to_vec();
    shape.height = 5;
    shape.width = 9;

    shape.art[0] = b"    *    ".to_vec();
    shape.art[1] = b"   ***   ".to_vec();
    shape.art[2] = b"  *****  ".to_vec();
    shape.art[3] = b" ******* ".to_vec();
    shape.art[4] = b"*********".to_vec();
}

fn init_heart(shape: &mut Shape) {
    shape.stype = SHAPE_HEART;
    shape.name = b"Heart".to_vec();
    shape.height = 6;
    shape.width = 11;

    shape.art[0] = b" *** ***  ".to_vec();
    shape.art[1] = b"*********  ".to_vec();
    shape.art[2] = b"*********  ".to_vec();
    shape.art[3] = b" ******* ".to_vec();
    shape.art[4] = b"  *****  ".to_vec();
    shape.art[5] = b"   ***   ".to_vec();
}

fn init_rainbow(shape: &mut Shape) {
    shape.stype = SHAPE_RAINBOW;
    shape.name = b"Rainbow".to_vec();
    shape.height = 5;
    shape.width = 21;

    shape.art[0] = b"      _______      ".to_vec();
    shape.art[1] = b"    /         \\    ".to_vec();
    shape.art[2] = b"   /           \\   ".to_vec();
    shape.art[3] = b"  /             \\  ".to_vec();
    shape.art[4] = b" /               \\ ".to_vec();
}


// Singleton shape instances
static SHAPES: OnceLock<Vec<&'static Shape>> = OnceLock::new();

/// Initialize the shape manager (allocate all shapes once)
pub fn shape_manager_init() {
    let mut shapes: Vec<&'static Shape> = Vec::new();
    for i in 0..SHAPE_COUNT {
        let mut shape = Box::new(Shape::blank());
        match i {
            SHAPE_TREE => init_tree(&mut shape),
            SHAPE_TRACTOR => init_tractor(&mut shape),
            SHAPE_HOUSE => init_house(&mut shape),
            SHAPE_SUN => init_sun(&mut shape),
            SHAPE_CLOUD => init_cloud(&mut shape),
            SHAPE_FLOWER => init_flower(&mut shape),
            SHAPE_CAR => init_car(&mut shape),
            SHAPE_STAR => init_star(&mut shape),
            SHAPE_HEART => init_heart(&mut shape),
            SHAPE_RAINBOW => init_rainbow(&mut shape),
            _ => {}
        }
        shapes.push(Box::leak(shape));
    }
    let _ = SHAPES.set(shapes);
}

/// Clean up shape manager.
///
/// The C version frees the singletons and NULLs the table; it is only ever
/// called immediately before the program returns from main, so there is nothing
/// observable to reproduce here.
pub fn shape_manager_cleanup() {}

/// Get a shape by type (returns the singleton instance)
pub fn shape_get(stype: i32) -> Option<&'static Shape> {
    if stype < 0 || stype >= SHAPE_COUNT {
        return None;
    }
    SHAPES.get().map(|v| v[stype as usize])
}

/// The address that C would print with `%p` for this shape.
pub fn shape_ptr(shape: &Shape) -> String {
    format!("{:p}", shape as *const Shape)
}

/// Print a shape to stdout
pub fn shape_print(out: &mut COut, shape: Option<&Shape>) {
    let shape = match shape {
        None => {
            out.puts("(null shape)\n");
            return;
        }
        Some(s) => s,
    };

    out.put(&shape.name);
    out.puts(":\n");
    for i in 0..shape.height {
        out.put(&shape.art[i as usize]);
        out.puts("\n");
    }
}

/// Compare two shapes (equal if the pointers are identical)
pub fn shape_equals(s1: Option<&Shape>, s2: Option<&Shape>) -> i32 {
    if shape_same(s1, s2) {
        1
    } else {
        0
    }
}

/// `s1 == s2` on the raw pointers.
pub fn shape_same(s1: Option<&Shape>, s2: Option<&Shape>) -> bool {
    match (s1, s2) {
        (Some(a), Some(b)) => std::ptr::eq(a as *const Shape, b as *const Shape),
        (None, None) => true,
        _ => false,
    }
}

/// Get shape type name
pub fn shape_type_name(stype: i32) -> &'static str {
    match stype {
        SHAPE_TREE => "Tree",
        SHAPE_TRACTOR => "Tractor",
        SHAPE_HOUSE => "House",
        SHAPE_SUN => "Sun",
        SHAPE_CLOUD => "Cloud",
        SHAPE_FLOWER => "Flower",
        SHAPE_CAR => "Car",
        SHAPE_STAR => "Star",
        SHAPE_HEART => "Heart",
        SHAPE_RAINBOW => "Rainbow",
        _ => "Unknown",
    }
}
