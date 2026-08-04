use std::io::{self, BufRead, Write, Read};
use std::fs::File;
use std::ptr;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

// ============ Constants ============

const MAX_SHAPE_WIDTH: usize = 80;
const MAX_SHAPE_HEIGHT: usize = 30;
const MAX_SHAPE_NAME: usize = 32;
const MAX_SHAPES_IN_SCENE: usize = 50;
const MAX_SCENE_NAME: usize = 64;
const MAX_SCENES: usize = 10;
const SHAPE_COUNT: usize = 10;

// ============ Shape ============

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
enum ShapeType {
    Tree = 0,
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
    fn from_usize(v: usize) -> Option<ShapeType> {
        if v < SHAPE_COUNT {
            Some(unsafe { std::mem::transmute::<usize, ShapeType>(v) })
        } else {
            None
        }
    }
}

struct Shape {
    type_: ShapeType,
    name: String,
    art: Vec<String>,
    height: usize,
}

fn rs_shape_type_name(t: ShapeType) -> &'static str {
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

fn init_shapes() -> Vec<Box<Shape>> {
    let mut shapes: Vec<Box<Shape>> = Vec::with_capacity(SHAPE_COUNT);

    // Tree
    shapes.push(Box::new(Shape {
        type_: ShapeType::Tree, name: "Tree".into(), height: 7,
        art: vec![
            "    /\\    ".into(),
            "   /  \\   ".into(),
            "  /____\\  ".into(),
            "  /    \\  ".into(),
            " /______\\ ".into(),
            "    ||    ".into(),
            "    ||    ".into(),
        ],
    }));

    // Tractor
    shapes.push(Box::new(Shape {
        type_: ShapeType::Tractor, name: "Tractor".into(), height: 6,
        art: vec![
            "      ________     ".into(),
            "     |        |___ ".into(),
            "     |  []  []|   |".into(),
            "  ___|________|___|".into(),
            " /  o        o   \\".into(),
            "|___|        |___| ".into(),
        ],
    }));

    // House
    shapes.push(Box::new(Shape {
        type_: ShapeType::House, name: "House".into(), height: 7,
        art: vec![
            "     /\\     ".into(),
            "    /  \\    ".into(),
            "   /____\\   ".into(),
            "   |    |   ".into(),
            "   | [] |   ".into(),
            "   |    |   ".into(),
            "   |____|   ".into(),
        ],
    }));

    // Sun
    shapes.push(Box::new(Shape {
        type_: ShapeType::Sun, name: "Sun".into(), height: 7,
        art: vec![
            "  \\  |  / ".into(),
            "   \\ | /  ".into(),
            "--- (@) ---".into(),
            "   / | \\  ".into(),
            "  /  |  \\ ".into(),
            "          ".into(),
            "          ".into(),
        ],
    }));

    // Cloud
    shapes.push(Box::new(Shape {
        type_: ShapeType::Cloud, name: "Cloud".into(), height: 4,
        art: vec![
            "   _____       ".into(),
            "  /     \\_    ".into(),
            " /  ___  _\\  ".into(),
            "(__/   \\_)   ".into(),
        ],
    }));

    // Flower
    shapes.push(Box::new(Shape {
        type_: ShapeType::Flower, name: "Flower".into(), height: 7,
        art: vec![
            "  \\|/  ".into(),
            " -(@)- ".into(),
            "  /|\\  ".into(),
            "   |   ".into(),
            "   |   ".into(),
            "  / \\  ".into(),
            " /   \\ ".into(),
        ],
    }));

    // Car
    shapes.push(Box::new(Shape {
        type_: ShapeType::Car, name: "Car".into(), height: 4,
        art: vec![
            "  ____       ".into(),
            " /|_||_\\____ ".into(),
            "( o     o  ) ".into(),
            " -----------  ".into(),
        ],
    }));

    // Star
    shapes.push(Box::new(Shape {
        type_: ShapeType::Star, name: "Star".into(), height: 5,
        art: vec![
            "    *    ".into(),
            "   ***   ".into(),
            "  *****  ".into(),
            " ******* ".into(),
            "*********".into(),
        ],
    }));

    // Heart
    shapes.push(Box::new(Shape {
        type_: ShapeType::Heart, name: "Heart".into(), height: 6,
        art: vec![
            " *** ***  ".into(),
            "*********  ".into(),
            "*********  ".into(),
            " ******* ".into(),
            "  *****  ".into(),
            "   ***   ".into(),
        ],
    }));

    // Rainbow
    shapes.push(Box::new(Shape {
        type_: ShapeType::Rainbow, name: "Rainbow".into(), height: 5,
        art: vec![
            "      _______      ".into(),
            "    /         \\    ".into(),
            "   /           \\   ".into(),
            "  /             \\  ".into(),
            " /               \\ ".into(),
        ],
    }));

    shapes
}

