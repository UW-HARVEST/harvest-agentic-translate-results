#[allow(dead_code)]
pub const MAX_SHAPE_WIDTH: usize = 80;
#[allow(dead_code)]
pub const MAX_SHAPE_HEIGHT: usize = 30;
#[allow(dead_code)]
pub const MAX_SHAPE_NAME: usize = 32;

pub const SHAPE_COUNT: usize = 10;

pub const SHAPE_TREE: usize = 0;
pub const SHAPE_TRACTOR: usize = 1;
pub const SHAPE_HOUSE: usize = 2;
pub const SHAPE_SUN: usize = 3;
pub const SHAPE_CLOUD: usize = 4;
pub const SHAPE_FLOWER: usize = 5;
pub const SHAPE_CAR: usize = 6;
pub const SHAPE_STAR: usize = 7;
pub const SHAPE_HEART: usize = 8;
pub const SHAPE_RAINBOW: usize = 9;

pub struct Shape {
    pub shape_type: usize,
    pub name: String,
    pub art: Vec<String>,
    #[allow(dead_code)]
    pub width: i32,
    pub height: i32,
}

static mut SHAPES: [*mut Shape; SHAPE_COUNT] = [std::ptr::null_mut(); SHAPE_COUNT];

fn init_tree() -> Shape {
    Shape {
        shape_type: SHAPE_TREE,
        name: "Tree".into(),
        width: 11,
        height: 7,
        art: vec![
            "    /\\    ".into(),
            "   /  \\   ".into(),
            "  /____\\  ".into(),
            "  /    \\  ".into(),
            " /______\\ ".into(),
            "    ||    ".into(),
            "    ||    ".into(),
        ],
    }
}

fn init_tractor() -> Shape {
    Shape {
        shape_type: SHAPE_TRACTOR,
        name: "Tractor".into(),
        width: 20,
        height: 6,
        art: vec![
            "      ________     ".into(),
            "     |        |___ ".into(),
            "     |  []  []|   |".into(),
            "  ___|________|___|".into(),
            " /  o        o   \\".into(),
            "|___|        |___| ".into(),
        ],
    }
}

fn init_house() -> Shape {
    Shape {
        shape_type: SHAPE_HOUSE,
        name: "House".into(),
        width: 13,
        height: 7,
        art: vec![
            "     /\\     ".into(),
            "    /  \\    ".into(),
            "   /____\\   ".into(),
            "   |    |   ".into(),
            "   | [] |   ".into(),
            "   |    |   ".into(),
            "   |____|   ".into(),
        ],
    }
}

fn init_sun() -> Shape {
    Shape {
        shape_type: SHAPE_SUN,
        name: "Sun".into(),
        width: 11,
        height: 7,
        art: vec![
            "  \\  |  / ".into(),
            "   \\ | /  ".into(),
            "--- (@) ---".into(),
            "   / | \\  ".into(),
            "  /  |  \\ ".into(),
            "          ".into(),
            "          ".into(),
        ],
    }
}

fn init_cloud() -> Shape {
    Shape {
        shape_type: SHAPE_CLOUD,
        name: "Cloud".into(),
        width: 16,
        height: 4,
        art: vec![
            "   _____       ".into(),
            "  /     \\_    ".into(),
            " /  ___  _\\  ".into(),
            "(__/   \\_)   ".into(),
        ],
    }
}

fn init_flower() -> Shape {
    Shape {
        shape_type: SHAPE_FLOWER,
        name: "Flower".into(),
        width: 9,
        height: 7,
        art: vec![
            "  \\|/  ".into(),
            " -(@)- ".into(),
            "  /|\\  ".into(),
            "   |   ".into(),
            "   |   ".into(),
            "  / \\  ".into(),
            " /   \\ ".into(),
        ],
    }
}

