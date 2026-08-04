
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> core::ffi::c_int {
    // Stub implementation - actual logic to be implemented later
    0
}




use std::io::{self, Write, BufRead};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub enum shape_type_t {
    SHAPE_TREE = 0,
    SHAPE_TRACTOR = 1,
    SHAPE_HOUSE = 2,
    SHAPE_SUN = 3,
    SHAPE_CLOUD = 4,
    SHAPE_FLOWER = 5,
    SHAPE_CAR = 6,
    SHAPE_STAR = 7,
    SHAPE_HEART = 8,
    SHAPE_RAINBOW = 9,
    SHAPE_COUNT = 10,
}

pub const RUST_SHAPE_COUNT: usize = 10;
pub const RUST_MAX_SHAPE_NAME: usize = 32;
pub const RUST_MAX_SHAPE_HEIGHT: usize = 16;
pub const RUST_MAX_SHAPE_WIDTH: usize = 32;
pub const RUST_MAX_SCENE_NAME: usize = 64;
pub const RUST_MAX_SHAPES_IN_SCENE: usize = 20;
pub const RUST_MAX_SCENES: usize = 10;

#[derive(Clone)]
pub struct shape_t {
    pub type_: shape_type_t,
    pub name: String,
    pub art: Vec<String>,
    pub width: i32,
    pub height: i32,
}

pub struct scene_t {
    pub name: String,
    pub shapes: Vec<Rc<shape_t>>,
    pub shape_count: i32,
}

unsafe extern "C" {
    static mut shapes: [*mut core::ffi::c_void; RUST_SHAPE_COUNT];
    static mut scene_count: core::ffi::c_int;
    static mut scenes: [*mut core::ffi::c_void; RUST_MAX_SCENES];
}

pub fn rust_get_shapes() -> [*mut core::ffi::c_void; RUST_SHAPE_COUNT] {
    unsafe { shapes }
}
pub fn rust_set_shapes(val: [*mut core::ffi::c_void; RUST_SHAPE_COUNT]) {
    unsafe { shapes = val; }
}
pub fn rust_get_scene_count() -> core::ffi::c_int {
    unsafe { scene_count }
}
pub fn rust_set_scene_count(val: core::ffi::c_int) {
    unsafe { scene_count = val; }
}
pub fn rust_get_scenes() -> [*mut core::ffi::c_void; RUST_MAX_SCENES] {
    unsafe { scenes }
}
pub fn rust_set_scenes(val: [*mut core::ffi::c_void; RUST_MAX_SCENES]) {
    unsafe { scenes = val; }
}

thread_local! {
    static RUST_SHAPES: RefCell<Vec<Option<Rc<shape_t>>>> = RefCell::new(vec![None; RUST_SHAPE_COUNT]);
    static RUST_SCENES: RefCell<Vec<scene_t>> = RefCell::new(Vec::new());
}

pub fn rust_scene_add_shape(scene: &mut scene_t, shape: Rc<shape_t>) -> Result<(), String> {
    if scene.shape_count as usize >= RUST_MAX_SHAPES_IN_SCENE {
        eprintln!("Error: Scene is full");
        return Err("Scene is full".to_string());
    }
    scene.shapes.push(shape);
    scene.shape_count += 1;
    Ok(())
}

pub fn rust_shape_get(type_: i32) -> Option<Rc<shape_t>> {
    if !(0..RUST_SHAPE_COUNT as i32).contains(&type_) {
        return None;
    }
    RUST_SHAPES.with(|s| s.borrow()[type_ as usize].clone())
}

pub fn rust_shape_type_name(type_: i32) -> &'static str {
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

fn read_line_stdin() -> Option<String> {
    let mut line = String::new();
    match io::stdin().lock().read_line(&mut line) {
        Ok(0) => None,
        Ok(_) => Some(line),
        Err(_) => None,
    }
}