fn rs_shape_print(shape: &Shape) {
    println!("{}:", shape.name);
    for i in 0..shape.height {
        println!("{}", shape.art[i]);
    }
}

fn shape_ptr(shape: &Shape) -> *const Shape {
    shape as *const Shape
}

// ============ Scene ============

struct Scene {
    name: String,
    shape_indices: Vec<usize>, // indices into the shapes array
    shape_count: usize,
}

fn rs_scene_create(name: &str) -> Scene {
    Scene {
        name: name.to_string(),
        shape_indices: Vec::new(),
        shape_count: 0,
    }
}

fn rs_scene_add_shape(scene: &mut Scene, shape_idx: usize) -> i32 {
    if scene.shape_count >= MAX_SHAPES_IN_SCENE {
        eprint!("Error: Scene is full\n");
        return -1;
    }
    scene.shape_indices.push(shape_idx);
    scene.shape_count += 1;
    0
}

fn rs_scene_remove_shape(scene: &mut Scene, index: i32) -> i32 {
    if index < 0 || index as usize >= scene.shape_count {
        return -1;
    }
    scene.shape_indices.remove(index as usize);
    scene.shape_count -= 1;
    0
}

fn rs_scene_print(scene: &Scene, shapes: &[Box<Shape>]) {
    println!("\n=== Scene: {} ===", scene.name);
    println!("Contains {} shape(s)\n", scene.shape_count);

    for i in 0..scene.shape_count {
        println!("Shape #{}:", i + 1);
        rs_shape_print(&shapes[scene.shape_indices[i]]);
        println!();
    }
}

