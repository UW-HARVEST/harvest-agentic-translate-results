// shape.rs — translation of c_src/src/shape.c
//
// Singletons are stored as Box<Shape> on the heap to mirror the C version's
// `malloc(sizeof(shape_t))`. Their pointers are exposed (as `usize`) so we can
// reproduce C's `printf("%p", ...)` output and pointer-equality semantics.

use std::cell::RefCell;

pub const MAX_SHAPE_NAME: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

    pub fn to_i32(self) -> i32 {
        self as i32
    }
}

pub struct Shape {
    pub stype: ShapeType,
    pub name: String,
    pub art: Vec<String>,
    pub width: i32,
    pub height: i32,
}

impl Shape {
    fn new(stype: ShapeType, name: &str, height: i32, width: i32, art: &[&str]) -> Self {
        Shape {
            stype,
            name: name.to_string(),
            art: art.iter().map(|s| s.to_string()).collect(),
            width,
            height,
        }
    }
}

thread_local! {
    static SHAPES: RefCell<Vec<Option<Box<Shape>>>> =
        RefCell::new(Vec::new());
}

pub fn shape_manager_init() {
    SHAPES.with(|cell| {
        let mut v = cell.borrow_mut();
        v.clear();
        // Allocate each shape once (singleton pattern).
        // Order matters because the C code does this same sequence of
        // mallocs, which lets pointer-printing output be deterministic
        // relative to allocator behavior. We use Box::new in the same order.
        v.push(Some(Box::new(Shape::new(
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
        ))));
        v.push(Some(Box::new(Shape::new(
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
        ))));
        v.push(Some(Box::new(Shape::new(
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
        ))));
        v.push(Some(Box::new(Shape::new(
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
        ))));
        v.push(Some(Box::new(Shape::new(
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
        ))));
        v.push(Some(Box::new(Shape::new(
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
        ))));
        v.push(Some(Box::new(Shape::new(
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
        ))));
        v.push(Some(Box::new(Shape::new(
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
        ))));
        v.push(Some(Box::new(Shape::new(
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
        ))));
        v.push(Some(Box::new(Shape::new(
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
        ))));
    });
}

pub fn shape_manager_cleanup() {
    SHAPES.with(|cell| {
        let mut v = cell.borrow_mut();
        for s in v.iter_mut() {
            *s = None;
        }
    });
}

/// Returns the raw pointer (as usize) for the singleton shape of `stype`, or
/// 0 if unavailable. We expose pointers (rather than borrows) because the
/// program needs to print and compare them like C does.
pub fn shape_get_ptr(stype_id: i32) -> usize {
    if stype_id < 0 || stype_id >= SHAPE_COUNT {
        return 0;
    }
    SHAPES.with(|cell| {
        let v = cell.borrow();
        match v.get(stype_id as usize).and_then(|x| x.as_ref()) {
            Some(boxed) => &**boxed as *const Shape as usize,
            None => 0,
        }
    })
}

/// Borrow the shape and call `f` with a reference. Panics if shape is missing.
pub fn with_shape<R>(ptr: usize, f: impl FnOnce(&Shape) -> R) -> Option<R> {
    if ptr == 0 {
        return None;
    }
    // SAFETY: the pointer was obtained from a valid Box stored in the
    // thread-local SHAPES; we only read the shape, and we hold no aliased
    // mutable references because all access to SHAPES is via immutable borrows
    // here. We also keep the Box alive for the program's lifetime (until
    // shape_manager_cleanup is called at exit).
    let shape: &Shape = unsafe { &*(ptr as *const Shape) };
    Some(f(shape))
}

pub fn shape_print(ptr: usize) {
    if ptr == 0 {
        println!("(null shape)");
        return;
    }
    with_shape(ptr, |shape| {
        println!("{}:", shape.name);
        for i in 0..shape.height as usize {
            println!("{}", shape.art[i]);
        }
    });
}

pub fn shape_equals(p1: usize, p2: usize) -> bool {
    p1 == p2 && p1 != 0
}

pub fn shape_type_name(t: i32) -> &'static str {
    match t {
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

/// Returns the shape's name (for printing), or "(null shape)" if NULL.
pub fn shape_name(ptr: usize) -> String {
    with_shape(ptr, |s| s.name.clone()).unwrap_or_else(|| "(null shape)".to_string())
}

/// Returns the shape's type id (or -1 if invalid).
pub fn shape_type_id(ptr: usize) -> i32 {
    with_shape(ptr, |s| s.stype.to_i32()).unwrap_or(-1)
}

/// Format a pointer the same way `printf("%p", p)` does on glibc/Linux.
/// glibc prints "(nil)" for NULL, otherwise lowercase "0x" prefix + hex.
pub fn fmt_ptr(p: usize) -> String {
    if p == 0 {
        "(nil)".to_string()
    } else {
        format!("0x{:x}", p)
    }
}