fn read_int() -> Option<i32> {
    read_line_stdin()?.trim().parse::<i32>().ok()
}

fn prompt(msg: &str) {
    print!("{}", msg);
    let _ = io::stdout().flush();
}

pub fn rust_add_shape_to_scene() {
    let sc_count = rust_get_scene_count();
    if sc_count == 0 {
        println!("No scenes available. Create a scene first.");
        return;
    }

    prompt(&format!("Select scene (0-{}): ", sc_count - 1));
    let scene_idx = match read_int() {
        Some(v) if (0..sc_count).contains(&v) => v,
        Some(_) => { println!("Invalid scene index"); return; }
        None => { println!("Invalid input"); return; }
    };

    println!("\nSelect shape to add:");
    for i in 0..RUST_SHAPE_COUNT as i32 {
        println!("{}. {}", i, rust_shape_type_name(i));
    }
    prompt("Choice: ");
    let shape_type = match read_int() {
        Some(v) if (0..RUST_SHAPE_COUNT as i32).contains(&v) => v,
        Some(_) => { println!("Invalid shape type"); return; }
        None => { println!("Invalid input"); return; }
    };

    let shape = match rust_shape_get(shape_type) {
        Some(s) => s,
        None => { println!("Error adding shape"); return; }
    };
    let shape_ptr_str = format!("{:p}", Rc::as_ptr(&shape));
    let shape_name = shape.name.clone();

    let result = RUST_SCENES.with(|scenes_cell| {
        let mut scenes_ref = scenes_cell.borrow_mut();
        match scenes_ref.get_mut(scene_idx as usize) {
            Some(scene) => rust_scene_add_shape(scene, shape.clone()),
            None => Err("Scene not found".to_string()),
        }
    });

    match result {
        Ok(()) => println!(
            "Shape '{}' added to scene (reusing singleton at {})",
            shape_name, shape_ptr_str
        ),
        Err(_) => println!("Error adding shape"),
    }
}

pub fn rust_shape_equals(s1: Option<&Rc<shape_t>>, s2: Option<&Rc<shape_t>>) -> bool {
    matches!((s1, s2), (Some(a), Some(b)) if Rc::ptr_eq(a, b))
}

pub fn rust_scene_equals(s1: &scene_t, s2: &scene_t) -> bool {
    if s1.shape_count != s2.shape_count {
        return false;
    }
    let mut matched = vec![false; RUST_MAX_SHAPES_IN_SCENE];
    for i in 0..s1.shape_count as usize {
        let a = s1.shapes.get(i);
        let mut found = false;
        for j in 0..s2.shape_count as usize {
            let b = s2.shapes.get(j);
            if !matched[j] && rust_shape_equals(a, b) {
                matched[j] = true;
                found = true;
                break;
            }
        }
        if !found {
            return false;
        }
    }
    true
}

pub fn rust_scene_list_shapes(scene: &scene_t) {
    println!("\nScene: {}", scene.name);
    println!("Shapes ({}):", scene.shape_count);
    for (i, sh) in scene.shapes.iter().take(scene.shape_count as usize).enumerate() {
        println!("  {}. {} (ptr: {:p})", i + 1, sh.name, Rc::as_ptr(sh));
    }
}

