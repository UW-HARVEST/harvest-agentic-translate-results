use std::io::{self, BufRead, Write, Read};
use std::fs::File;
use std::ptr;

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

fn shape_type_name(t: ShapeType) -> &'static str {
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

fn shape_print(shape: &Shape) {
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

fn scene_create(name: &str) -> Scene {
    Scene {
        name: name.to_string(),
        shape_indices: Vec::new(),
        shape_count: 0,
    }
}

fn scene_add_shape(scene: &mut Scene, shape_idx: usize) -> i32 {
    if scene.shape_count >= MAX_SHAPES_IN_SCENE {
        eprint!("Error: Scene is full\n");
        return -1;
    }
    scene.shape_indices.push(shape_idx);
    scene.shape_count += 1;
    0
}

fn scene_remove_shape(scene: &mut Scene, index: i32) -> i32 {
    if index < 0 || index as usize >= scene.shape_count {
        return -1;
    }
    scene.shape_indices.remove(index as usize);
    scene.shape_count -= 1;
    0
}

fn scene_print(scene: &Scene, shapes: &[Box<Shape>]) {
    println!("\n=== Scene: {} ===", scene.name);
    println!("Contains {} shape(s)\n", scene.shape_count);

    for i in 0..scene.shape_count {
        println!("Shape #{}:", i + 1);
        shape_print(&shapes[scene.shape_indices[i]]);
        println!();
    }
}

fn scene_equals(s1: &Scene, s2: &Scene) -> bool {
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

fn scene_save(scene: &Scene, filename: &str) -> i32 {
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

fn scene_load(filename: &str, _shapes: &[Box<Shape>]) -> Option<Scene> {
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

            let mut scene = scene_create(&name);
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
                    scene_add_shape(&mut scene, type_val);
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

fn scene_list_shapes(scene: &Scene, shapes: &[Box<Shape>]) {
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
        shape_print(&shapes[i]);
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
    scenes[idx] = Some(scene_create(&name));
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
        println!("{}. {}", i, shape_type_name(ShapeType::from_usize(i).unwrap()));
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
    if scene_add_shape(scene, st) == 0 {
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
    scene_list_shapes(scenes[si].as_ref().unwrap(), shapes);

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
    if scene_remove_shape(scene, shape_idx - 1) == 0 {
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

    scene_print(scenes[scene_idx as usize].as_ref().unwrap(), shapes);
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

    scene_save(scenes[scene_idx as usize].as_ref().unwrap(), &filename);
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

    if let Some(scene) = scene_load(&filename, shapes) {
        let idx = *scene_count;
        scenes[idx] = Some(scene);
        println!("Scene loaded (index {})", idx);
        *scene_count += 1;
    }
}

fn compare_shapes_fn(shapes: &[Box<Shape>], stdin_lock: &mut io::StdinLock) {
    println!("\nSelect first shape (0-{}):", SHAPE_COUNT - 1);
    for i in 0..SHAPE_COUNT {
        println!("{}. {}", i, shape_type_name(ShapeType::from_usize(i).unwrap()));
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
    scene_list_shapes(sc1, shapes);

    println!("\nScene 2: {} ({} shapes)", sc2.name, sc2.shape_count);
    scene_list_shapes(sc2, shapes);

    if scene_equals(sc1, sc2) {
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
