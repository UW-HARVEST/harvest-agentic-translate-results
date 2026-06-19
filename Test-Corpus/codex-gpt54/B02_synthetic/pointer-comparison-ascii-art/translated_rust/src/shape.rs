#![allow(dead_code)]

pub const MAX_SHAPE_WIDTH: usize = 80;
pub const MAX_SHAPE_HEIGHT: usize = 30;
pub const MAX_SHAPE_NAME: usize = 32;
pub const SHAPE_COUNT: usize = 10;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

impl ShapeType {
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Tree),
            1 => Some(Self::Tractor),
            2 => Some(Self::House),
            3 => Some(Self::Sun),
            4 => Some(Self::Cloud),
            5 => Some(Self::Flower),
            6 => Some(Self::Car),
            7 => Some(Self::Star),
            8 => Some(Self::Heart),
            9 => Some(Self::Rainbow),
            _ => None,
        }
    }
}

pub struct Shape {
    pub shape_type: ShapeType,
    pub name: &'static str,
    pub art: &'static [&'static str],
    pub width: i32,
    pub height: i32,
}

pub struct ShapeManager {
    shapes: [Box<Shape>; SHAPE_COUNT],
}

impl ShapeManager {
    pub fn new() -> Self {
        Self {
            shapes: [
                Box::new(init_tree()),
                Box::new(init_tractor()),
                Box::new(init_house()),
                Box::new(init_sun()),
                Box::new(init_cloud()),
                Box::new(init_flower()),
                Box::new(init_car()),
                Box::new(init_star()),
                Box::new(init_heart()),
                Box::new(init_rainbow()),
            ],
        }
    }

    pub fn get(&self, shape_type: ShapeType) -> &Shape {
        &self.shapes[shape_type as usize]
    }

    pub fn get_by_i32(&self, value: i32) -> Option<&Shape> {
        ShapeType::from_i32(value).map(|shape_type| self.get(shape_type))
    }
}

pub fn shape_print(shape: Option<&Shape>) {
    match shape {
        Some(shape) => {
            println!("{}:", shape.name);
            for line in shape.art.iter().take(shape.height as usize) {
                println!("{line}");
            }
        }
        None => println!("(null shape)"),
    }
}

pub fn shape_equals(s1: &Shape, s2: &Shape) -> bool {
    std::ptr::eq(s1, s2)
}

pub fn shape_type_name(value: i32) -> &'static str {
    match ShapeType::from_i32(value) {
        Some(ShapeType::Tree) => "Tree",
        Some(ShapeType::Tractor) => "Tractor",
        Some(ShapeType::House) => "House",
        Some(ShapeType::Sun) => "Sun",
        Some(ShapeType::Cloud) => "Cloud",
        Some(ShapeType::Flower) => "Flower",
        Some(ShapeType::Car) => "Car",
        Some(ShapeType::Star) => "Star",
        Some(ShapeType::Heart) => "Heart",
        Some(ShapeType::Rainbow) => "Rainbow",
        None => "Unknown",
    }
}

fn init_tree() -> Shape {
    Shape {
        shape_type: ShapeType::Tree,
        name: "Tree",
        height: 7,
        width: 11,
        art: &[
            "    /\\    ",
            "   /  \\   ",
            "  /____\\  ",
            "  /    \\  ",
            " /______\\ ",
            "    ||    ",
            "    ||    ",
        ],
    }
}

fn init_tractor() -> Shape {
    Shape {
        shape_type: ShapeType::Tractor,
        name: "Tractor",
        height: 6,
        width: 20,
        art: &[
            "      ________     ",
            "     |        |___ ",
            "     |  []  []|   |",
            "  ___|________|___|",
            " /  o        o   \\",
            "|___|        |___| ",
        ],
    }
}

fn init_house() -> Shape {
    Shape {
        shape_type: ShapeType::House,
        name: "House",
        height: 7,
        width: 13,
        art: &[
            "     /\\     ",
            "    /  \\    ",
            "   /____\\   ",
            "   |    |   ",
            "   | [] |   ",
            "   |    |   ",
            "   |____|   ",
        ],
    }
}

fn init_sun() -> Shape {
    Shape {
        shape_type: ShapeType::Sun,
        name: "Sun",
        height: 7,
        width: 11,
        art: &[
            "  \\  |  / ",
            "   \\ | /  ",
            "--- (@) ---",
            "   / | \\  ",
            "  /  |  \\ ",
            "          ",
            "          ",
        ],
    }
}

fn init_cloud() -> Shape {
    Shape {
        shape_type: ShapeType::Cloud,
        name: "Cloud",
        height: 4,
        width: 16,
        art: &[
            "   _____       ",
            "  /     \\_    ",
            " /  ___  _\\  ",
            "(__/   \\_)   ",
        ],
    }
}

fn init_flower() -> Shape {
    Shape {
        shape_type: ShapeType::Flower,
        name: "Flower",
        height: 7,
        width: 9,
        art: &[
            "  \\|/  ",
            " -(@)- ",
            "  /|\\  ",
            "   |   ",
            "   |   ",
            "  / \\  ",
            " /   \\ ",
        ],
    }
}

fn init_car() -> Shape {
    Shape {
        shape_type: ShapeType::Car,
        name: "Car",
        height: 4,
        width: 16,
        art: &[
            "  ____       ",
            " /|_||_\\____ ",
            "( o     o  ) ",
            " -----------  ",
        ],
    }
}

fn init_star() -> Shape {
    Shape {
        shape_type: ShapeType::Star,
        name: "Star",
        height: 5,
        width: 9,
        art: &[
            "    *    ",
            "   ***   ",
            "  *****  ",
            " ******* ",
            "*********",
        ],
    }
}

fn init_heart() -> Shape {
    Shape {
        shape_type: ShapeType::Heart,
        name: "Heart",
        height: 6,
        width: 11,
        art: &[
            " *** ***  ",
            "*********  ",
            "*********  ",
            " ******* ",
            "  *****  ",
            "   ***   ",
        ],
    }
}

fn init_rainbow() -> Shape {
    Shape {
        shape_type: ShapeType::Rainbow,
        name: "Rainbow",
        height: 5,
        width: 21,
        art: &[
            "      _______      ",
            "    /         \\    ",
            "   /           \\   ",
            "  /             \\  ",
            " /               \\ ",
        ],
    }
}