pub fn rust_compare_scenes() {
    let sc_count = rust_get_scene_count();
    if sc_count < 2 {
        println!("Need at least 2 scenes to compare");
        return;
    }

    prompt(&format!("Select first scene (0-{}): ", sc_count - 1));
    let idx1 = match read_int() {
        Some(v) => v,
        None => { println!("Invalid input"); return; }
    };
    prompt(&format!("Select second scene (0-{}): ", sc_count - 1));
    let idx2 = match read_int() {
        Some(v) => v,
        None => { println!("Invalid input"); return; }
    };

    if !(0..sc_count).contains(&idx1) || !(0..sc_count).contains(&idx2) {
        println!("Invalid scene index");
        return;
    }

    RUST_SCENES.with(|scenes_cell| {
        let scenes_ref = scenes_cell.borrow();
        let sc1 = &scenes_ref[idx1 as usize];
        let sc2 = &scenes_ref[idx2 as usize];

        println!("\nScene 1: {} ({} shapes)", sc1.name, sc1.shape_count);
        rust_scene_list_shapes(sc1);

        println!("\nScene 2: {} ({} shapes)", sc2.name, sc2.shape_count);
        rust_scene_list_shapes(sc2);

        if rust_scene_equals(sc1, sc2) {
            println!("\nResult: Scenes are EQUAL (1:1 correspondence)");
        } else {
            println!("\nResult: Scenes are NOT EQUAL");
        }
    });
}

pub fn rust_compare_shapes() {
    println!("\nSelect first shape (0-{}):", RUST_SHAPE_COUNT as i32 - 1);
    for i in 0..RUST_SHAPE_COUNT as i32 {
        println!("{}. {}", i, rust_shape_type_name(i));
    }
    prompt("Choice: ");
    let type1 = match read_int() {
        Some(v) => v,
        None => { println!("Invalid input"); return; }
    };

    prompt(&format!("\nSelect second shape (0-{}): ", RUST_SHAPE_COUNT as i32 - 1));
    let type2 = match read_int() {
        Some(v) => v,
        None => { println!("Invalid input"); return; }
    };

    let range = 0..RUST_SHAPE_COUNT as i32;
    if !range.contains(&type1) || !range.contains(&type2) {
        println!("Invalid shape type");
        return;
    }

    let s1 = rust_shape_get(type1);
    let s2 = rust_shape_get(type2);

    if let Some(s) = &s1 {
        println!("\nShape 1: {} (ptr: {:p})", s.name, Rc::as_ptr(s));
    }
    if let Some(s) = &s2 {
        println!("Shape 2: {} (ptr: {:p})", s.name, Rc::as_ptr(s));
    }

    let ptr_eq = if rust_shape_equals(s1.as_ref(), s2.as_ref()) { 1 } else { 0 };
    println!("Comparison of pointers: {}", ptr_eq);

    if rust_shape_equals(s1.as_ref(), s2.as_ref()) {
        println!("Result: Shapes are EQUAL (same instance)");
    } else {
        println!("Result: Shapes are NOT EQUAL (different instances)");
    }
}

pub fn rust_scene_create(name: Option<&str>) -> scene_t {
    let n = match name {
        Some(s) => s.chars().take(RUST_MAX_SCENE_NAME - 1).collect(),
        None => "Untitled Scene".to_string(),
    };
    scene_t {
        name: n,
        shapes: Vec::with_capacity(RUST_MAX_SHAPES_IN_SCENE),
        shape_count: 0,
    }
}

pub fn rust_create_new_scene() {
    let sc_count = rust_get_scene_count();
    if sc_count as usize >= RUST_MAX_SCENES {
        println!("Error: Maximum scenes reached");
        return;
    }

    prompt("Enter scene name: ");
    let line = match read_line_stdin() {
        Some(l) => l,
        None => return,
    };
    let name: String = line.trim_end_matches(&['\n', '\r'][..]).to_string();
    let scene = rust_scene_create(Some(&name));
    let display_name = scene.name.clone();

    RUST_SCENES.with(|scenes_cell| {
        scenes_cell.borrow_mut().push(scene);
    });

    println!("Scene '{}' created (index {})", display_name, sc_count);
    rust_set_scene_count(sc_count + 1);
}

pub fn rust_scene_destroy(_scene: scene_t) {
    // Drop automatically when going out of scope.
}

