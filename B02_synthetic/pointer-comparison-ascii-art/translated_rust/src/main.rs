use std::io::{self, BufRead, Write};
use std::fs;

const MAX_SHAPES_IN_SCENE: usize = 50;
const MAX_SCENES: usize = 10;
const SHAPE_COUNT: usize = 10;

// Shape type indices
const SHAPE_TREE: usize = 0;
const SHAPE_TRACTOR: usize = 1;
const SHAPE_HOUSE: usize = 2;
const SHAPE_SUN: usize = 3;
const SHAPE_CLOUD: usize = 4;
const SHAPE_FLOWER: usize = 5;
const SHAPE_CAR: usize = 6;
const SHAPE_STAR: usize = 7;
const SHAPE_HEART: usize = 8;
const SHAPE_RAINBOW: usize = 9;

struct Shape {
    #[allow(dead_code)]
    shape_type: usize,
    name: String,
    art: Vec<String>,
    #[allow(dead_code)]
    width: i32,
    height: i32,
}

struct Scene {
    name: String,
    shapes: Vec<usize>, // indices into the global shapes array
}

struct ShapeManager {
    shapes: Vec<Box<Shape>>,
}

impl ShapeManager {
    fn init() -> Self {
        let mut shapes: Vec<Box<Shape>> = Vec::with_capacity(SHAPE_COUNT);

        // Tree
        shapes.push(Box::new(Shape {
            shape_type: SHAPE_TREE, name: "Tree".into(), width: 11, height: 7,
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
            shape_type: SHAPE_TRACTOR, name: "Tractor".into(), width: 20, height: 6,
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
            shape_type: SHAPE_HOUSE, name: "House".into(), width: 13, height: 7,
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
            shape_type: SHAPE_SUN, name: "Sun".into(), width: 11, height: 7,
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
            shape_type: SHAPE_CLOUD, name: "Cloud".into(), width: 16, height: 4,
            art: vec![
                "   _____       ".into(),
                "  /     \\_    ".into(),
                " /  ___  _\\  ".into(),
                "(__/   \\_)   ".into(),
            ],
        }));
        // Flower
        shapes.push(Box::new(Shape {
            shape_type: SHAPE_FLOWER, name: "Flower".into(), width: 9, height: 7,
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
            shape_type: SHAPE_CAR, name: "Car".into(), width: 16, height: 4,
            art: vec![
                "  ____       ".into(),
                " /|_||_\\____ ".into(),
                "( o     o  ) ".into(),
                " -----------  ".into(),
            ],
        }));
        // Star
        shapes.push(Box::new(Shape {
            shape_type: SHAPE_STAR, name: "Star".into(), width: 9, height: 5,
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
            shape_type: SHAPE_HEART, name: "Heart".into(), width: 11, height: 6,
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
            shape_type: SHAPE_RAINBOW, name: "Rainbow".into(), width: 21, height: 5,
            art: vec![
                "      _______      ".into(),
                "    /         \\    ".into(),
                "   /           \\   ".into(),
                "  /             \\  ".into(),
                " /               \\ ".into(),
            ],
        }));

        ShapeManager { shapes }
    }

    fn get(&self, t: usize) -> Option<&Shape> {
        self.shapes.get(t).map(|b| b.as_ref())
    }

    fn ptr(&self, t: usize) -> *const Shape {
        &*self.shapes[t] as *const Shape
    }
}

fn shape_type_name(t: usize) -> &'static str {
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

fn shape_print(shape: &Shape) {
    println!("{}:", shape.name);
    for i in 0..shape.height as usize {
        println!("{}", shape.art[i]);
    }
}

fn shape_equals(mgr: &ShapeManager, i1: usize, i2: usize) -> bool {
    // C uses pointer comparison: s1 == s2
    // Since shapes are singletons, same index => same pointer
    mgr.ptr(i1) == mgr.ptr(i2)
}

// Scene functions

fn scene_create(name: &str) -> Scene {
    Scene {
        name: name.to_string(),
        shapes: Vec::new(),
    }
}

fn scene_add_shape(scene: &mut Scene, shape_idx: usize) -> i32 {
    if scene.shapes.len() >= MAX_SHAPES_IN_SCENE {
        eprintln!("Error: Scene is full");
        return -1;
    }
    scene.shapes.push(shape_idx);
    0
}

fn scene_remove_shape(scene: &mut Scene, index: i32) -> i32 {
    if index < 0 || index as usize >= scene.shapes.len() {
        return -1;
    }
    scene.shapes.remove(index as usize);
    0
}