fn init_car() -> Shape {
    Shape {
        shape_type: SHAPE_CAR,
        name: "Car".into(),
        width: 16,
        height: 4,
        art: vec![
            "  ____       ".into(),
            " /|_||_\\____ ".into(),
            "( o     o  ) ".into(),
            " -----------  ".into(),
        ],
    }
}

fn init_star() -> Shape {
    Shape {
        shape_type: SHAPE_STAR,
        name: "Star".into(),
        width: 9,
        height: 5,
        art: vec![
            "    *    ".into(),
            "   ***   ".into(),
            "  *****  ".into(),
            " ******* ".into(),
            "*********".into(),
        ],
    }
}

fn init_heart() -> Shape {
    Shape {
        shape_type: SHAPE_HEART,
        name: "Heart".into(),
        width: 11,
        height: 6,
        art: vec![
            " *** ***  ".into(),
            "*********  ".into(),
            "*********  ".into(),
            " ******* ".into(),
            "  *****  ".into(),
            "   ***   ".into(),
        ],
    }
}

fn init_rainbow() -> Shape {
    Shape {
        shape_type: SHAPE_RAINBOW,
        name: "Rainbow".into(),
        width: 21,
        height: 5,
        art: vec![
            "      _______      ".into(),
            "    /         \\    ".into(),
            "   /           \\   ".into(),
            "  /             \\  ".into(),
            " /               \\ ".into(),
        ],
    }
}

pub fn shape_manager_init() {
    let inits: [fn() -> Shape; SHAPE_COUNT] = [
        init_tree, init_tractor, init_house, init_sun, init_cloud,
        init_flower, init_car, init_star, init_heart, init_rainbow,
    ];
    unsafe {
        for i in 0..SHAPE_COUNT {
            SHAPES[i] = Box::into_raw(Box::new(inits[i]()));
        }
    }
}

pub fn shape_manager_cleanup() {
    unsafe {
        for i in 0..SHAPE_COUNT {
            if !SHAPES[i].is_null() {
                drop(Box::from_raw(SHAPES[i]));
                SHAPES[i] = std::ptr::null_mut();
            }
        }
    }
}

pub fn shape_get(shape_type: usize) -> *mut Shape {
    if shape_type >= SHAPE_COUNT {
        return std::ptr::null_mut();
    }
    unsafe { SHAPES[shape_type] }
}

pub fn shape_print(shape: *const Shape) {
    if shape.is_null() {
        print!("(null shape)\n");
        return;
    }
    let s = unsafe { &*shape };
    print!("{}:\n", s.name);
    for i in 0..s.height as usize {
        print!("{}\n", s.art[i]);
    }
}

pub fn shape_equals(s1: *const Shape, s2: *const Shape) -> bool {
    s1 == s2
}

pub fn shape_type_name(t: usize) -> &'static str {
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

// --- C ABI exports with exact C symbol names ---

#[export_name = "shape_manager_init"]
pub unsafe extern "C" fn _export_shape_manager_init() {
    shape_manager_init();
}

#[export_name = "shape_manager_cleanup"]
pub unsafe extern "C" fn _export_shape_manager_cleanup() {
    shape_manager_cleanup();
}

#[export_name = "shape_get"]
pub unsafe extern "C" fn _export_shape_get(shape_type: std::os::raw::c_int) -> *mut Shape {
    shape_get(shape_type as usize)
}

#[export_name = "shape_print"]
pub unsafe extern "C" fn _export_shape_print(shape: *const Shape) {
    shape_print(shape);
}

#[export_name = "shape_equals"]
pub unsafe extern "C" fn _export_shape_equals(s1: *const Shape, s2: *const Shape) -> std::os::raw::c_int {
    if shape_equals(s1, s2) { 1 } else { 0 }
}

#[export_name = "shape_type_name"]
pub unsafe extern "C" fn _export_shape_type_name(t: std::os::raw::c_int) -> *const std::os::raw::c_char {
    let name = shape_type_name(t as usize);
    name.as_ptr() as *const std::os::raw::c_char
}