pub fn rust_delete_scene() {
    let sc_count = rust_get_scene_count();
    if sc_count == 0 {
        println!("No scenes available");
        return;
    }

    prompt(&format!("Select scene to delete (0-{}): ", sc_count - 1));
    let scene_idx = match read_int() {
        Some(v) if (0..sc_count).contains(&v) => v,
        Some(_) => { println!("Invalid scene index"); return; }
        None => { println!("Invalid input"); return; }
    };

    let removed = RUST_SCENES.with(|scenes_cell| {
        let mut scenes_ref = scenes_cell.borrow_mut();
        scenes_ref.remove(scene_idx as usize)
    });
    rust_scene_destroy(removed);

    rust_set_scene_count(sc_count - 1);
    println!("Scene deleted");
}

fn make_art(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|s| (*s).to_string()).collect()
}

pub fn rust_init_car(shape: &mut shape_t) {
    shape.type_ = shape_type_t::SHAPE_CAR;
    shape.name = "Car".to_string();
    shape.height = 4;
    shape.width = 16;
    shape.art = make_art(&[
        "  ____       ",
        " /|_||_\\____ ",
        "( o     o  ) ",
        " -----------  ",
    ]);
}

pub fn rust_init_cloud(shape: &mut shape_t) {
    shape.type_ = shape_type_t::SHAPE_CLOUD;
    shape.name = "Cloud".to_string();
    shape.height = 4;
    shape.width = 16;
    shape.art = make_art(&[
        "   _____       ",
        "  /     \\_    ",
        " /  ___  _\\  ",
        "(__/   \\_)   ",
    ]);
}

pub fn rust_init_flower(shape: &mut shape_t) {
    shape.type_ = shape_type_t::SHAPE_FLOWER;
    shape.name = "Flower".to_string();
    shape.height = 7;
    shape.width = 9;
    shape.art = make_art(&[
        "  \\|/  ",
        " -(@)- ",
        "  /|\\  ",
        "   |   ",
        "   |   ",
        "  / \\  ",
        " /   \\ ",
    ]);
}

pub fn rust_init_heart(shape: &mut shape_t) {
    shape.type_ = shape_type_t::SHAPE_HEART;
    shape.name = "Heart".to_string();
    shape.height = 6;
    shape.width = 11;
    shape.art = make_art(&[
        " *** ***  ",
        "*********  ",
        "*********  ",
        " ******* ",
        "  *****  ",
        "   ***   ",
    ]);
}

pub fn rust_init_house(shape: &mut shape_t) {
    shape.type_ = shape_type_t::SHAPE_HOUSE;
    shape.name = "House".to_string();
    shape.height = 7;
    shape.width = 13;
    shape.art = make_art(&[
        "     /\\     ",
        "    /  \\    ",
        "   /____\\   ",
        "   |    |   ",
        "   | [] |   ",
        "   |    |   ",
        "   |____|   ",
    ]);
}

pub fn rust_init_rainbow(shape: &mut shape_t) {
    shape.type_ = shape_type_t::SHAPE_RAINBOW;
    shape.name = "Rainbow".to_string();
    shape.height = 5;
    shape.width = 21;
    shape.art = make_art(&[
        "      _______      ",
        "    /         \\    ",
        "   /           \\   ",
        "  /             \\  ",
        " /               \\ ",
    ]);
}

pub fn rust_init_star(shape: &mut shape_t) {
    shape.type_ = shape_type_t::SHAPE_STAR;
    shape.name = "Star".to_string();
    shape.height = 5;
    shape.width = 9;
    shape.art = make_art(&[
        "    *    ",
        "   ***   ",
        "  *****  ",
        " ******* ",
        "*********",
    ]);
}


pub fn rust_init_sun(shape: &mut shape_t) {
    *shape = shape_t {
        type_: shape_type_t::SHAPE_SUN,
        name: "Sun".to_string(),
        height: 7,
        width: 11,
        art: make_art(&[
            "  \\  |  / ",
            "   \\ | /  ",
            "--- (@) ---",
            "   / | \\  ",
            "  /  |  \\ ",
            "          ",
            "          ",
        ]),
    };
}

