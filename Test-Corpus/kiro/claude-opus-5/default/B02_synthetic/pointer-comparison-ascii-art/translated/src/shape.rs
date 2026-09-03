// Translation of shape.c / shape.h
//
// The C code allocates one `shape_t` per shape type with malloc and hands out
// those singleton pointers; identity comparison and `%p` printing of those
// pointers are part of the observable behaviour, so the layout of `Shape`
// mirrors the C struct byte for byte and one heap allocation is made per shape.

use crate::cio::Out;

pub const MAX_SHAPE_WIDTH: usize = 80;
pub const MAX_SHAPE_HEIGHT: usize = 30;
pub const MAX_SHAPE_NAME: usize = 32;

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

#[repr(C)]
pub struct Shape {
    pub stype: i32,
    pub name: [u8; MAX_SHAPE_NAME],
    pub art: [[u8; MAX_SHAPE_WIDTH]; MAX_SHAPE_HEIGHT],
    pub width: i32,
    pub height: i32,
}

impl Shape {
    fn blank() -> Shape {
        Shape {
            stype: 0,
            name: [0u8; MAX_SHAPE_NAME],
            art: [[0u8; MAX_SHAPE_WIDTH]; MAX_SHAPE_HEIGHT],
            width: 0,
            height: 0,
        }
    }

    /// strcpy(shape->name, name)
    fn set_name(&mut self, name: &str) {
        let b = name.as_bytes();
        self.name[..b.len()].copy_from_slice(b);
        self.name[b.len()] = 0;
    }

    /// strcpy(shape->art[row], art)
    fn set_art(&mut self, row: usize, art: &str) {
        let b = art.as_bytes();
        self.art[row][..b.len()].copy_from_slice(b);
        self.art[row][b.len()] = 0;
    }

    fn init(&mut self, stype: i32, name: &str, height: i32, width: i32, art: &[&str]) {
        self.stype = stype;
        self.set_name(name);
        self.height = height;
        self.width = width;
        for (i, line) in art.iter().enumerate() {
            self.set_art(i, line);
        }
    }
}

pub struct ShapeManager {
    shapes: Vec<Option<Box<Shape>>>,
}

impl ShapeManager {
    /// shape_manager_init()
    pub fn init() -> ShapeManager {
        // Reserve the table first so the ten shape allocations are made
        // back-to-back, mirroring the C loop's malloc sequence (and therefore
        // the relative spacing of the pointers that get printed with %p).
        let mut shapes: Vec<Option<Box<Shape>>> = Vec::with_capacity(SHAPE_COUNT as usize);
        for _ in 0..SHAPE_COUNT {
            shapes.push(Some(Box::new(Shape::blank())));
        }
        let mut mgr = ShapeManager { shapes };

        mgr.at(SHAPE_TREE).init(
            SHAPE_TREE,
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
        );
        mgr.at(SHAPE_TRACTOR).init(
            SHAPE_TRACTOR,
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
        );
        mgr.at(SHAPE_HOUSE).init(
            SHAPE_HOUSE,
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
        );
        mgr.at(SHAPE_SUN).init(
            SHAPE_SUN,
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
        );
        mgr.at(SHAPE_CLOUD).init(
            SHAPE_CLOUD,
            "Cloud",
            4,
            16,
            &[
                "   _____       ",
                "  /     \\_    ",
                " /  ___  _\\  ",
                "(__/   \\_)   ",
            ],
        );
        mgr.at(SHAPE_FLOWER).init(
            SHAPE_FLOWER,
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
        );
        mgr.at(SHAPE_CAR).init(
            SHAPE_CAR,
            "Car",
            4,
            16,
            &[
                "  ____       ",
                " /|_||_\\____ ",
                "( o     o  ) ",
                " -----------  ",
            ],
        );
        mgr.at(SHAPE_STAR).init(
            SHAPE_STAR,
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
        );
        mgr.at(SHAPE_HEART).init(
            SHAPE_HEART,
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
        );
        mgr.at(SHAPE_RAINBOW).init(
            SHAPE_RAINBOW,
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
        );

        mgr
    }

    fn at(&mut self, t: i32) -> &mut Shape {
        self.shapes[t as usize].as_mut().unwrap()
    }

    /// shape_manager_cleanup()
    pub fn cleanup(&mut self) {
        for slot in self.shapes.iter_mut() {
            *slot = None;
        }
    }

    /// shape_get(type): None models the NULL return.
    pub fn get(&self, t: i32) -> Option<&Shape> {
        if t < 0 || t >= SHAPE_COUNT {
            return None;
        }
        match &self.shapes[t as usize] {
            Some(b) => Some(&**b),
            None => None,
        }
    }

    /// The value printf("%p") would print for the singleton of this type.
    pub fn ptr_of(&self, t: i32) -> usize {
        match self.get(t) {
            Some(s) => s as *const Shape as usize,
            None => 0,
        }
    }
}

/// shape_print()
pub fn shape_print(out: &mut Out, shape: Option<&Shape>) {
    let shape = match shape {
        None => {
            out.put("(null shape)\n");
            return;
        }
        Some(s) => s,
    };

    out.put_bytes(crate::cio::c_str_bytes(&shape.name));
    out.put(":\n");
    for i in 0..shape.height {
        out.put_bytes(crate::cio::c_str_bytes(&shape.art[i as usize]));
        out.put("\n");
    }
}

/// shape_type_name()
pub fn shape_type_name(t: i32) -> &'static str {
    match t {
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
