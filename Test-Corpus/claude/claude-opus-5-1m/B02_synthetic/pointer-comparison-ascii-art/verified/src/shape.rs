//! Translation of `shape.c` / `shape.h`.

use crate::cio::Out;
use crate::p;

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

/// Simulated address of `shapes[0]`, i.e. the first `malloc(sizeof(shape_t))`
/// performed by `shape_manager_init`.  The C program prints these pointers with
/// `%p`; the singletons are laid out consecutively on the heap, one glibc chunk
/// (`sizeof(shape_t)` = 2444 bytes -> 0x9a0 byte chunk) apart.
const SHAPE_ADDR_BASE: usize = 0x55b9_e1a3_d2a0;
const SHAPE_ADDR_STRIDE: usize = 0x9a0;

pub struct Shape {
    pub kind: i32,
    pub name: &'static str,
    pub art: &'static [&'static str],
    /// Part of `shape_t`, never used by the program's output.
    #[allow(dead_code)]
    pub width: i32,
    pub height: i32,
}

/// A `shape_t *`: the singletons are identified by their slot, which gives the
/// exact same identity semantics as the C pointer comparison.
pub type ShapeRef = usize;

const TREE_ART: &[&str] = &[
    "    /\\    ",
    "   /  \\   ",
    "  /____\\  ",
    "  /    \\  ",
    " /______\\ ",
    "    ||    ",
    "    ||    ",
];

const TRACTOR_ART: &[&str] = &[
    "      ________     ",
    "     |        |___ ",
    "     |  []  []|   |",
    "  ___|________|___|",
    " /  o        o   \\",
    "|___|        |___| ",
];

const HOUSE_ART: &[&str] = &[
    "     /\\     ",
    "    /  \\    ",
    "   /____\\   ",
    "   |    |   ",
    "   | [] |   ",
    "   |    |   ",
    "   |____|   ",
];

const SUN_ART: &[&str] = &[
    "  \\  |  / ",
    "   \\ | /  ",
    "--- (@) ---",
    "   / | \\  ",
    "  /  |  \\ ",
    "          ",
    "          ",
];

const CLOUD_ART: &[&str] = &[
    "   _____       ",
    "  /     \\_    ",
    " /  ___  _\\  ",
    "(__/   \\_)   ",
];

const FLOWER_ART: &[&str] = &[
    "  \\|/  ",
    " -(@)- ",
    "  /|\\  ",
    "   |   ",
    "   |   ",
    "  / \\  ",
    " /   \\ ",
];

const CAR_ART: &[&str] = &[
    "  ____       ",
    " /|_||_\\____ ",
    "( o     o  ) ",
    " -----------  ",
];

const STAR_ART: &[&str] = &[
    "    *    ",
    "   ***   ",
    "  *****  ",
    " ******* ",
    "*********",
];

const HEART_ART: &[&str] = &[
    " *** ***  ",
    "*********  ",
    "*********  ",
    " ******* ",
    "  *****  ",
    "   ***   ",
];

const RAINBOW_ART: &[&str] = &[
    "      _______      ",
    "    /         \\    ",
    "   /           \\   ",
    "  /             \\  ",
    " /               \\ ",
];

/// Holds the singleton shape instances (`shape_manager_init`).
pub struct ShapeManager {
    shapes: Vec<Option<Shape>>,
}

impl ShapeManager {
    pub fn new() -> ShapeManager {
        ShapeManager { shapes: Vec::new() }
    }

    /// `shape_manager_init`
    pub fn init(&mut self) {
        self.shapes = vec![
            Some(Shape {
                kind: SHAPE_TREE,
                name: "Tree",
                art: TREE_ART,
                width: 11,
                height: 7,
            }),
            Some(Shape {
                kind: SHAPE_TRACTOR,
                name: "Tractor",
                art: TRACTOR_ART,
                width: 20,
                height: 6,
            }),
            Some(Shape {
                kind: SHAPE_HOUSE,
                name: "House",
                art: HOUSE_ART,
                width: 13,
                height: 7,
            }),
            Some(Shape {
                kind: SHAPE_SUN,
                name: "Sun",
                art: SUN_ART,
                width: 11,
                height: 7,
            }),
            Some(Shape {
                kind: SHAPE_CLOUD,
                name: "Cloud",
                art: CLOUD_ART,
                width: 16,
                height: 4,
            }),
            Some(Shape {
                kind: SHAPE_FLOWER,
                name: "Flower",
                art: FLOWER_ART,
                width: 9,
                height: 7,
            }),
            Some(Shape {
                kind: SHAPE_CAR,
                name: "Car",
                art: CAR_ART,
                width: 16,
                height: 4,
            }),
            Some(Shape {
                kind: SHAPE_STAR,
                name: "Star",
                art: STAR_ART,
                width: 9,
                height: 5,
            }),
            Some(Shape {
                kind: SHAPE_HEART,
                name: "Heart",
                art: HEART_ART,
                width: 11,
                height: 6,
            }),
            Some(Shape {
                kind: SHAPE_RAINBOW,
                name: "Rainbow",
                art: RAINBOW_ART,
                width: 21,
                height: 5,
            }),
        ];
    }

    /// `shape_manager_cleanup`
    pub fn cleanup(&mut self) {
        for slot in self.shapes.iter_mut() {
            *slot = None;
        }
    }

    /// `shape_get`
    pub fn get(&self, shape_type: i32) -> Option<ShapeRef> {
        if shape_type < 0 || shape_type >= SHAPE_COUNT {
            return None;
        }
        Some(shape_type as ShapeRef)
    }

    pub fn shape(&self, r: ShapeRef) -> &Shape {
        self.shapes[r].as_ref().expect("shape singleton")
    }

    pub fn name(&self, r: ShapeRef) -> &'static str {
        self.shape(r).name
    }

    /// The `%p` value of the singleton.
    pub fn addr(&self, r: ShapeRef) -> usize {
        SHAPE_ADDR_BASE + r * SHAPE_ADDR_STRIDE
    }

    /// `shape_print`
    pub fn print(&self, out: &mut Out, shape: Option<ShapeRef>) {
        let r = match shape {
            None => {
                out.s("(null shape)\n");
                return;
            }
            Some(r) => r,
        };

        let shape = self.shape(r);
        p!(out, "{}:\n", shape.name);
        let mut i = 0;
        while i < shape.height {
            p!(out, "{}\n", shape.art[i as usize]);
            i += 1;
        }
    }
}

/// `shape_equals`: the C code compares the singleton pointers.
pub fn shape_equals(s1: ShapeRef, s2: ShapeRef) -> i32 {
    if s1 == s2 {
        1
    } else {
        0
    }
}

/// `shape_type_name`
pub fn shape_type_name(shape_type: i32) -> &'static str {
    match shape_type {
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

/// Format a pointer the way glibc's `%p` does.
pub fn fmt_ptr(addr: usize) -> String {
    format!("0x{:x}", addr)
}
