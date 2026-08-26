// Translation of shape.c / shape.h to Rust.
// Singletons live in a Vec<Box<Shape>> behind a OnceLock.

use std::sync::OnceLock;

pub const MAX_SHAPE_WIDTH: usize = 80;
pub const MAX_SHAPE_HEIGHT: usize = 30;
pub const MAX_SHAPE_NAME: usize = 32;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
    pub fn from_int(v: i32) -> ShapeType {
        match v {
            0 => ShapeType::Tree,
            1 => ShapeType::Tractor,
            2 => ShapeType::House,
            3 => ShapeType::Sun,
            4 => ShapeType::Cloud,
            5 => ShapeType::Flower,
            6 => ShapeType::Car,
            7 => ShapeType::Star,
            8 => ShapeType::Heart,
            9 => ShapeType::Rainbow,
            _ => ShapeType::Tree, // unused / clamped
        }
    }
    pub fn as_int(self) -> i32 {
        self as i32
    }
}

pub struct Shape {
    pub stype: ShapeType,
    // C uses fixed-size char arrays for the name; we just store a String.
    pub name_buf: String,
    pub art: Vec<String>,
    pub width: i32,
    pub height: i32,
}

impl Shape {
    pub fn name(&self) -> &str {
        &self.name_buf
    }
    pub fn type_int(&self) -> i32 {
        self.stype as i32
    }
}

// A heap-stable singleton table.  Boxed so addresses are stable for the
// lifetime of the process (matching C's malloc'd singletons).
static SHAPES: OnceLock<Vec<Box<Shape>>> = OnceLock::new();

fn build_shape(stype: ShapeType, name: &str, height: i32, width: i32, art: &[&str]) -> Box<Shape> {
    Box::new(Shape {
        stype,
        name_buf: name.to_string(),
        art: art.iter().map(|s| s.to_string()).collect(),
        width,
        height,
    })
}

pub fn shape_manager_init() {
    let _ = SHAPES.set(vec![
        build_shape(
            ShapeType::Tree, "Tree", 7, 11,
            &[
                "    /\\    ",
                "   /  \\   ",
                "  /____\\  ",
                "  /    \\  ",
                " /______\\ ",
                "    ||    ",
                "    ||    ",
            ],
        ),
        build_shape(
            ShapeType::Tractor, "Tractor", 6, 20,
            &[
                "      ________     ",
                "     |        |___ ",
                "     |  []  []|   |",
                "  ___|________|___|",
                " /  o        o   \\",
                "|___|        |___| ",
            ],
        ),
        build_shape(
            ShapeType::House, "House", 7, 13,
            &[
                "     /\\     ",
                "    /  \\    ",
                "   /____\\   ",
                "   |    |   ",
                "   | [] |   ",
                "   |    |   ",
                "   |____|   ",
            ],
        ),
        build_shape(
            ShapeType::Sun, "Sun", 7, 11,
            &[
                "  \\  |  / ",
                "   \\ | /  ",
                "--- (@) ---",
                "   / | \\  ",
                "  /  |  \\ ",
                "          ",
                "          ",
            ],
        ),
        build_shape(
            ShapeType::Cloud, "Cloud", 4, 16,
            &[
                "   _____       ",
                "  /     \\_    ",
                " /  ___  _\\  ",
                "(__/   \\_)   ",
            ],
        ),
        build_shape(
            ShapeType::Flower, "Flower", 7, 9,
            &[
                "  \\|/  ",
                " -(@)- ",
                "  /|\\  ",
                "   |   ",
                "   |   ",
                "  / \\  ",
                " /   \\ ",
            ],
        ),
        build_shape(
            ShapeType::Car, "Car", 4, 16,
            &[
                "  ____       ",
                " /|_||_\\____ ",
                "( o     o  ) ",
                " -----------  ",
            ],
        ),
        build_shape(
            ShapeType::Star, "Star", 5, 9,
            &[
                "    *    ",
                "   ***   ",
                "  *****  ",
                " ******* ",
                "*********",
            ],
        ),
        build_shape(
            ShapeType::Heart, "Heart", 6, 11,
            &[
                " *** ***  ",
                "*********  ",
                "*********  ",
                " ******* ",
                "  *****  ",
                "   ***   ",
            ],
        ),
        build_shape(
            ShapeType::Rainbow, "Rainbow", 5, 21,
            &[
                "      _______      ",
                "    /         \\    ",
                "   /           \\   ",
                "  /             \\  ",
                " /               \\ ",
            ],
        ),
    ]);
}

pub fn shape_manager_cleanup() {
    // Singletons live for the program lifetime; we can't deallocate via OnceLock,
    // but the C cleanup is just freeing memory — no observable output.
}

pub fn shape_get(stype: ShapeType) -> Option<&'static Shape> {
    let v = SHAPES.get()?;
    v.get(stype as usize).map(|b| b.as_ref())
}

pub fn shape_print(shape: Option<&Shape>) {
    match shape {
        None => crate::print("(null shape)\n"),
        Some(s) => {
            crate::print(&format!("{}:\n", s.name_buf));
            for i in 0..s.height as usize {
                crate::print(&format!("{}\n", s.art[i]));
            }
        }
    }
}

pub fn shape_equals(a: Option<&Shape>, b: Option<&Shape>) -> i32 {
    match (a, b) {
        (Some(x), Some(y)) if std::ptr::eq(x as *const _, y as *const _) => 1,
        _ => 0,
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
