// shape.rs - Translation of shape.c/shape.h to Rust

use std::sync::OnceLock;

pub const MAX_SHAPE_WIDTH: usize = 80;
pub const MAX_SHAPE_HEIGHT: usize = 30;
pub const MAX_SHAPE_NAME: usize = 32;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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

pub const SHAPE_COUNT: usize = 10;

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

pub struct Shape {
    pub shape_type: ShapeType,
    pub name: String,
    pub art: Vec<String>,
    pub width: i32,
    pub height: i32,
}

fn make_shape(
    shape_type: ShapeType,
    name: &str,
    width: i32,
    height: i32,
    art: &[&str],
) -> Shape {
    Shape {
        shape_type,
        name: name.to_string(),
        art: art.iter().map(|s| s.to_string()).collect(),
        width,
        height,
    }
}

fn init_tree() -> Shape {
    make_shape(
        ShapeType::Tree,
        "Tree",
        11,
        7,
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
        20,
        6,
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
        13,
        7,
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
        11,
        7,
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
        16,
        4,
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
        9,
        7,
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
        16,
        4,
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
        9,
        5,
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
        11,
        6,
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
        21,
        5,
        &[
            "      _______      ",
            "    /         \\    ",
            "   /           \\   ",
            "  /             \\  ",
            " /               \\ ",
        ],
    )
}

static SHAPES: OnceLock<Vec<Shape>> = OnceLock::new();

pub fn shape_manager_init() {
    let _ = SHAPES.set(vec![
        init_tree(),
        init_tractor(),
        init_house(),
        init_sun(),
        init_cloud(),
        init_flower(),
        init_car(),
        init_star(),
        init_heart(),
        init_rainbow(),
    ]);
}

pub fn shape_manager_cleanup() {
    // In Rust, the OnceLock will live until program end.
    // We can't easily reset it; data will be reclaimed at exit.
}

pub fn shape_get(shape_type: ShapeType) -> Option<&'static Shape> {
    let shapes = SHAPES.get()?;
    shapes.get(shape_type.as_i32() as usize)
}

pub fn shape_get_by_index(index: i32) -> Option<&'static Shape> {
    if index < 0 || (index as usize) >= SHAPE_COUNT {
        return None;
    }
    let shapes = SHAPES.get()?;
    shapes.get(index as usize)
}

pub fn shape_print(shape: Option<&Shape>) {
    match shape {
        None => {
            println!("(null shape)");
        }
        Some(s) => {
            println!("{}:", s.name);
            for i in 0..(s.height as usize) {
                if i < s.art.len() {
                    println!("{}", s.art[i]);
                }
            }
        }
    }
}

/// Compare two shape references; equal when they point to the same singleton.
pub fn shape_equals(s1: Option<&Shape>, s2: Option<&Shape>) -> i32 {
    match (s1, s2) {
        (Some(a), Some(b)) => {
            if std::ptr::eq(a, b) {
                1
            } else {
                0
            }
        }
        _ => 0,
    }
}

pub fn shape_type_name(shape_type: ShapeType) -> &'static str {
    match shape_type {
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
