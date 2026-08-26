use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeType {
    Tree,
    Tractor,
    House,
    Sun,
    Cloud,
    Flower,
    Car,
    Star,
    Heart,
    Rainbow,
}

impl ShapeType {
    pub const COUNT: usize = 10;

    pub fn name(&self) -> &'static str {
        match self {
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

    pub fn from_usize(val: usize) -> Option<Self> {
        match val {
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
}

pub struct Shape {
    pub shape_type: ShapeType,
    pub name: String,
    pub art: Vec<String>,
    pub width: usize,
    pub height: usize,
}

static SHAPES: OnceLock<Vec<Shape>> = OnceLock::new();

pub fn shape_manager_init() {
    let mut shapes = Vec::new();

    shapes.push(Shape {
        shape_type: ShapeType::Tree,
        name: "Tree".to_string(),
        art: vec![
            "    /\\    ".to_string(),
            "   /  \\   ".to_string(),
            "  /____\\  ".to_string(),
            "  /    \\  ".to_string(),
            " /______\\ ".to_string(),
            "    ||    ".to_string(),
            "    ||    ".to_string(),
        ],
        width: 11,
        height: 7,
    });

    shapes.push(Shape {
        shape_type: ShapeType::Tractor,
        name: "Tractor".to_string(),
        art: vec![
            "      ________     ".to_string(),
            "     |        |___ ".to_string(),
            "     |  []  []|   |".to_string(),
            "  ___|________|___|".to_string(),
            " /  o        o   \\".to_string(),
            "|___|        |___| ".to_string(),
        ],
        width: 20,
        height: 6,
    });

    shapes.push(Shape {
        shape_type: ShapeType::House,
        name: "House".to_string(),
        art: vec![
            "     /\\     ".to_string(),
            "    /  \\    ".to_string(),
            "   /____\\   ".to_string(),
            "   |    |   ".to_string(),
            "   | [] |   ".to_string(),
            "   |    |   ".to_string(),
            "   |____|   ".to_string(),
        ],
        width: 13,
        height: 7,
    });

    shapes.push(Shape {
        shape_type: ShapeType::Sun,
        name: "Sun".to_string(),
        art: vec![
            "  \\  |  / ".to_string(),
            "   \\ | /  ".to_string(),
            "--- (@) ---".to_string(),
            "   / | \\  ".to_string(),
            "  /  |  \\ ".to_string(),
            "          ".to_string(),
            "          ".to_string(),
        ],
        width: 11,
        height: 7,
    });

    shapes.push(Shape {
        shape_type: ShapeType::Cloud,
        name: "Cloud".to_string(),
        art: vec![
            "   _____       ".to_string(),
            "  /     \\_    ".to_string(),
            " /  ___  _\\  ".to_string(),
            "(__/   \\_)   ".to_string(),
        ],
        width: 16,
        height: 4,
    });

    shapes.push(Shape {
        shape_type: ShapeType::Flower,
        name: "Flower".to_string(),
        art: vec![
            "  \\|/  ".to_string(),
            " -(@)- ".to_string(),
            "  /|\\  ".to_string(),
            "   |   ".to_string(),
            "   |   ".to_string(),
            "  / \\  ".to_string(),
            " /   \\ ".to_string(),
        ],
        width: 9,
        height: 7,
    });

    shapes.push(Shape {
        shape_type: ShapeType::Car,
        name: "Car".to_string(),
        art: vec![
            "  ____       ".to_string(),
            " /|_||_\\____ ".to_string(),
            "( o     o  ) ".to_string(),
            " -----------  ".to_string(),
        ],
        width: 16,
        height: 4,
    });

    shapes.push(Shape {
        shape_type: ShapeType::Star,
        name: "Star".to_string(),
        art: vec![
            "    *    ".to_string(),
            "   ***   ".to_string(),
            "  *****  ".to_string(),
            " ******* ".to_string(),
            "*********".to_string(),
        ],
        width: 9,
        height: 5,
    });

    shapes.push(Shape {
        shape_type: ShapeType::Heart,
        name: "Heart".to_string(),
        art: vec![
            " *** ***  ".to_string(),
            "*********  ".to_string(),
            "*********  ".to_string(),
            " ******* ".to_string(),
            "  *****  ".to_string(),
            "   ***   ".to_string(),
        ],
        width: 11,
        height: 6,
    });

    shapes.push(Shape {
        shape_type: ShapeType::Rainbow,
        name: "Rainbow".to_string(),
        art: vec![
            "      _______      ".to_string(),
            "    /         \\    ".to_string(),
            "   /           \\   ".to_string(),
            "  /             \\  ".to_string(),
            " /               \\ ".to_string(),
        ],
        width: 21,
        height: 5,
    });

    let _ = SHAPES.set(shapes);
}

pub fn shape_manager_cleanup() {
    // No-op in Rust
}

pub fn shape_get(shape_type: ShapeType) -> Option<&'static Shape> {
    SHAPES.get().and_then(|shapes| shapes.get(shape_type as usize))
}

pub fn shape_print(shape: Option<&Shape>) {
    if let Some(s) = shape {
        println!("{}:", s.name);
        for line in &s.art {
            println!("{}", line);
        }
    } else {
        println!("(null shape)");
    }
}

pub fn shape_equals(s1: Option<&Shape>, s2: Option<&Shape>) -> bool {
    match (s1, s2) {
        (Some(a), Some(b)) => std::ptr::eq(a, b),
        (None, None) => true,
        _ => false,
    }
}
