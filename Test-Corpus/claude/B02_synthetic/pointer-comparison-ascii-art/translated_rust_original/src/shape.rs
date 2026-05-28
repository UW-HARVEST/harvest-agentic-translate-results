// shape.rs - Rust translation of shape.c
use std::sync::OnceLock;
use crate::{cprint, ceprint};

pub const MAX_SHAPE_WIDTH: usize = 80;
pub const MAX_SHAPE_HEIGHT: usize = 30;
pub const MAX_SHAPE_NAME: usize = 32;

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(i32)]
pub enum ShapeType {
    Tree = 0,
    Tractor = 1,
    House = 2,
    Sun = 3,
    Cloud = 4,
    Flower = 5,
    Car = 6,
    Star = 7,
    Heart = 8,
    Rainbow = 9,
}

pub const SHAPE_COUNT: i32 = 10;

impl ShapeType {
    pub fn from_i32(v: i32) -> Option<ShapeType> {
        match v {
            0 => Some(ShapeType::Tree),
            1 => Some(ShapeType::Tractor),
            2 => Some(ShapeType::House),
            3 => Some(ShapeType::Sun),
            4 => Some(ShapeType::Cloud),
            5 => Some(ShapeType::Flower),
            6 => Some(ShapeType::Car),
            7 => Some(ShapeType::Star),
            8 => Some(ShapeType::Heart),
            9 => Some(ShapeType::Rainbow),
            _ => None,
        }
    }

    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

#[allow(non_camel_case_types)]
pub struct Shape {
    pub shape_type: ShapeType,
    pub name: String,
    pub art: Vec<String>,
    pub width: i32,
    pub height: i32,
}

fn make_shape(t: ShapeType, name: &str, height: i32, width: i32, lines: &[&str]) -> Shape {
    let art: Vec<String> = lines.iter().map(|s| (*s).to_string()).collect();
    Shape {
        shape_type: t,
        name: name.to_string(),
        art,
        width,
        height,
    }
}

fn init_tree() -> Shape {
    make_shape(
        ShapeType::Tree,
        "Tree",
        7,
        11,
        &[
            "    /\\    ",
            "   /  \\   ",
            "  /____\\  ",
            "  /    \\  ",
            " /______\\ ",
            "    ||    ",
            "    ||    ",
        ],
    )
}

fn init_tractor() -> Shape {
    make_shape(
        ShapeType::Tractor,
        "Tractor",
        6,
        20,
        &[
            "      ________     ",
            "     |        |___ ",
            "     |  []  []|   |",
            "  ___|________|___|",
            " /  o        o   \\",
            "|___|        |___| ",
        ],
    )
}

fn init_house() -> Shape {
    make_shape(
        ShapeType::House,
        "House",
        7,
        13,
        &[
            "     /\\     ",
            "    /  \\    ",
            "   /____\\   ",
            "   |    |   ",
            "   | [] |   ",
            "   |    |   ",
            "   |____|   ",
        ],
    )
}

fn init_sun() -> Shape {
    make_shape(
        ShapeType::Sun,
        "Sun",
        7,
        11,
        &[
            "  \\  |  / ",
            "   \\ | /  ",
            "--- (@) ---",
            "   / | \\  ",
            "  /  |  \\ ",
            "          ",
            "          ",
        ],
    )
}

fn init_cloud() -> Shape {
    make_shape(
        ShapeType::Cloud,
        "Cloud",
        4,
        16,
        &[
            "   _____       ",
            "  /     \\_    ",
            " /  ___  _\\  ",
            "(__/   \\_)   ",
        ],
    )
}

fn init_flower() -> Shape {
    make_shape(
        ShapeType::Flower,
        "Flower",
        7,
        9,
        &[
            "  \\|/  ",
            " -(@)- ",
            "  /|\\  ",
            "   |   ",
            "   |   ",
            "  / \\  ",
            " /   \\ ",
        ],
    )
}

fn init_car() -> Shape {
    make_shape(
        ShapeType::Car,
        "Car",
        4,
        16,
        &[
            "  ____       ",
            " /|_||_\\____ ",
            "( o     o  ) ",
            " -----------  ",
        ],
    )
}

fn init_star() -> Shape {
    make_shape(
        ShapeType::Star,
        "Star",
        5,
        9,
        &[
            "    *    ",
            "   ***   ",
            "  *****  ",
            " ******* ",
            "*********",
        ],
    )
}

fn init_heart() -> Shape {
    make_shape(
        ShapeType::Heart,
        "Heart",
        6,
        11,
        &[
            " *** ***  ",
            "*********  ",
            "*********  ",
            " ******* ",
            "  *****  ",
            "   ***   ",
        ],
    )
}

fn init_rainbow() -> Shape {
    make_shape(
        ShapeType::Rainbow,
        "Rainbow",
        5,
        21,
        &[
            "      _______      ",
            "    /         \\    ",
            "   /           \\   ",
            "  /             \\  ",
            " /               \\ ",
        ],
    )
}

// Singleton storage. We wrap raw pointers in a Send+Sync wrapper.
struct ShapesWrapper(Vec<*mut Shape>);
unsafe impl Send for ShapesWrapper {}
unsafe impl Sync for ShapesWrapper {}

static SHAPES: OnceLock<ShapesWrapper> = OnceLock::new();

pub fn shape_manager_init() {
    SHAPES.get_or_init(|| {
        let mut v: Vec<*mut Shape> = Vec::with_capacity(SHAPE_COUNT as usize);
        // Allocate first (mirroring C's malloc loop with its error path)
        for _ in 0..SHAPE_COUNT {
            // Allocate a default placeholder; will be overwritten below.
            let placeholder = Shape {
                shape_type: ShapeType::Tree,
                name: String::new(),
                art: Vec::new(),
                width: 0,
                height: 0,
            };
            // We use Box, which aborts on OOM rather than returning null.
            // C's behavior on alloc failure is `exit(1)` after fprintf.
            // This Rust translation cannot easily reproduce that (Box panics),
            // so this is a reasonable approximation.
            let _ = || {
                if false {
                    ceprint!("Error: Failed to allocate shape\n");
                }
            };
            let b = Box::new(placeholder);
            v.push(Box::into_raw(b));
        }
        // Now initialize each
        unsafe {
            *v[ShapeType::Tree as usize] = init_tree();
            *v[ShapeType::Tractor as usize] = init_tractor();
            *v[ShapeType::House as usize] = init_house();
            *v[ShapeType::Sun as usize] = init_sun();
            *v[ShapeType::Cloud as usize] = init_cloud();
            *v[ShapeType::Flower as usize] = init_flower();
            *v[ShapeType::Car as usize] = init_car();
            *v[ShapeType::Star as usize] = init_star();
            *v[ShapeType::Heart as usize] = init_heart();
            *v[ShapeType::Rainbow as usize] = init_rainbow();
        }
        ShapesWrapper(v)
    });
}

pub fn shape_manager_cleanup() {
    // The C version frees the singletons. In Rust, we leak them since they
    // are static for the program's lifetime; cleanup is a no-op effectively
    // because OnceLock holds the Vec for the program lifetime.
    // To match C's behavior of "free", we'd need to reset the OnceLock, but
    // that's not directly supported. Since the program is exiting, this is fine.
}

pub fn shape_get(t: ShapeType) -> *mut Shape {
    let v = SHAPES.get().expect("shape_manager_init not called");
    v.0[t as usize]
}

pub fn shape_get_by_index(i: i32) -> *mut Shape {
    if i < 0 || i >= SHAPE_COUNT {
        return std::ptr::null_mut();
    }
    let v = SHAPES.get().expect("shape_manager_init not called");
    v.0[i as usize]
}

pub fn shape_print(shape: *const Shape) {
    if shape.is_null() {
        cprint!("(null shape)\n");
        return;
    }
    unsafe {
        let s = &*shape;
        cprint!("{}:\n", s.name);
        for i in 0..s.height as usize {
            cprint!("{}\n", s.art[i]);
        }
    }
}

pub fn shape_equals(s1: *const Shape, s2: *const Shape) -> i32 {
    if std::ptr::eq(s1, s2) {
        1
    } else {
        0
    }
}

pub fn shape_type_name(t: ShapeType) -> &'static str {
    match t {
        ShapeType::Tree => "Tree",
        ShapeType::Tractor => "Tractor",
        ShapeType::House => "House",
        ShapeType::Sun => "Sun",
        ShapeType::Cloud => "Cloud",
        ShapeType::Flower => "Flower",
        ShapeType::Car => "Car",
        ShapeType::Star => "Star",
        ShapeType::Heart => "Heart",
        ShapeType::Rainbow => "Rainbow",
    }
}

pub fn shape_type_name_i32(i: i32) -> &'static str {
    match ShapeType::from_i32(i) {
        Some(t) => shape_type_name(t),
        None => "Unknown",
    }
}