pub fn rust_init_tractor(shape: &mut shape_t) {
    *shape = shape_t {
        type_: shape_type_t::SHAPE_TRACTOR,
        name: "Tractor".to_string(),
        height: 6,
        width: 20,
        art: make_art(&[
            "      ________     ",
            "     |        |___ ",
            "     |  []  []|   |",
            "  ___|________|___|",
            " /  o        o   \\",
            "|___|        |___| ",
        ]),
    };
}

pub fn rust_init_tree(shape: &mut shape_t) {
    *shape = shape_t {
        type_: shape_type_t::SHAPE_TREE,
        name: "Tree".to_string(),
        height: 7,
        width: 11,
        art: make_art(&[
            "    /\\    ",
            "   /  \\   ",
            "  /____\\  ",
            "  /    \\  ",
            " /______\\ ",
            "    ||    ",
            "    ||    ",
        ]),
    };
}

pub fn rust_list_all_scenes() {
    println!("\n=== All Scenes ===");
    let sc_count = rust_get_scene_count();
    if sc_count == 0 {
        println!("No scenes created yet");
        return;
    }
    RUST_SCENES.with(|scenes_cell| {
        let scenes_ref = scenes_cell.borrow();
        for (i, sc) in scenes_ref.iter().take(sc_count as usize).enumerate() {
            println!("{}. {} ({} shapes)", i, sc.name, sc.shape_count);
        }
    });
}

pub fn rust_scene_load(filename: &str) -> Option<scene_t> {
    if filename.is_empty() {
        return None;
    }
    let contents = std::fs::read_to_string(filename)
        .map_err(|_| {
            eprintln!("Error: Could not open file '{}' for reading", filename);
        })
        .ok()?;

    let mut lines = contents.lines();
    let name = lines.next()?.to_string();
    let mut scene = rust_scene_create(Some(&name));

    let shape_count: i32 = lines.next()?.trim().parse().ok()?;

    for _ in 0..shape_count {
        let type_val: i32 = lines.next()?.trim().parse().ok()?;
        if let Some(shape) = rust_shape_get(type_val) {
            let _ = rust_scene_add_shape(&mut scene, shape);
        }
    }

    println!("Scene loaded from '{}'", filename);
    Some(scene)
}

pub fn rust_load_scene_from_file() {
    let sc_count = rust_get_scene_count();
    if sc_count as usize >= RUST_MAX_SCENES {
        println!("Error: Maximum scenes reached");
        return;
    }
    prompt("Enter filename: ");
    let Some(line) = read_line_stdin() else { return; };
    let filename: String = line.trim_end_matches(&['\n', '\r'][..]).to_string();

    if let Some(scene) = rust_scene_load(&filename) {
        RUST_SCENES.with(|scenes_cell| {
            scenes_cell.borrow_mut().push(scene);
        });
        rust_set_scene_count(sc_count + 1);
        println!("Scene loaded (index {})", sc_count);
    }
}

pub fn rust_print_menu() {
    let menu = [
        "",
        "=========================================",
        "  ASCII ART DRAWING APPLICATION",
        "=========================================",
        "1. View all available shapes",
        "2. Create new scene",
        "3. Add shape to scene",
        "4. Remove shape from scene",
        "5. View scene",
        "6. List all scenes",
        "7. Save scene",
        "8. Load scene",
        "9. Compare two shapes",
        "10. Compare two scenes",
        "11. Delete scene",
        "12. Exit",
        "=========================================",
    ];
    for line in menu.iter() {
        println!("{}", line);
    }
    prompt("Choice: ");
}

pub fn rust_scene_remove_shape(scene: &mut scene_t, index: i32) -> Result<(), ()> {
    if index < 0 || index >= scene.shape_count {
        return Err(());
    }
    scene.shapes.remove(index as usize);
    scene.shape_count -= 1;
    Ok(())
}

