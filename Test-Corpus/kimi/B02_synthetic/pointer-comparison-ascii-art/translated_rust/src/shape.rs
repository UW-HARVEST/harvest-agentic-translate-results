use std::sync::Mutex;
use std::sync::OnceLock;

const MAX_SHAPE_WIDTH: usize = 80;
const MAX_SHAPE_HEIGHT: usize = 30;
const MAX_SHAPE_NAME: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    pub const COUNT: usize = 11;
    
    pub fn from_usize(value: usize) -> Self {
        match value {
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
            _ => panic!("Invalid shape type"),
        }
    }
    
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
}

pub struct Shape {
    shape_type: ShapeType,
    name: String,
    art: Vec<String>,
    width: usize,
    height: usize,
}

impl Shape {
    fn new(shape_type: ShapeType, name: &str, art: &[&str], width: usize, height: usize) -> Self {
        let art: Vec<String> = art.iter().map(|s| s.to_string()).collect();
        Shape {
            shape_type,
            name: name.to_string(),
            art,
            width,
            height,
        }
    }
    
    pub fn shape_type(&self) -> ShapeType {
        self.shape_type
    }
    
    pub fn name(&self) -> &str {
        &self.name
    }
    
    pub fn art(&self) -> &[String] {
        &self.art
    }
    
    pub fn width(&self) -> usize {
        self.width
    }
    
    pub fn height(&self) -> usize {
        self.height
    }
    
    pub fn print(&self) {
        println!("{}:", self.name);
        for line in &self.art {
            println!("{}", line);
        }
    }
}

static SHAPES: OnceLock<Mutex<Vec<Option<Box<Shape>>>>> = OnceLock::new();

pub struct ShapeManager;

impl ShapeManager {
    pub fn init() {
        let _ = SHAPES.get_or_init(|| {
            let mut shapes: Vec<Option<Box<Shape>>> = Vec::with_capacity(ShapeType::COUNT);
            
            shapes.push(Some(Box::new(Shape::new(
                ShapeType::Tree,
                "Tree",
                &[
                    "    /\\    ",
                    "   /  \\   ",
                    "  /____\\  ",
                    "  /    \\  ",
                    " /______\\ ",
                    "    ||    ",
                    "    ||    ",
                ],
                11,
                7,
            ))));
            
            shapes.push(Some(Box::new(Shape::new(
                ShapeType::Tractor,
                "Tractor",
                &[
                    "      ________     ",
                    "     |        |___ ",
                    "     |  []  []|   |",
                    "  ___|________|___|",
                    " /  o        o   \\",
                    "|___|        |___| ",
                ],
                20,
                6,
            ))));
            
            shapes.push(Some(Box::new(Shape::new(
                ShapeType::House,
                "House",
                &[
                    "     /\\     ",
                    "    /  \\    ",
                    "   /____\\   ",
                    "   |    |   ",
                    "   | [] |   ",
                    "   |    |   ",
                    "   |____|   ",
                ],
                13,
                7,
            ))));
            
            shapes.push(Some(Box::new(Shape::new(
                ShapeType::Sun,
                "Sun",
                &[
                    "  \\  |  / ",
                    "   \\ | /  ",
                    "--- (@) ---",
                    "   / | \\  ",
                    "  /  |  \\ ",
                    "          ",
                    "          ",
                ],
                11,
                7,
            ))));
            
            shapes.push(Some(Box::new(Shape::new(
                ShapeType::Cloud,
                "Cloud",
                &[
                    "   _____       ",
                    "  /     \\_    ",
                    " /  ___  _\\  ",
                    "(__/   \\_)   ",
                ],
                16,
                4,
            ))));
            
            shapes.push(Some(Box::new(Shape::new(
                ShapeType::Flower,
                "Flower",
                &[
                    "  \\|/  ",
                    " -(@)- ",
                    "  /|\\  ",
                    "   |   ",
                    "   |   ",
                    "  / \\  ",
                    " /   \\ ",
                ],
                9,
                7,
            ))));
            
            shapes.push(Some(Box::new(Shape::new(
                ShapeType::Car,
                "Car",
                &[
                    "  ____       ",
                    " /|_||_\\____ ",
                    "( o     o  ) ",
                    " -----------  ",
                ],
                16,
                4,
            ))));
            
            shapes.push(Some(Box::new(Shape::new(
                ShapeType::Star,
                "Star",
                &[
                    "    *    ",
                    "   ***   ",
                    "  *****  ",
                    " ******* ",
                    "*********",
                ],
                9,
                5,
            ))));
            
            shapes.push(Some(Box::new(Shape::new(
                ShapeType::Heart,
                "Heart",
                &[
                    " *** ***  ",
                    "*********  ",
                    "*********  ",
                    " ******* ",
                    "  *****  ",
                    "   ***   ",
                ],
                11,
                6,
            ))));
            
            shapes.push(Some(Box::new(Shape::new(
                ShapeType::Rainbow,
                "Rainbow",
                &[
                    "      _______      ",
                    "    /         \\    ",
                    "   /           \\   ",
                    "  /             \\  ",
                    " /               \\ ",
                ],
                21,
                5,
            ))));
            
            Mutex::new(shapes)
        });
    }
    
    pub fn cleanup() {
        if let Some(shapes) = SHAPES.get() {
            let mut shapes = shapes.lock().unwrap();
            shapes.clear();
        }
    }
    
    pub fn get(shape_type: ShapeType) -> Option<&'static Shape> {
        let shapes = SHAPES.get()?;
        let shapes = shapes.lock().unwrap();
        shapes[shape_type as usize].as_ref().map(|s| &**s)
    }
    
    pub fn equals(s1: &Shape, s2: &Shape) -> bool {
        std::ptr::eq(s1, s2)
    }
}