fn scene_print(scene: &Scene, mgr: &ShapeManager) {
    println!();
    println!("=== Scene: {} ===", scene.name);
    println!("Contains {} shape(s)\n", scene.shapes.len());

    for (i, &si) in scene.shapes.iter().enumerate() {
        println!("Shape #{}:", i + 1);
        shape_print(mgr.get(si).unwrap());
        println!();
    }
}

fn scene_equals(s1: &Scene, s2: &Scene, mgr: &ShapeManager) -> bool {
    if s1.shapes.len() != s2.shapes.len() {
        return false;
    }
    let mut matched = vec![false; s2.shapes.len()];
    for &si in &s1.shapes {
        let mut found = false;
        for (j, &sj) in s2.shapes.iter().enumerate() {
            if !matched[j] && shape_equals(mgr, si, sj) {
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
    let mut content = String::new();
    content.push_str(&scene.name);
    content.push('\n');
    content.push_str(&scene.shapes.len().to_string());
    content.push('\n');
    for &si in &scene.shapes {
        content.push_str(&si.to_string());
        content.push('\n');
    }
    match fs::write(filename, &content) {
        Ok(_) => {
            println!("Scene saved to '{}'", filename);
            0
        }
        Err(_) => {
            eprintln!("Error: Could not open file '{}' for writing", filename);
            -1
        }
    }
}

fn scene_load(filename: &str, mgr: &ShapeManager) -> Option<Scene> {
    let content = match fs::read_to_string(filename) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Error: Could not open file '{}' for reading", filename);
            return None;
        }
    };
    let mut lines = content.lines();
    let name = lines.next()?;
    let shape_count: usize = lines.next()?.parse().ok()?;
    let mut scene = scene_create(name);
    for _ in 0..shape_count {
        let t: usize = lines.next()?.parse().ok()?;
        if mgr.get(t).is_some() {
            scene_add_shape(&mut scene, t);
        }
    }
    println!("Scene loaded from '{}'", filename);
    Some(scene)
}

fn scene_list_shapes(scene: &Scene, mgr: &ShapeManager) {
    println!();
    println!("Scene: {}", scene.name);
    println!("Shapes ({}):", scene.shapes.len());
    for (i, &si) in scene.shapes.iter().enumerate() {
        let s = mgr.get(si).unwrap();
        println!("  {}. {} (ptr: {:?})", i + 1, s.name, mgr.ptr(si));
    }
}

// Helper: read a line from stdin, return None on EOF
fn read_line(stdin: &io::Stdin) -> Option<String> {
    let lock = stdin.lock();
    match lock.lines().next() {
        Some(Ok(line)) => Some(line),
        _ => None,
    }
}

// Helper: mimic scanf("%d") + consume rest of line
// Reads a line, parses first int from it
fn scanf_int(stdin: &io::Stdin) -> Option<i32> {
    let line = read_line(stdin)?;
    // sscanf behavior: skip leading whitespace, parse int
    let trimmed = line.trim_start();
    // parse the leading integer
    let end = trimmed.find(|c: char| !c.is_ascii_digit() && c != '-').unwrap_or(trimmed.len());
    if end == 0 {
        return None;
    }
    trimmed[..end].parse().ok()
}

fn main() {
    println!("\u{2554}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2557}");
    println!("\u{2551}  ASCII ART DRAWING APPLICATION        \u{2551}");
    println!("\u{2551}  Child-Friendly Shape Editor           \u{2551}");
    println!("\u{255a}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255d}");

    let mgr = ShapeManager::init();
    let stdin = io::stdin();
    let mut scenes: Vec<Option<Scene>> = Vec::new();
    let mut scene_count: usize = 0;

    loop {
        // print_menu
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

        let line = match read_line(&stdin) {
            Some(l) => l,
            None => break,
        };
        let choice: i32 = match line.trim().parse::<i32>() {
            Ok(v) => v,
            Err(_) => {
                // Check if sscanf would have parsed something
                let trimmed = line.trim_start();
                let end = trimmed.find(|c: char| !c.is_ascii_digit() && c != '-').unwrap_or(trimmed.len());
                if end == 0 {
                    println!("Invalid input");
                    continue;
                }
                match trimmed[..end].parse::<i32>() {
                    Ok(v) => v,
                    Err(_) => {
                        println!("Invalid input");
                        continue;
                    }
                }
            }
        };

        match choice {
            1 => {
                // view_all_shapes
                println!("\n=== Available Shapes ===");
                for i in 0..SHAPE_COUNT {
                    print!("\n{}. ", i + 1);
                    shape_print(mgr.get(i).unwrap());
                }
            }
            2 => {
                // create_new_scene
                if scene_count >= MAX_SCENES {
                    println!("Error: Maximum scenes reached");
                    continue;
                }
                print!("Enter scene name: ");
                let _ = io::stdout().flush();
                let name = match read_line(&stdin) {
                    Some(n) => n,
                    None => continue,
                };
                let scene = scene_create(&name);
                println!("Scene '{}' created (index {})", name, scene_count);
                scenes.push(Some(scene));
                scene_count += 1;
            }
            3 => {
                // add_shape_to_scene
                if scene_count == 0 {
                    println!("No scenes available. Create a scene first.");
                    continue;
                }
                print!("Select scene (0-{}): ", scene_count - 1);
                let _ = io::stdout().flush();
                let scene_idx = match scanf_int(&stdin) {
                    Some(v) => v,
                    None => {
                        println!("Invalid input");
                        continue;
                    }
                };
                if scene_idx < 0 || scene_idx as usize >= scene_count {
                    println!("Invalid scene index");
                    continue;
                }
                let si = scene_idx as usize;
                println!("\nSelect shape to add:");
                for i in 0..SHAPE_COUNT {
                    println!("{}. {}", i, shape_type_name(i));
                }
                print!("Choice: ");
                let _ = io::stdout().flush();
                let shape_type = match scanf_int(&stdin) {
                    Some(v) => v,
                    None => {
                        println!("Invalid input");
                        continue;
                    }
                };
                if shape_type < 0 || shape_type as usize >= SHAPE_COUNT {
                    println!("Invalid shape type");
                    continue;
                }
                let st = shape_type as usize;
                let s = mgr.get(st).unwrap();
                let ptr = mgr.ptr(st);
                if let Some(ref mut scene) = scenes[si] {
                    if scene_add_shape(scene, st) == 0 {
                        println!("Shape '{}' added to scene (reusing singleton at {:?})", s.name, ptr);
                    } else {
                        println!("Error adding shape");
                    }
                }
            }
            4 => {
                // remove_shape_from_scene
                if scene_count == 0 {
                    println!("No scenes available");
                    continue;
                }
                print!("Select scene (0-{}): ", scene_count - 1);
                let _ = io::stdout().flush();
                let scene_idx = match scanf_int(&stdin) {
                    Some(v) => v,
                    None => {
                        println!("Invalid input");
                        continue;
                    }
                };
                if scene_idx < 0 || scene_idx as usize >= scene_count {
                    println!("Invalid scene index");
                    continue;
                }
                let si = scene_idx as usize;
                if let Some(ref scene) = scenes[si] {
                    scene_list_shapes(scene, &mgr);
                    if scene.shapes.is_empty() {
                        println!("Scene is empty");
                        continue;
                    }
                    print!("Select shape to remove (1-{}): ", scene.shapes.len());
                    let _ = io::stdout().flush();
                }
                let shape_idx = match scanf_int(&stdin) {
                    Some(v) => v,
                    None => {
                        println!("Invalid input");
                        continue;
                    }
                };
                if let Some(ref mut scene) = scenes[si] {
                    if scene_remove_shape(scene, shape_idx - 1) == 0 {
                        println!("Shape removed");
                    } else {
                        println!("Error removing shape");
                    }
                }
            }
            5 => {
                // view_scene
                if scene_count == 0 {
                    println!("No scenes available");
                    continue;
                }
                print!("Select scene (0-{}): ", scene_count - 1);
                let _ = io::stdout().flush();
                let scene_idx = match scanf_int(&stdin) {
                    Some(v) => v,
                    None => {
                        println!("Invalid input");
                        continue;
                    }
                };
                if scene_idx < 0 || scene_idx as usize >= scene_count {
                    println!("Invalid scene index");
                    continue;
                }
                if let Some(ref scene) = scenes[scene_idx as usize] {
                    scene_print(scene, &mgr);
                }
            }
            6 => {
                // list_all_scenes
                println!("\n=== All Scenes ===");
                if scene_count == 0 {
                    println!("No scenes created yet");
                } else {
                    for i in 0..scene_count {
                        if let Some(ref scene) = scenes[i] {
                            println!("{}. {} ({} shapes)", i, scene.name, scene.shapes.len());
                        }
                    }
                }
            }
            7 => {
                // save_scene_to_file
                if scene_count == 0 {
                    println!("No scenes available");
                    continue;
                }
                print!("Select scene (0-{}): ", scene_count - 1);
                let _ = io::stdout().flush();
                let scene_idx = match scanf_int(&stdin) {
                    Some(v) => v,
                    None => {
                        println!("Invalid input");
                        continue;
                    }
                };
                if scene_idx < 0 || scene_idx as usize >= scene_count {
                    println!("Invalid scene index");
                    continue;
                }
                print!("Enter filename: ");
                let _ = io::stdout().flush();
                let filename = match read_line(&stdin) {
                    Some(f) => f,
                    None => continue,
                };
                if let Some(ref scene) = scenes[scene_idx as usize] {
                    scene_save(scene, &filename);
                }
            }
            8 => {
                // load_scene_from_file
                if scene_count >= MAX_SCENES {
                    println!("Error: Maximum scenes reached");
                    continue;
                }
                print!("Enter filename: ");
                let _ = io::stdout().flush();
                let filename = match read_line(&stdin) {
                    Some(f) => f,
                    None => continue,
                };
                if let Some(scene) = scene_load(&filename, &mgr) {
                    scenes.push(Some(scene));
                    println!("Scene loaded (index {})", scene_count);
                    scene_count += 1;
                }
            }
            9 => {
                // compare_shapes
                println!("\nSelect first shape (0-{}):", SHAPE_COUNT - 1);
                for i in 0..SHAPE_COUNT {
                    println!("{}. {}", i, shape_type_name(i));
                }
                print!("Choice: ");
                let _ = io::stdout().flush();
                let type1 = match scanf_int(&stdin) {
                    Some(v) => v,
                    None => {
                        println!("Invalid input");
                        continue;
                    }
                };
                print!("\nSelect second shape (0-{}): ", SHAPE_COUNT - 1);
                let _ = io::stdout().flush();
                let type2 = match scanf_int(&stdin) {
                    Some(v) => v,
                    None => {
                        println!("Invalid input");
                        continue;
                    }
                };
                if type1 < 0 || type1 as usize >= SHAPE_COUNT || type2 < 0 || type2 as usize >= SHAPE_COUNT {
                    println!("Invalid shape type");
                    continue;
                }
                let t1 = type1 as usize;
                let t2 = type2 as usize;
                let s1 = mgr.get(t1).unwrap();
                let s2 = mgr.get(t2).unwrap();
                let p1 = mgr.ptr(t1);
                let p2 = mgr.ptr(t2);
                println!("\nShape 1: {} (ptr: {:?})", s1.name, p1);
                println!("Shape 2: {} (ptr: {:?})", s2.name, p2);
                println!("Comparison of pointers: {}", if p1 == p2 { 1 } else { 0 });
                if shape_equals(&mgr, t1, t2) {
                    println!("Result: Shapes are EQUAL (same instance)");
                } else {
                    println!("Result: Shapes are NOT EQUAL (different instances)");
                }
            }
            10 => {
                // compare_scenes
                if scene_count < 2 {
                    println!("Need at least 2 scenes to compare");
                    continue;
                }
                print!("Select first scene (0-{}): ", scene_count - 1);
                let _ = io::stdout().flush();
                let idx1 = match scanf_int(&stdin) {
                    Some(v) => v,
                    None => {
                        println!("Invalid input");
                        continue;
                    }
                };
                print!("Select second scene (0-{}): ", scene_count - 1);
                let _ = io::stdout().flush();
                let idx2 = match scanf_int(&stdin) {
                    Some(v) => v,
                    None => {
                        println!("Invalid input");
                        continue;
                    }
                };
                if idx1 < 0 || idx1 as usize >= scene_count || idx2 < 0 || idx2 as usize >= scene_count {
                    println!("Invalid scene index");
                    continue;
                }
                let i1 = idx1 as usize;
                let i2 = idx2 as usize;
                if let (Some(ref sc1), Some(ref sc2)) = (&scenes[i1], &scenes[i2]) {
                    println!("\nScene 1: {} ({} shapes)", sc1.name, sc1.shapes.len());
                    scene_list_shapes(sc1, &mgr);
                    println!("\nScene 2: {} ({} shapes)", sc2.name, sc2.shapes.len());
                    scene_list_shapes(sc2, &mgr);
                    if scene_equals(sc1, sc2, &mgr) {
                        println!("\nResult: Scenes are EQUAL (1:1 correspondence)");
                    } else {
                        println!("\nResult: Scenes are NOT EQUAL");
                    }
                }
            }
            11 => {
                // delete_scene
                if scene_count == 0 {
                    println!("No scenes available");
                    continue;
                }
                print!("Select scene to delete (0-{}): ", scene_count - 1);
                let _ = io::stdout().flush();
                let scene_idx = match scanf_int(&stdin) {
                    Some(v) => v,
                    None => {
                        println!("Invalid input");
                        continue;
                    }
                };
                if scene_idx < 0 || scene_idx as usize >= scene_count {
                    println!("Invalid scene index");
                    continue;
                }
                scenes.remove(scene_idx as usize);
                scene_count -= 1;
                println!("Scene deleted");
            }
            12 => {
                println!("\nCleaning up and exiting...");
                println!("Goodbye!");
                return;
            }
            _ => {
                println!("Invalid choice");
            }
        }
    }
}