pub fn rust_remove_shape_from_scene() {
    let sc_count = rust_get_scene_count();
    if sc_count == 0 {
        println!("No scenes available");
        return;
    }
    prompt(&format!("Select scene (0-{}): ", sc_count - 1));
    let scene_idx = match read_int() {
        Some(v) => v,
        None => { println!("Invalid input"); return; }
    };
    if !(0..sc_count).contains(&scene_idx) {
        println!("Invalid scene index");
        return;
    }

    let (empty, shape_count) = RUST_SCENES.with(|scenes_cell| {
        let scenes_ref = scenes_cell.borrow();
        let sc = &scenes_ref[scene_idx as usize];
        rust_scene_list_shapes(sc);
        (sc.shape_count == 0, sc.shape_count)
    });

    if empty {
        println!("Scene is empty");
        return;
    }

    prompt(&format!("Select shape to remove (1-{}): ", shape_count));
    let shape_idx = match read_int() {
        Some(v) => v,
        None => { println!("Invalid input"); return; }
    };

    let result = RUST_SCENES.with(|scenes_cell| {
        let mut scenes_ref = scenes_cell.borrow_mut();
        rust_scene_remove_shape(&mut scenes_ref[scene_idx as usize], shape_idx - 1)
    });

    match result {
        Ok(()) => println!("Shape removed"),
        Err(()) => println!("Error removing shape"),
    }
}

pub fn rust_scene_save(scene: &scene_t, filename: &str) -> Result<(), ()> {
    if filename.is_empty() {
        return Err(());
    }
    let mut out = String::new();
    out.push_str(&scene.name);
    out.push('\n');
    out.push_str(&format!("{}\n", scene.shape_count));
    for sh in scene.shapes.iter().take(scene.shape_count as usize) {
        out.push_str(&format!("{}\n", sh.type_ as i32));
    }
    match std::fs::write(filename, out) {
        Ok(_) => {
            println!("Scene saved to '{}'", filename);
            Ok(())
        }
        Err(_) => {
            eprintln!("Error: Could not open file '{}' for writing", filename);
            Err(())
        }
    }
}

pub fn rust_save_scene_to_file() {
    let sc_count = rust_get_scene_count();
    if sc_count == 0 {
        println!("No scenes available");
        return;
    }
    prompt(&format!("Select scene (0-{}): ", sc_count - 1));
    let scene_idx = match read_int() {
        Some(v) => v,
        None => { println!("Invalid input"); return; }
    };
    if !(0..sc_count).contains(&scene_idx) {
        println!("Invalid scene index");
        return;
    }
    prompt("Enter filename: ");
    let Some(line) = read_line_stdin() else { return; };
    let filename: String = line.trim_end_matches(&['\n', '\r'][..]).to_string();
    RUST_SCENES.with(|scenes_cell| {
        let scenes_ref = scenes_cell.borrow();
        let _ = rust_scene_save(&scenes_ref[scene_idx as usize], &filename);
    });
}

pub fn rust_shape_manager_cleanup() {
    RUST_SHAPES.with(|s| {
        let mut sh = s.borrow_mut();
        for slot in sh.iter_mut() {
            *slot = None;
        }
    });
}

fn default_shape() -> shape_t {
    shape_t {
        type_: shape_type_t::SHAPE_TREE,
        name: String::new(),
        art: Vec::new(),
        width: 0,
        height: 0,
    }
}