fn rs_scene_equals(s1: &Scene, s2: &Scene) -> bool {
    if s1.shape_count != s2.shape_count {
        return false;
    }
    let mut matched = vec![false; s1.shape_count];
    for i in 0..s1.shape_count {
        let mut found = false;
        for j in 0..s2.shape_count {
            if !matched[j] && s1.shape_indices[i] == s2.shape_indices[j] {
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

fn rs_scene_save(scene: &Scene, filename: &str) -> i32 {
    let file = File::create(filename);
    match file {
        Ok(mut f) => {
            let _ = writeln!(f, "{}", scene.name);
            let _ = writeln!(f, "{}", scene.shape_count);
            for i in 0..scene.shape_count {
                let _ = writeln!(f, "{}", scene.shape_indices[i]);
            }
            println!("Scene saved to '{}'", filename);
            0
        }
        Err(_) => {
            eprint!("Error: Could not open file '{}' for writing\n", filename);
            -1
        }
    }
}

fn rs_scene_load(filename: &str, _shapes: &[Box<Shape>]) -> Option<Scene> {
    let file = File::open(filename);
    match file {
        Ok(f) => {
            let reader = io::BufReader::new(f);
            let mut lines = reader.lines();

            let name = match lines.next() {
                Some(Ok(l)) => l,
                _ => return None,
            };

            let count_line = match lines.next() {
                Some(Ok(l)) => l,
                _ => return None,
            };
            let shape_count: usize = match count_line.trim().parse() {
                Ok(v) => v,
                Err(_) => return None,
            };

            let mut scene = rs_scene_create(&name);
            for _ in 0..shape_count {
                let type_line = match lines.next() {
                    Some(Ok(l)) => l,
                    _ => return None,
                };
                let type_val: usize = match type_line.trim().parse() {
                    Ok(v) => v,
                    Err(_) => return None,
                };
                if type_val < SHAPE_COUNT {
                    rs_scene_add_shape(&mut scene, type_val);
                }
            }
            println!("Scene loaded from '{}'", filename);
            Some(scene)
        }
        Err(_) => {
            eprint!("Error: Could not open file '{}' for reading\n", filename);
            None
        }
    }
}

fn rs_scene_list_shapes(scene: &Scene, shapes: &[Box<Shape>]) {
    println!("\nScene: {}", scene.name);
    println!("Shapes ({}):", scene.shape_count);
    for i in 0..scene.shape_count {
        let s = &shapes[scene.shape_indices[i]];
        println!("  {}. {} (ptr: {:p})", i + 1, s.name, &**s as *const Shape);
    }
}

// ============ Input helpers ============

/// Read a line from stdin (like fgets). Returns None on EOF.
fn read_line() -> Option<String> {
    let mut buf = String::new();
    match io::stdin().read_line(&mut buf) {
        Ok(0) => None,
        Ok(_) => {
            // Remove trailing newline like C's strcspn
            if buf.ends_with('\n') {
                buf.pop();
            }
            Some(buf)
        }
        Err(_) => None,
    }
}

/// Read an int via scanf-like behavior: skip whitespace, read digits.
/// After reading, consume up to and including the next newline (like `while(getchar()!='\n')` in C).
fn scanf_int(stdin_lock: &mut io::StdinLock) -> Option<i32> {
    // Skip whitespace, read optional sign + digits
    let mut buf = Vec::new();
    let mut found_digit = false;

    // We need byte-by-byte reading to match C scanf behavior
    let mut byte = [0u8; 1];
    // Skip leading whitespace
    loop {
        match stdin_lock.read(&mut byte) {
            Ok(0) => return None,
            Ok(_) => {
                let c = byte[0] as char;
                if c.is_ascii_whitespace() {
                    continue;
                } else {
                    buf.push(byte[0]);
                    if c == '-' || c == '+' {
                    } else if c.is_ascii_digit() {
                        found_digit = true;
                    } else {
                        // Not a valid start for an integer
                        // Consume rest of line
                        consume_line(stdin_lock);
                        return None;
                    }
                    break;
                }
            }
            Err(_) => return None,
        }
    }

    // Read remaining digits
    loop {
        match stdin_lock.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                let c = byte[0] as char;
                if c.is_ascii_digit() {
                    buf.push(byte[0]);
                    found_digit = true;
                } else {
                    // If this is newline, we're done consuming
                    if c != '\n' {
                        consume_line(stdin_lock);
                    }
                    break;
                }
            }
            Err(_) => break,
        }
    }

    if !found_digit {
        return None;
    }

    let s = String::from_utf8_lossy(&buf);
    s.parse::<i32>().ok()
}

fn consume_line(stdin_lock: &mut io::StdinLock) {
    let mut byte = [0u8; 1];
    loop {
        match stdin_lock.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

/// Read a line from stdin_lock (like fgets). Returns None on EOF.
fn fgets_line(stdin_lock: &mut io::StdinLock) -> Option<String> {
    let mut buf = String::new();
    match stdin_lock.read_line(&mut buf) {
        Ok(0) => None,
        Ok(_) => {
            if buf.ends_with('\n') {
                buf.pop();
            }
            Some(buf)
        }
        Err(_) => None,
    }
}

// ============ Menu functions ============

fn print_menu() {
    println!();
    println!("=========================================");
    println!("  ASCII ART DRAWING APPLICATION");
    println!("=========================================");
    println!("1. View all available shapes");
    println!("2. Create new scene");
    println!("3. Add shape to scene");
    println!("4. Remove shape from scene");
    println!("5. View scene");
    println!("6. List all scenes");
    println!("7. Save scene");
    println!("8. Load scene");
    println!("9. Compare two shapes");
    println!("10. Compare two scenes");
    println!("11. Delete scene");
    println!("12. Exit");
    println!("=========================================");
    print!("Choice: ");
    let _ = io::stdout().flush();
}

fn view_all_shapes(shapes: &[Box<Shape>]) {
    println!("\n=== Available Shapes ===");
    for i in 0..SHAPE_COUNT {
        print!("\n{}. ", i + 1);
        rs_shape_print(&shapes[i]);
    }
}

fn create_new_scene(scenes: &mut Vec<Option<Scene>>, scene_count: &mut usize, stdin_lock: &mut io::StdinLock) {
    if *scene_count >= MAX_SCENES {
        println!("Error: Maximum scenes reached");
        return;
    }

    print!("Enter scene name: ");
    let _ = io::stdout().flush();
    let name = match fgets_line(stdin_lock) {
        Some(n) => n,
        None => return,
    };

    let idx = *scene_count;
    scenes[idx] = Some(rs_scene_create(&name));
    println!("Scene '{}' created (index {})", name, idx);
    *scene_count += 1;
}

fn add_shape_to_scene(scenes: &mut Vec<Option<Scene>>, scene_count: usize, shapes: &[Box<Shape>], stdin_lock: &mut io::StdinLock) {
    if scene_count == 0 {
        println!("No scenes available. Create a scene first.");
        return;
    }

    print!("Select scene (0-{}): ", scene_count - 1);
    let _ = io::stdout().flush();
    let scene_idx = match scanf_int(stdin_lock) {
        Some(v) => v,
        None => { println!("Invalid input"); return; }
    };

    if scene_idx < 0 || scene_idx as usize >= scene_count {
        println!("Invalid scene index");
        return;
    }

    println!("\nSelect shape to add:");
    for i in 0..SHAPE_COUNT {
        println!("{}. {}", i, rs_shape_type_name(ShapeType::from_usize(i).unwrap()));
    }
    print!("Choice: ");
    let _ = io::stdout().flush();

    let shape_type = match scanf_int(stdin_lock) {
        Some(v) => v,
        None => { println!("Invalid input"); return; }
    };

    if shape_type < 0 || shape_type as usize >= SHAPE_COUNT {
        println!("Invalid shape type");
        return;
    }

    let si = scene_idx as usize;
    let st = shape_type as usize;
    let scene = scenes[si].as_mut().unwrap();
    if rs_scene_add_shape(scene, st) == 0 {
        let s = &shapes[st];
        println!("Shape '{}' added to scene (reusing singleton at {:p})", s.name, &**s as *const Shape);
    } else {
        println!("Error adding shape");
    }
}

fn remove_shape_from_scene(scenes: &mut Vec<Option<Scene>>, scene_count: usize, shapes: &[Box<Shape>], stdin_lock: &mut io::StdinLock) {
    if scene_count == 0 {
        println!("No scenes available");
        return;
    }

    print!("Select scene (0-{}): ", scene_count - 1);
    let _ = io::stdout().flush();
    let scene_idx = match scanf_int(stdin_lock) {
        Some(v) => v,
        None => { println!("Invalid input"); return; }
    };

    if scene_idx < 0 || scene_idx as usize >= scene_count {
        println!("Invalid scene index");
        return;
    }

    let si = scene_idx as usize;
    rs_scene_list_shapes(scenes[si].as_ref().unwrap(), shapes);

    let sc = scenes[si].as_ref().unwrap().shape_count;
    if sc == 0 {
        println!("Scene is empty");
        return;
    }

    print!("Select shape to remove (1-{}): ", sc);
    let _ = io::stdout().flush();
    let shape_idx = match scanf_int(stdin_lock) {
        Some(v) => v,
        None => { println!("Invalid input"); return; }
    };

    let scene = scenes[si].as_mut().unwrap();
    if rs_scene_remove_shape(scene, shape_idx - 1) == 0 {
        println!("Shape removed");
    } else {
        println!("Error removing shape");
    }
}

fn view_scene_fn(scenes: &Vec<Option<Scene>>, scene_count: usize, shapes: &[Box<Shape>], stdin_lock: &mut io::StdinLock) {
    if scene_count == 0 {
        println!("No scenes available");
        return;
    }

    print!("Select scene (0-{}): ", scene_count - 1);
    let _ = io::stdout().flush();
    let scene_idx = match scanf_int(stdin_lock) {
        Some(v) => v,
        None => { println!("Invalid input"); return; }
    };

    if scene_idx < 0 || scene_idx as usize >= scene_count {
        println!("Invalid scene index");
        return;
    }

    rs_scene_print(scenes[scene_idx as usize].as_ref().unwrap(), shapes);
}

fn list_all_scenes(scenes: &Vec<Option<Scene>>, scene_count: usize) {
    println!("\n=== All Scenes ===");
    if scene_count == 0 {
        println!("No scenes created yet");
        return;
    }

    for i in 0..scene_count {
        let sc = scenes[i].as_ref().unwrap();
        println!("{}. {} ({} shapes)", i, sc.name, sc.shape_count);
    }
}

fn save_scene_to_file(scenes: &Vec<Option<Scene>>, scene_count: usize, stdin_lock: &mut io::StdinLock) {
    if scene_count == 0 {
        println!("No scenes available");
        return;
    }

    print!("Select scene (0-{}): ", scene_count - 1);
    let _ = io::stdout().flush();
    let scene_idx = match scanf_int(stdin_lock) {
        Some(v) => v,
        None => { println!("Invalid input"); return; }
    };

    if scene_idx < 0 || scene_idx as usize >= scene_count {
        println!("Invalid scene index");
        return;
    }

    print!("Enter filename: ");
    let _ = io::stdout().flush();
    let filename = match fgets_line(stdin_lock) {
        Some(f) => f,
        None => return,
    };

    rs_scene_save(scenes[scene_idx as usize].as_ref().unwrap(), &filename);
}

fn load_scene_from_file(scenes: &mut Vec<Option<Scene>>, scene_count: &mut usize, shapes: &[Box<Shape>], stdin_lock: &mut io::StdinLock) {
    if *scene_count >= MAX_SCENES {
        println!("Error: Maximum scenes reached");
        return;
    }

    print!("Enter filename: ");
    let _ = io::stdout().flush();
    let filename = match fgets_line(stdin_lock) {
        Some(f) => f,
        None => return,
    };

    if let Some(scene) = rs_scene_load(&filename, shapes) {
        let idx = *scene_count;
        scenes[idx] = Some(scene);
        println!("Scene loaded (index {})", idx);
        *scene_count += 1;
    }
}

fn compare_shapes_fn(shapes: &[Box<Shape>], stdin_lock: &mut io::StdinLock) {
    println!("\nSelect first shape (0-{}):", SHAPE_COUNT - 1);
    for i in 0..SHAPE_COUNT {
        println!("{}. {}", i, rs_shape_type_name(ShapeType::from_usize(i).unwrap()));
    }
    print!("Choice: ");
    let _ = io::stdout().flush();

    let type1 = match scanf_int(stdin_lock) {
        Some(v) => v,
        None => { println!("Invalid input"); return; }
    };

    print!("\nSelect second shape (0-{}): ", SHAPE_COUNT - 1);
    let _ = io::stdout().flush();

    let type2 = match scanf_int(stdin_lock) {
        Some(v) => v,
        None => { println!("Invalid input"); return; }
    };

    if type1 < 0 || type1 as usize >= SHAPE_COUNT || type2 < 0 || type2 as usize >= SHAPE_COUNT {
        println!("Invalid shape type");
        return;
    }

    let t1 = type1 as usize;
    let t2 = type2 as usize;
    let s1 = &shapes[t1];
    let s2 = &shapes[t2];

    let p1: *const Shape = &**s1;
    let p2: *const Shape = &**s2;

    println!("\nShape 1: {} (ptr: {:p})", s1.name, p1);
    println!("Shape 2: {} (ptr: {:p})", s2.name, p2);
    println!("Comparison of pointers: {}", if ptr::eq(p1, p2) { 1 } else { 0 });

    if t1 == t2 {
        println!("Result: Shapes are EQUAL (same instance)");
    } else {
        println!("Result: Shapes are NOT EQUAL (different instances)");
    }
}

fn compare_scenes_fn(scenes: &Vec<Option<Scene>>, scene_count: usize, shapes: &[Box<Shape>], stdin_lock: &mut io::StdinLock) {
    if scene_count < 2 {
        println!("Need at least 2 scenes to compare");
        return;
    }

    print!("Select first scene (0-{}): ", scene_count - 1);
    let _ = io::stdout().flush();
    let idx1 = match scanf_int(stdin_lock) {
        Some(v) => v,
        None => { println!("Invalid input"); return; }
    };

    print!("Select second scene (0-{}): ", scene_count - 1);
    let _ = io::stdout().flush();
    let idx2 = match scanf_int(stdin_lock) {
        Some(v) => v,
        None => { println!("Invalid input"); return; }
    };

    if idx1 < 0 || idx1 as usize >= scene_count || idx2 < 0 || idx2 as usize >= scene_count {
        println!("Invalid scene index");
        return;
    }

    let i1 = idx1 as usize;
    let i2 = idx2 as usize;
    let sc1 = scenes[i1].as_ref().unwrap();
    let sc2 = scenes[i2].as_ref().unwrap();

    println!("\nScene 1: {} ({} shapes)", sc1.name, sc1.shape_count);
    rs_scene_list_shapes(sc1, shapes);

    println!("\nScene 2: {} ({} shapes)", sc2.name, sc2.shape_count);
    rs_scene_list_shapes(sc2, shapes);

    if rs_scene_equals(sc1, sc2) {
        println!("\nResult: Scenes are EQUAL (1:1 correspondence)");
    } else {
        println!("\nResult: Scenes are NOT EQUAL");
    }
}

fn delete_scene(scenes: &mut Vec<Option<Scene>>, scene_count: &mut usize, stdin_lock: &mut io::StdinLock) {
    if *scene_count == 0 {
        println!("No scenes available");
        return;
    }

    print!("Select scene to delete (0-{}): ", *scene_count - 1);
    let _ = io::stdout().flush();
    let scene_idx = match scanf_int(stdin_lock) {
        Some(v) => v,
        None => { println!("Invalid input"); return; }
    };

    if scene_idx < 0 || scene_idx as usize >= *scene_count {
        println!("Invalid scene index");
        return;
    }

    let si = scene_idx as usize;
    // Shift remaining scenes
    for i in si..(*scene_count - 1) {
        scenes.swap(i, i + 1);
    }
    scenes[*scene_count - 1] = None;
    *scene_count -= 1;
    println!("Scene deleted");
}

fn main() {
    println!("╔════════════════════════════════════════╗");
    println!("║  ASCII ART DRAWING APPLICATION        ║");
    println!("║  Child-Friendly Shape Editor           ║");
    println!("╚════════════════════════════════════════╝");

    let shapes = init_shapes();

    let mut scenes: Vec<Option<Scene>> = (0..MAX_SCENES).map(|_| None).collect();
    let mut scene_count: usize = 0;

    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();

    loop {
        print_menu();

        let mut input = String::new();
        match stdin_lock.read_line(&mut input) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }

        let choice: i32 = match input.trim().parse() {
            Ok(v) => v,
            Err(_) => { println!("Invalid input"); continue; }
        };

        match choice {
            1 => view_all_shapes(&shapes),
            2 => create_new_scene(&mut scenes, &mut scene_count, &mut stdin_lock),
            3 => add_shape_to_scene(&mut scenes, scene_count, &shapes, &mut stdin_lock),
            4 => remove_shape_from_scene(&mut scenes, scene_count, &shapes, &mut stdin_lock),
            5 => view_scene_fn(&scenes, scene_count, &shapes, &mut stdin_lock),
            6 => list_all_scenes(&scenes, scene_count),
            7 => save_scene_to_file(&scenes, scene_count, &mut stdin_lock),
            8 => load_scene_from_file(&mut scenes, &mut scene_count, &shapes, &mut stdin_lock),
            9 => compare_shapes_fn(&shapes, &mut stdin_lock),
            10 => compare_scenes_fn(&scenes, scene_count, &shapes, &mut stdin_lock),
            11 => delete_scene(&mut scenes, &mut scene_count, &mut stdin_lock),
            12 => {
                println!("\nCleaning up and exiting...");
                println!("Goodbye!");
                return;
            }
            _ => { println!("Invalid choice"); }
        }
    }
}

// ============ FFI repr(C) types matching C layout ============

#[repr(C)]
pub struct FfiShape {
    type_: c_int, // shape_type_t is an enum = int in C
    name: [c_char; MAX_SHAPE_NAME],
    art: [[c_char; MAX_SHAPE_WIDTH]; MAX_SHAPE_HEIGHT],
    width: c_int,
    height: c_int,
}

#[repr(C)]
pub struct FfiScene {
    name: [c_char; MAX_SCENE_NAME],
    shapes: [*mut FfiShape; MAX_SHAPES_IN_SCENE],
    shape_count: c_int,
}

// Global singleton shapes for FFI, mirroring C's static shape_t *shapes[SHAPE_COUNT]
static mut FFI_SHAPES: [*mut FfiShape; SHAPE_COUNT] = [std::ptr::null_mut(); SHAPE_COUNT];

fn str_to_fixed_chars<const N: usize>(s: &str) -> [c_char; N] {
    let mut buf = [0i8; N];
    for (i, b) in s.bytes().enumerate() {
        if i >= N - 1 { break; }
        buf[i] = b as c_char;
    }
    buf
}

fn fill_ffi_shape(ffi: &mut FfiShape, shape: &Shape) {
    ffi.type_ = shape.type_ as c_int;
    ffi.name = str_to_fixed_chars::<MAX_SHAPE_NAME>(&shape.name);
    ffi.width = 0; // will set below
    ffi.height = shape.height as c_int;
    ffi.art = [[0i8; MAX_SHAPE_WIDTH]; MAX_SHAPE_HEIGHT];
    for (i, line) in shape.art.iter().enumerate() {
        if i >= MAX_SHAPE_HEIGHT { break; }
        for (j, b) in line.bytes().enumerate() {
            if j >= MAX_SHAPE_WIDTH - 1 { break; }
            ffi.art[i][j] = b as c_char;
        }
    }
    // Set width from the Shape's art (C code sets width per shape)
    // We need to match the C init_* functions' width values
    ffi.width = match shape.type_ {
        ShapeType::Tree => 11,
        ShapeType::Tractor => 20,
        ShapeType::House => 13,
        ShapeType::Sun => 11,
        ShapeType::Cloud => 16,
        ShapeType::Flower => 9,
        ShapeType::Car => 16,
        ShapeType::Star => 9,
        ShapeType::Heart => 11,
        ShapeType::Rainbow => 21,
    };
}

#[no_mangle]
pub extern "C" fn shape_manager_init() {
    let shapes = init_shapes();
    unsafe {
        for (i, shape) in shapes.iter().enumerate() {
            let ffi = Box::new(std::mem::zeroed::<FfiShape>());
            let ptr = Box::into_raw(ffi);
            fill_ffi_shape(&mut *ptr, shape);
            FFI_SHAPES[i] = ptr;
        }
    }
}

#[no_mangle]
pub extern "C" fn shape_manager_cleanup() {
    unsafe {
        for i in 0..SHAPE_COUNT {
            if !FFI_SHAPES[i].is_null() {
                drop(Box::from_raw(FFI_SHAPES[i]));
                FFI_SHAPES[i] = std::ptr::null_mut();
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn shape_get(type_: c_int) -> *mut FfiShape {
    if type_ < 0 || type_ as usize >= SHAPE_COUNT {
        return std::ptr::null_mut();
    }
    unsafe { FFI_SHAPES[type_ as usize] }
}

#[no_mangle]
pub extern "C" fn shape_print(shape: *const FfiShape) {
    if shape.is_null() {
        print!("(null shape)\n");
        return;
    }
    unsafe {
        let name = CStr::from_ptr((*shape).name.as_ptr()).to_str().unwrap_or("");
        print!("{}:\n", name);
        for i in 0..(*shape).height as usize {
            let line = CStr::from_ptr((*shape).art[i].as_ptr()).to_str().unwrap_or("");
            print!("{}\n", line);
        }
    }
}

#[no_mangle]
pub extern "C" fn shape_equals(s1: *const FfiShape, s2: *const FfiShape) -> c_int {
    if s1 == s2 { 1 } else { 0 }
}

// Static storage for shape_type_name return values (matching C's returning string literals)
static SHAPE_TYPE_NAMES: [&[u8]; 11] = [
    b"Tree\0", b"Tractor\0", b"House\0", b"Sun\0", b"Cloud\0",
    b"Flower\0", b"Car\0", b"Star\0", b"Heart\0", b"Rainbow\0",
    b"Unknown\0",
];

#[no_mangle]
pub extern "C" fn shape_type_name(type_: c_int) -> *const c_char {
    let idx = if type_ >= 0 && (type_ as usize) < SHAPE_COUNT {
        type_ as usize
    } else {
        10 // "Unknown"
    };
    SHAPE_TYPE_NAMES[idx].as_ptr() as *const c_char
}

#[no_mangle]
pub extern "C" fn scene_create(name: *const c_char) -> *mut FfiScene {
    unsafe {
        let scene = Box::new(std::mem::zeroed::<FfiScene>());
        let ptr = Box::into_raw(scene);
        if !name.is_null() {
            let cname = CStr::from_ptr(name).to_bytes();
            let len = cname.len().min(MAX_SCENE_NAME - 1);
            for i in 0..len {
                (*ptr).name[i] = cname[i] as c_char;
            }
            (*ptr).name[len] = 0;
        } else {
            let default = b"Untitled Scene\0";
            for (i, &b) in default.iter().enumerate() {
                (*ptr).name[i] = b as c_char;
            }
        }
        (*ptr).shape_count = 0;
        ptr
    }
}

#[no_mangle]
pub extern "C" fn scene_destroy(scene: *mut FfiScene) {
    if !scene.is_null() {
        unsafe { drop(Box::from_raw(scene)); }
    }
}

#[no_mangle]
pub extern "C" fn scene_add_shape(scene: *mut FfiScene, shape: *mut FfiShape) -> c_int {
    if scene.is_null() || shape.is_null() {
        return -1;
    }
    unsafe {
        if (*scene).shape_count >= MAX_SHAPES_IN_SCENE as c_int {
            eprint!("Error: Scene is full\n");
            return -1;
        }
        let idx = (*scene).shape_count as usize;
        (*scene).shapes[idx] = shape;
        (*scene).shape_count += 1;
        0
    }
}

#[no_mangle]
pub extern "C" fn scene_remove_shape(scene: *mut FfiScene, index: c_int) -> c_int {
    if scene.is_null() {
        return -1;
    }
    unsafe {
        if index < 0 || index >= (*scene).shape_count {
            return -1;
        }
        let idx = index as usize;
        let count = (*scene).shape_count as usize;
        for i in idx..count - 1 {
            (*scene).shapes[i] = (*scene).shapes[i + 1];
        }
        (*scene).shape_count -= 1;
        0
    }
}

#[no_mangle]
pub extern "C" fn scene_print(scene: *const FfiScene) {
    if scene.is_null() {
        print!("(null scene)\n");
        return;
    }
    unsafe {
        let name = CStr::from_ptr((*scene).name.as_ptr()).to_str().unwrap_or("");
        print!("\n=== Scene: {} ===\n", name);
        print!("Contains {} shape(s)\n\n", (*scene).shape_count);
        for i in 0..(*scene).shape_count as usize {
            print!("Shape #{}:\n", i + 1);
            shape_print((*scene).shapes[i]);
            print!("\n");
        }
    }
}

#[no_mangle]
pub extern "C" fn scene_equals(s1: *const FfiScene, s2: *const FfiScene) -> c_int {
    if s1.is_null() || s2.is_null() {
        return 0;
    }
    unsafe {
        if (*s1).shape_count != (*s2).shape_count {
            return 0;
        }
        let count = (*s1).shape_count as usize;
        let mut matched = [false; MAX_SHAPES_IN_SCENE];
        for i in 0..count {
            let mut found = false;
            for j in 0..count {
                if !matched[j] && shape_equals((*s1).shapes[i], (*s2).shapes[j]) == 1 {
                    matched[j] = true;
                    found = true;
                    break;
                }
            }
            if !found {
                return 0;
            }
        }
        1
    }
}

#[no_mangle]
pub extern "C" fn scene_save(scene: *const FfiScene, filename: *const c_char) -> c_int {
    if scene.is_null() || filename.is_null() {
        return -1;
    }
    unsafe {
        let fname = CStr::from_ptr(filename).to_str().unwrap_or("");
        let file = File::create(fname);
        match file {
            Ok(mut f) => {
                let name = CStr::from_ptr((*scene).name.as_ptr()).to_str().unwrap_or("");
                let _ = writeln!(f, "{}", name);
                let _ = writeln!(f, "{}", (*scene).shape_count);
                for i in 0..(*scene).shape_count as usize {
                    let shape = (*scene).shapes[i];
                    let _ = writeln!(f, "{}", (*shape).type_);
                }
                print!("Scene saved to '{}'\n", fname);
                0
            }
            Err(_) => {
                eprint!("Error: Could not open file '{}' for writing\n", fname);
                -1
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn scene_load(filename: *const c_char) -> *mut FfiScene {
    if filename.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let fname = CStr::from_ptr(filename).to_str().unwrap_or("");
        let file = match File::open(fname) {
            Ok(f) => f,
            Err(_) => {
                eprint!("Error: Could not open file '{}' for reading\n", fname);
                return std::ptr::null_mut();
            }
        };
        let reader = io::BufReader::new(file);
        let mut lines = reader.lines();

        let name = match lines.next() {
            Some(Ok(l)) => l,
            _ => return std::ptr::null_mut(),
        };
        // Remove newline (already done by lines())
        let name_c = CString::new(name).unwrap_or_default();
        let scene = scene_create(name_c.as_ptr());
        if scene.is_null() {
            return std::ptr::null_mut();
        }

        let count_line = match lines.next() {
            Some(Ok(l)) => l,
            _ => { scene_destroy(scene); return std::ptr::null_mut(); }
        };
        let shape_count: i32 = match count_line.trim().parse() {
            Ok(v) => v,
            Err(_) => { scene_destroy(scene); return std::ptr::null_mut(); }
        };

        for _ in 0..shape_count {
            let type_line = match lines.next() {
                Some(Ok(l)) => l,
                _ => { scene_destroy(scene); return std::ptr::null_mut(); }
            };
            let type_val: i32 = match type_line.trim().parse() {
                Ok(v) => v,
                Err(_) => { scene_destroy(scene); return std::ptr::null_mut(); }
            };
            let shape = shape_get(type_val);
            if !shape.is_null() {
                scene_add_shape(scene, shape);
            }
        }
        print!("Scene loaded from '{}'\n", fname);
        scene
    }
}

#[no_mangle]
pub extern "C" fn scene_list_shapes(scene: *const FfiScene) {
    if scene.is_null() {
        print!("(null scene)\n");
        return;
    }
    unsafe {
        let name = CStr::from_ptr((*scene).name.as_ptr()).to_str().unwrap_or("");
        print!("\nScene: {}\n", name);
        print!("Shapes ({}):\n", (*scene).shape_count);
        for i in 0..(*scene).shape_count as usize {
            let shape = (*scene).shapes[i];
            let sname = CStr::from_ptr((*shape).name.as_ptr()).to_str().unwrap_or("");
            print!("  {}. {} (ptr: {:p})\n", i + 1, sname, shape);
        }
    }
}
