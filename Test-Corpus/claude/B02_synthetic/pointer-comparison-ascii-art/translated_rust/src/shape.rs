// shape.rs - Translation of shape.c/shape.h

pub const MAX_SHAPE_NAME: usize = 32;
pub const SHAPE_COUNT: i32 = 10;

#[derive(Clone, Copy, PartialEq, Eq)]
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

pub struct Shape {
    pub type_: i32,
    pub name: String,
    pub art: Vec<String>,
    #[allow(dead_code)]
    pub width: i32,
    pub height: i32,
}

impl Shape {
    fn new(type_: i32, name: &str, width: i32, height: i32, art: &[&str]) -> Self {
        // Mimic the C strncpy behavior - the name is at most MAX_SHAPE_NAME-1 chars
        let mut n = name.to_string();
        if n.len() > MAX_SHAPE_NAME - 1 {
            n.truncate(MAX_SHAPE_NAME - 1);
        }
        Shape {
            type_,
            name: n,
            art: art.iter().map(|s| s.to_string()).collect(),
            width,
            height,
        }
    }
}

pub struct ShapeManager {
    shapes: Vec<Box<Shape>>,
}

impl ShapeManager {
    pub fn new() -> Self {
        // Allocate each shape once (singleton pattern)
        let mut shapes: Vec<Box<Shape>> = Vec::with_capacity(SHAPE_COUNT as usize);

        // Tree
        shapes.push(Box::new(Shape::new(
            ShapeType::Tree as i32,
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
        )));

        // Tractor
        shapes.push(Box::new(Shape::new(
            ShapeType::Tractor as i32,
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
        )));

        // House
        shapes.push(Box::new(Shape::new(
            ShapeType::House as i32,
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
        )));

        // Sun
        shapes.push(Box::new(Shape::new(
            ShapeType::Sun as i32,
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
        )));

        // Cloud
        shapes.push(Box::new(Shape::new(
            ShapeType::Cloud as i32,
            "Cloud",
            16,
            4,
            &[
                "   _____       ",
                "  /     \\_    ",
                " /  ___  _\\  ",
                "(__/   \\_)   ",
            ],
        )));

        // Flower
        shapes.push(Box::new(Shape::new(
            ShapeType::Flower as i32,
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
        )));

        // Car
        shapes.push(Box::new(Shape::new(
            ShapeType::Car as i32,
            "Car",
            16,
            4,
            &[
                "  ____       ",
                " /|_||_\\____ ",
                "( o     o  ) ",
                " -----------  ",
            ],
        )));

        // Star
        shapes.push(Box::new(Shape::new(
            ShapeType::Star as i32,
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
        )));

        // Heart
        shapes.push(Box::new(Shape::new(
            ShapeType::Heart as i32,
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
        )));

        // Rainbow
        shapes.push(Box::new(Shape::new(
            ShapeType::Rainbow as i32,
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
        )));

        ShapeManager { shapes }
    }

    /// Returns a raw pointer to the singleton shape (matches C semantics for pointer comparisons).
    /// Returns null pointer if type is out of range.
    pub fn shape_get(&self, type_: i32) -> *const Shape {
        if type_ < 0 || type_ >= SHAPE_COUNT {
            return std::ptr::null();
        }
        &*self.shapes[type_ as usize] as *const Shape
    }
}

pub fn shape_print(shape: *const Shape) {
    if shape.is_null() {
        println!("(null shape)");
        return;
    }
    // SAFETY: caller guarantees shape points to a live Shape (singleton from ShapeManager).
    let s = unsafe { &*shape };
    println!("{}:", s.name);
    for i in 0..s.height as usize {
        println!("{}", s.art[i]);
    }
}

pub fn shape_equals(s1: *const Shape, s2: *const Shape) -> i32 {
    if s1 == s2 { 1 } else { 0 }
}

pub fn shape_type_name(type_: i32) -> &'static str {
    match type_ {
        0 => "Tree",
        1 => "Tractor",
        2 => "House",
        3 => "Sun",
        4 => "Cloud",
        5 => "Flower",
        6 => "Car",
        7 => "Star",
        8 => "Heart",
        9 => "Rainbow",
        _ => "Unknown",
    }
}