pub fn rust_shape_manager_init() {
    let init_fns: [(shape_type_t, fn(&mut shape_t)); RUST_SHAPE_COUNT] = [
        (shape_type_t::SHAPE_TREE, rust_init_tree),
        (shape_type_t::SHAPE_TRACTOR, rust_init_tractor),
        (shape_type_t::SHAPE_HOUSE, rust_init_house),
        (shape_type_t::SHAPE_SUN, rust_init_sun),
        (shape_type_t::SHAPE_CLOUD, rust_init_cloud),
        (shape_type_t::SHAPE_FLOWER, rust_init_flower),
        (shape_type_t::SHAPE_CAR, rust_init_car),
        (shape_type_t::SHAPE_STAR, rust_init_star),
        (shape_type_t::SHAPE_HEART, rust_init_heart),
        (shape_type_t::SHAPE_RAINBOW, rust_init_rainbow),
    ];

    RUST_SHAPES.with(|s| {
        let mut sh = s.borrow_mut();
        for (ty, f) in init_fns.iter() {
            let mut new_shape = default_shape();
            f(&mut new_shape);
            sh[*ty as usize] = Some(Rc::new(new_shape));
        }
    });
}

pub fn rust_shape_print(shape: Option<&shape_t>) {
    let Some(shape) = shape else {
        println!("(null shape)");
        return;
    };
    println!("{}:", shape.name);
    for line in shape.art.iter().take(shape.height as usize) {
        println!("{}", line);
    }
}

pub fn rust_view_all_shapes() {
    println!("\n=== Available Shapes ===");
    for i in 0..RUST_SHAPE_COUNT as i32 {
        println!("\n{}. ", i + 1);
        let shape = rust_shape_get(i);
        rust_shape_print(shape.as_deref());
    }
}

pub fn rust_scene_print(scene: Option<&scene_t>) {
    let Some(scene) = scene else {
        println!("(null scene)");
        return;
    };
    println!("\n=== Scene: {} ===", scene.name);
    println!("Contains {} shape(s)\n", scene.shape_count);
    for (i, sh) in scene.shapes.iter().take(scene.shape_count as usize).enumerate() {
        println!("Shape #{}:", i + 1);
        rust_shape_print(Some(sh.as_ref()));
        println!();
    }
}

pub fn rust_view_scene() {
    let sc_count = rust_get_scene_count();
    if sc_count == 0 {
        println!("No scenes available");
        return;
    }
    prompt(&format!("Select scene (0-{}): ", sc_count - 1));
    let scene_idx = match read_int() {
        Some(v) => v,
        None => { println!("Invalid input"); return; }
    };
    if !(0..sc_count).contains(&scene_idx) {
        println!("Invalid scene index");
        return;
    }
    RUST_SCENES.with(|scenes_cell| {
        let scenes_ref = scenes_cell.borrow();
        rust_scene_print(scenes_ref.get(scene_idx as usize));
    });
}

fn cleanup_all() {
    RUST_SCENES.with(|s| s.borrow_mut().clear());
    rust_set_scene_count(0);
    rust_shape_manager_cleanup();
}

#[unsafe(no_mangle)]
pub extern "C" fn main_pointer_comparison_ascii_art_main() -> core::ffi::c_int {
    println!("╔════════════════════════════════════════╗");
    println!("║  ASCII ART DRAWING APPLICATION        ║");
    println!("║  Child-Friendly Shape Editor           ║");
    println!("╚════════════════════════════════════════╝");

    rust_shape_manager_init();

    while let Some(input) = read_line_stdin() {
        rust_print_menu();
        let choice: i32 = match input.trim().parse() {
            Ok(v) => v,
            Err(_) => {
                println!("Invalid input");
                continue;
            }
        };

        match choice {
            1 => rust_view_all_shapes(),
            2 => rust_create_new_scene(),
            3 => rust_add_shape_to_scene(),
            4 => rust_remove_shape_from_scene(),
            5 => rust_view_scene(),
            6 => rust_list_all_scenes(),
            7 => rust_save_scene_to_file(),
            8 => rust_load_scene_from_file(),
            9 => rust_compare_shapes(),
            10 => rust_compare_scenes(),
            11 => rust_delete_scene(),
            12 => {
                println!("\nCleaning up and exiting...");
                cleanup_all();
                println!("Goodbye!");
                return 0;
            }
            _ => println!("Invalid choice"),
        }
    }

    cleanup_all();
    0
}
