use std::sync::OnceLock;

pub const MAX_SHAPE_WIDTH: usize = 80;
pub const MAX_SHAPE_HEIGHT: usize = 30;
pub const MAX_SHAPE_NAME: usize = 32;

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    Count = 10,
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

    pub fn all() -> [Self; SHAPE_COUNT] {
        [
            Self::Tree,
            Self::Tractor,
            Self::House,
            Self::Sun,
            Self::Cloud,
            Self::Flower,
            Self::Car,
            Self::Star,
            Self::Heart,
            Self::Rainbow,
        ]
    }

    pub fn index(self) -> usize {
        self as usize
    }
}

pub const SHAPE_COUNT: usize = ShapeType::Count as usize;
pub type ShapeTypeT = ShapeType;

#[derive(Debug)]
pub struct Shape {
    pub type_: ShapeType,
    pub name: String,
    pub art: Vec<String>,
    pub width: i32,
    pub height: i32,
}

pub type ShapeT = Shape;

static SHAPES: OnceLock<[Shape; SHAPE_COUNT]> = OnceLock::new();

fn make_shape(type_: ShapeType, name: &str, width: i32, height: i32, art: &[&str]) -> Shape {
    Shape {
        type_,
        name: name.to_string(),
        art: art.iter().map(|s| s.to_string()).collect(),
        width,
        height,
    }
}

fn build_shapes() -> [Shape; SHAPE_COUNT] {
    [
        make_shape(ShapeType::Tree, "Tree", 11, 7, &[
            "    /\\    ",
            "   /  \\   ",
            "  /____\\  ",
            "  /    \\  ",
            " /______\\ ",
            "    ||    ",
            "    ||    ",
        ]),
        make_shape(ShapeType::Tractor, "Tractor", 20, 6, &[
            "      ________     ",
            "     |        |___ ",
            "     |  []  []|   |",
            "  ___|________|___|",
            " /  o        o   \\",
            "|___|        |___| ",
        ]),
        make_shape(ShapeType::House, "House", 13, 7, &[
            "     /\\     ",
            "    /  \\    ",
            "   /____\\   ",
            "   |    |   ",
            "   | [] |   ",
            "   |    |   ",
            "   |____|   ",
        ]),
        make_shape(ShapeType::Sun, "Sun", 11, 7, &[
            "  \\  |  / ",
            "   \\ | /  ",
            "--- (@) ---",
            "   / | \\  ",
            "  /  |  \\ ",
            "          ",
            "          ",
        ]),
        make_shape(ShapeType::Cloud, "Cloud", 16, 4, &[
            "   _____       ",
            "  /     \\_    ",
            " /  ___  _\\  ",
            "(__/   \\_)   ",
        ]),
        make_shape(ShapeType::Flower, "Flower", 9, 7, &[
            "  \\|/  ",
            " -(@)- ",
            "  /|\\  ",
            "   |   ",
            "   |   ",
            "  / \\  ",
            " /   \\ ",
        ]),
        make_shape(ShapeType::Car, "Car", 16, 4, &[
            "  ____       ",
            " /|_||_\\____ ",
            "( o     o  ) ",
            " -----------  ",
        ]),
        make_shape(ShapeType::Star, "Star", 9, 5, &[
            "    *    ",
            "   ***   ",
            "  *****  ",
            " ******* ",
            "*********",
        ]),
        make_shape(ShapeType::Heart, "Heart", 11, 6, &[
            " *** ***  ",
            "*********  ",
            "*********  ",
            " ******* ",
            "  *****  ",
            "   ***   ",
        ]),
        make_shape(ShapeType::Rainbow, "Rainbow", 21, 5, &[
            "      _______      ",
            "    /         \\    ",
            "   /           \\   ",
            "  /             \\  ",
            " /               \\ ",
        ]),
    ]
}

pub fn shape_manager_init() {
    let _ = SHAPES.get_or_init(build_shapes);
}

pub fn shape_manager_cleanup() {}

pub fn shape_get(type_: ShapeType) -> Option<&'static Shape> {
    SHAPES.get().map(|shapes| &shapes[type_.index()])
}

pub fn shape_print(shape: Option<&Shape>) {
    match shape {
        Some(shape) => {
            println!("{}:", shape.name);
            for line in shape.art.iter().take(shape.height.max(0) as usize) {
                println!("{}", line);
            }
        }
        None => println!("(null shape)"),
    }
}

pub fn shape_equals(s1: Option<&Shape>, s2: Option<&Shape>) -> i32 {
    if match (s1, s2) {
        (Some(a), Some(b)) => std::ptr::eq(a, b),
        (None, None) => true,
        _ => false,
    } {
        1
    } else {
        0
    }
}

pub fn shape_type_name(type_: ShapeType) -> &'static str {
    match type_ {
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
        ShapeType::Count => "Unknown",
    }
}
