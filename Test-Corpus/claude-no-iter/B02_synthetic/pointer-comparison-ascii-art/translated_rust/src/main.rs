// Rust translation of the C ASCII art drawing application.
// Reproduces output byte-identically for the same inputs.

use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::sync::OnceLock;

// ---------- constants ----------

const SHAPE_COUNT: usize = 10;
const MAX_SHAPES_IN_SCENE: usize = 50;
const MAX_SCENE_NAME: usize = 64;
const MAX_SCENES: usize = 10;

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

// ---------- data ----------

struct Shape {
    type_: usize,
    name: String,
    art: Vec<String>,
    #[allow(dead_code)]
    width: usize,
    height: usize,
}

struct Scene {
    name: String,
    shapes: Vec<&'static Shape>,
}

static SHAPES: OnceLock<Vec<&'static Shape>> = OnceLock::new();

// ---------- helpers ----------

fn flush_stdout() {
    let _ = io::stdout().flush();
}

fn fmt_ptr<T: ?Sized>(p: *const T) -> String {
    if p.is_null() {
        "(nil)".to_string()
    } else {
        format!("0x{:x}", p as *const () as usize)
    }
}

// ---------- StdinReader: scanf/fgets/getchar emulation ----------

struct StdinReader {
    reader: BufReader<io::Stdin>,
}

impl StdinReader {
    fn new() -> Self {
        Self {
            reader: BufReader::new(io::stdin()),
        }
    }

    /// C-style fgets reading up to `max - 1` bytes or up to and including '\n'.
    /// Returns None on immediate EOF (no bytes read).
    fn fgets(&mut self, max: usize) -> Option<Vec<u8>> {
        flush_stdout();
        let mut out: Vec<u8> = Vec::new();
        if max == 0 {
            return Some(out);
        }
        let limit = max - 1;
        while out.len() < limit {
            let (slice_take, found_nl, was_empty);
            {
                let buf = match self.reader.fill_buf() {
                    Ok(b) => b,
                    Err(_) => {
                        return if out.is_empty() { None } else { Some(out) }
                    }
                };
                if buf.is_empty() {
                    was_empty = true;
                    slice_take = 0;
                    found_nl = None;
                } else {
                    was_empty = false;
                    let space = limit - out.len();
                    let take = buf.len().min(space);
                    let nl = buf[..take].iter().position(|&b| b == b'\n');
                    match nl {
                        Some(p) => {
                            out.extend_from_slice(&buf[..=p]);
                            slice_take = p + 1;
                            found_nl = Some(());
                        }
                        None => {
                            out.extend_from_slice(&buf[..take]);
                            slice_take = take;
                            found_nl = None;
                        }
                    }
                }
            }
            if was_empty {
                return if out.is_empty() { None } else { Some(out) };
            }
            self.reader.consume(slice_take);
            if found_nl.is_some() {
                return Some(out);
            }
        }
        Some(out)
    }

    /// C-style scanf("%d", ...). Skips leading whitespace, reads optional sign,
    /// then digits. Stops at first non-digit (which is left in the input buffer).
    /// Returns None if nothing parsable was found.
    fn scanf_int(&mut self) -> Option<i64> {
        flush_stdout();
        // Skip whitespace.
        loop {
            let consume_count;
            let exhausted;
            {
                let buf = match self.reader.fill_buf() {
                    Ok(b) => b,
                    Err(_) => return None,
                };
                if buf.is_empty() {
                    return None;
                }
                let mut c = 0;
                for &b in buf {
                    if b.is_ascii_whitespace() {
                        c += 1;
                    } else {
                        break;
                    }
                }
                consume_count = c;
                exhausted = c == buf.len();
            }
            self.reader.consume(consume_count);
            if !exhausted {
                break;
            }
        }

        let mut s: Vec<u8> = Vec::new();
        let mut started = false;
        loop {
            let consume_count;
            let stop;
            let eof;
            {
                let buf = match self.reader.fill_buf() {
                    Ok(b) => b,
                    Err(_) => break,
                };
                if buf.is_empty() {
                    eof = true;
                    stop = true;
                    consume_count = 0;
                } else {
                    let mut idx = 0;
                    let mut stopped = false;
                    for &b in buf {
                        if !started && (b == b'+' || b == b'-') {
                            s.push(b);
                            idx += 1;
                            started = true;
                        } else if b.is_ascii_digit() {
                            s.push(b);
                            idx += 1;
                            started = true;
                        } else {
                            stopped = true;
                            break;
                        }
                    }
                    consume_count = idx;
                    stop = stopped;
                    eof = false;
                }
            }
            self.reader.consume(consume_count);
            if stop || eof {
                break;
            }
        }

        if s.is_empty() || (s.len() == 1 && (s[0] == b'+' || s[0] == b'-')) {
            return None;
        }
        std::str::from_utf8(&s).ok().and_then(|x| x.parse::<i64>().ok())
    }

    /// Mimic `while (getchar() != '\n');` -- consume bytes up to and including newline.
    /// Returns gracefully on EOF (the original C would loop forever, but tests
    /// always provide a newline or EOF won't matter for output correctness).
    fn consume_to_newline(&mut self) {
        loop {
            let consume_count;
            let found;
            let eof;
            {
                let buf = match self.reader.fill_buf() {
                    Ok(b) => b,
                    Err(_) => return,
                };
                if buf.is_empty() {
                    return;
                }
                if let Some(p) = buf.iter().position(|&b| b == b'\n') {
                    consume_count = p + 1;
                    found = true;
                    eof = false;
                } else {
                    consume_count = buf.len();
                    found = false;
                    eof = false;
                }
            }
            self.reader.consume(consume_count);
            if found || eof {
                return;
            }
        }
    }
}

// ---------- shape data ----------

fn make_shape(t: usize) -> Shape {
    match t {
        SHAPE_TREE => Shape {
            type_: SHAPE_TREE,
            name: "Tree".to_string(),
            height: 7,
            width: 11,
            art: vec![
                "    /\\    ".to_string(),
                "   /  \\   ".to_string(),
                "  /____\\  ".to_string(),
                "  /    \\  ".to_string(),
                " /______\\ ".to_string(),
                "    ||    ".to_string(),
                "    ||    ".to_string(),
            ],
        },
        SHAPE_TRACTOR => Shape {
            type_: SHAPE_TRACTOR,
            name: "Tractor".to_string(),
            height: 6,
            width: 20,
            art: vec![
                "      ________     ".to_string(),
                "     |        |___ ".to_string(),
                "     |  []  []|   |".to_string(),
                "  ___|________|___|".to_string(),
                " /  o        o   \\".to_string(),
                "|___|        |___| ".to_string(),
            ],
        },
        SHAPE_HOUSE => Shape {
            type_: SHAPE_HOUSE,
            name: "House".to_string(),
            height: 7,
            width: 13,
            art: vec![
                "     /\\     ".to_string(),
                "    /  \\    ".to_string(),
                "   /____\\   ".to_string(),
                "   |    |   ".to_string(),
                "   | [] |   ".to_string(),
                "   |    |   ".to_string(),
                "   |____|   ".to_string(),
            ],
        },
        SHAPE_SUN => Shape {
            type_: SHAPE_SUN,
            name: "Sun".to_string(),
            height: 7,
            width: 11,
            art: vec![
                "  \\  |  / ".to_string(),
                "   \\ | /  ".to_string(),
                "--- (@) ---".to_string(),
                "   / | \\  ".to_string(),
                "  /  |  \\ ".to_string(),
                "          ".to_string(),
                "          ".to_string(),
            ],
        },
        SHAPE_CLOUD => Shape {
            type_: SHAPE_CLOUD,
            name: "Cloud".to_string(),
            height: 4,
            width: 16,
            art: vec![
                "   _____       ".to_string(),
                "  /     \\_    ".to_string(),
                " /  ___  _\\  ".to_string(),
                "(__/   \\_)   ".to_string(),
            ],
        },
        SHAPE_FLOWER => Shape {
            type_: SHAPE_FLOWER,
            name: "Flower".to_string(),
            height: 7,
            width: 9,
            art: vec![
                "  \\|/  ".to_string(),
                " -(@)- ".to_string(),
                "  /|\\  ".to_string(),
                "   |   ".to_string(),
                "   |   ".to_string(),
                "  / \\  ".to_string(),
                " /   \\ ".to_string(),
            ],
        },
        SHAPE_CAR => Shape {
            type_: SHAPE_CAR,
            name: "Car".to_string(),
            height: 4,
            width: 16,
            art: vec![
                "  ____       ".to_string(),
                " /|_||_\\____ ".to_string(),
                "( o     o  ) ".to_string(),
                " -----------  ".to_string(),
            ],
        },
        SHAPE_STAR => Shape {
            type_: SHAPE_STAR,
            name: "Star".to_string(),
            height: 5,
            width: 9,
            art: vec![
                "    *    ".to_string(),
                "   ***   ".to_string(),
                "  *****  ".to_string(),
                " ******* ".to_string(),
                "*********".to_string(),
            ],
        },
        SHAPE_HEART => Shape {
            type_: SHAPE_HEART,
            name: "Heart".to_string(),
            height: 6,
            width: 11,
            art: vec![
                " *** ***  ".to_string(),
                "*********  ".to_string(),
                "*********  ".to_string(),
                " ******* ".to_string(),
                "  *****  ".to_string(),
                "   ***   ".to_string(),
            ],
        },
        SHAPE_RAINBOW => Shape {
            type_: SHAPE_RAINBOW,
            name: "Rainbow".to_string(),
            height: 5,
            width: 21,
            art: vec![
                "      _______      ".to_string(),
                "    /         \\    ".to_string(),
                "   /           \\   ".to_string(),
                "  /             \\  ".to_string(),
                " /               \\ ".to_string(),
            ],
        },
        _ => unreachable!(),
    }
}

fn shape_manager_init() {
    // Mirror C: allocate all shapes first (so addresses come from one batch),
    // then initialize each one in the same order as the C code.
    let mut owned: Vec<Box<Shape>> = (0..SHAPE_COUNT)
        .map(|_| {
            Box::new(Shape {
                type_: 0,
                name: String::new(),
                art: Vec::new(),
                width: 0,
                height: 0,
            })
        })
        .collect();

    for i in 0..SHAPE_COUNT {
        *owned[i] = make_shape(i);
    }

    let leaked: Vec<&'static Shape> =
        owned.into_iter().map(|b| &*Box::leak(b)).collect();
    let _ = SHAPES.set(leaked);
}

fn shape_manager_cleanup() {
    // Singletons were leaked; the OS reclaims memory on exit.
    // No printable side effects in the C version.
}

fn shape_get(t: usize) -> Option<&'static Shape> {
    if t >= SHAPE_COUNT {
        return None;
    }
    SHAPES.get().and_then(|v| v.get(t).copied())
}

fn shape_print(shape: Option<&Shape>) {
    match shape {
        None => println!("(null shape)"),
        Some(s) => {
            println!("{}:", s.name);
            for i in 0..s.height {
                if i < s.art.len() {
                    println!("{}", s.art[i]);
                }
            }
        }
    }
}

fn shape_equals_ptr(a: &Shape, b: &Shape) -> bool {
    (a as *const Shape) == (b as *const Shape)
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

// ---------- scene functions ----------

fn scene_create(name: &str) -> Box<Scene> {
    // Mimic strncpy with MAX_SCENE_NAME-1 byte limit, never panicking on UTF-8 boundaries.
    let bytes = name.as_bytes();
    let truncated = if bytes.len() >= MAX_SCENE_NAME {
        // Find largest valid UTF-8 prefix within the byte limit.
        let mut end = MAX_SCENE_NAME - 1;
        while end > 0 && (bytes[end] & 0xC0) == 0x80 {
            end -= 1;
        }
        std::str::from_utf8(&bytes[..end]).unwrap_or("").to_string()
    } else {
        name.to_string()
    };
    Box::new(Scene {
        name: truncated,
        shapes: Vec::new(),
    })
}

fn scene_add_shape(scene: &mut Scene, shape: &'static Shape) -> i32 {
    if scene.shapes.len() >= MAX_SHAPES_IN_SCENE {
        eprintln!("Error: Scene is full");
        return -1;
    }
    scene.shapes.push(shape);
    0
}

fn scene_remove_shape(scene: &mut Scene, index: i64) -> i32 {
    if index < 0 || (index as usize) >= scene.shapes.len() {
        return -1;
    }
    scene.shapes.remove(index as usize);
    0
}

fn scene_print(scene: Option<&Scene>) {
    match scene {
        None => println!("(null scene)"),
        Some(sc) => {
            println!("\n=== Scene: {} ===", sc.name);
            println!("Contains {} shape(s)\n", sc.shapes.len());
            for (i, sh) in sc.shapes.iter().enumerate() {
                println!("Shape #{}:", i + 1);
                shape_print(Some(*sh));
                println!();
            }
        }
    }
}

fn scene_equals(s1: &Scene, s2: &Scene) -> bool {
    if s1.shapes.len() != s2.shapes.len() {
        return false;
    }
    let mut matched = vec![false; MAX_SHAPES_IN_SCENE];
    for &sh1 in &s1.shapes {
        let mut found = false;
        for (j, &sh2) in s2.shapes.iter().enumerate() {
            if !matched[j] && shape_equals_ptr(sh1, sh2) {
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
    let file = match File::create(filename) {
        Ok(f) => f,
        Err(_) => {
            eprintln!(
                "Error: Could not open file '{}' for writing",
                filename
            );
            return -1;
        }
    };
    let mut w = BufWriter::new(file);
    let _ = writeln!(w, "{}", scene.name);
    let _ = writeln!(w, "{}", scene.shapes.len());
    for sh in &scene.shapes {
        let _ = writeln!(w, "{}", sh.type_);
    }
    drop(w);
    println!("Scene saved to '{}'", filename);
    0
}

fn scene_load(filename: &str) -> Option<Box<Scene>> {
    let file = match File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            eprintln!(
                "Error: Could not open file '{}' for reading",
                filename
            );
            return None;
        }
    };
    let mut reader = BufReader::new(file);

    let mut name_buf: Vec<u8> = Vec::new();
    let n = reader.read_until(b'\n', &mut name_buf).ok()?;
    if n == 0 {
        return None;
    }
    while name_buf.last() == Some(&b'\n') {
        name_buf.pop();
    }
    let name = match std::str::from_utf8(&name_buf) {
        Ok(s) => s.to_string(),
        Err(_) => String::from_utf8_lossy(&name_buf).into_owned(),
    };
    let mut scene = scene_create(&name);

    let mut line = String::new();
    if reader.read_line(&mut line).ok()? == 0 {
        return None;
    }
    let count: i64 = match line.trim().parse() {
        Ok(c) => c,
        Err(_) => return None,
    };

    for _ in 0..count {
        line.clear();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let t: i64 = match line.trim().parse() {
            Ok(t) => t,
            Err(_) => return None,
        };
        if t >= 0 {
            if let Some(sh) = shape_get(t as usize) {
                scene_add_shape(&mut scene, sh);
            }
        }
    }

    println!("Scene loaded from '{}'", filename);
    Some(scene)
}

fn scene_list_shapes(scene: Option<&Scene>) {
    match scene {
        None => println!("(null scene)"),
        Some(sc) => {
            println!("\nScene: {}", sc.name);
            println!("Shapes ({}):", sc.shapes.len());
            for (i, sh) in sc.shapes.iter().enumerate() {
                let p: *const Shape = *sh;
                println!("  {}. {} (ptr: {})", i + 1, sh.name, fmt_ptr(p));
            }
        }
    }
}

// ---------- main menu functions ----------

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
}

fn view_all_shapes() {
    println!("\n=== Available Shapes ===");
    for i in 0..SHAPE_COUNT {
        print!("\n{}. ", i + 1);
        shape_print(shape_get(i));
    }
}

fn strip_trailing_newline(buf: &[u8]) -> &[u8] {
    let mut end = buf.len();
    if end > 0 && buf[end - 1] == b'\n' {
        end -= 1;
    }
    &buf[..end]
}

fn create_new_scene(stdin: &mut StdinReader, scenes: &mut Vec<Box<Scene>>) {
    if scenes.len() >= MAX_SCENES {
        println!("Error: Maximum scenes reached");
        return;
    }

    print!("Enter scene name: ");
    let raw = match stdin.fgets(MAX_SCENE_NAME) {
        Some(b) => b,
        None => return,
    };
    let trimmed = strip_trailing_newline(&raw);
    let name = match std::str::from_utf8(trimmed) {
        Ok(s) => s.to_string(),
        Err(_) => String::from_utf8_lossy(trimmed).into_owned(),
    };

    let scene = scene_create(&name);
    let idx = scenes.len();
    scenes.push(scene);
    println!("Scene '{}' created (index {})", name, idx);
}

fn add_shape_to_scene(stdin: &mut StdinReader, scenes: &mut [Box<Scene>]) {
    if scenes.is_empty() {
        println!("No scenes available. Create a scene first.");
        return;
    }

    print!("Select scene (0-{}): ", scenes.len() - 1);
    let scene_idx = match stdin.scanf_int() {
        Some(v) => v,
        None => {
            println!("Invalid input");
            stdin.consume_to_newline();
            return;
        }
    };
    stdin.consume_to_newline();

    if scene_idx < 0 || (scene_idx as usize) >= scenes.len() {
        println!("Invalid scene index");
        return;
    }

    println!("\nSelect shape to add:");
    for i in 0..SHAPE_COUNT {
        println!("{}. {}", i, shape_type_name(i));
    }
    print!("Choice: ");

    let shape_type = match stdin.scanf_int() {
        Some(v) => v,
        None => {
            println!("Invalid input");
            stdin.consume_to_newline();
            return;
        }
    };
    stdin.consume_to_newline();

    if shape_type < 0 || (shape_type as usize) >= SHAPE_COUNT {
        println!("Invalid shape type");
        return;
    }

    let shape = match shape_get(shape_type as usize) {
        Some(s) => s,
        None => {
            println!("Error adding shape");
            return;
        }
    };

    let scene = &mut scenes[scene_idx as usize];
    if scene_add_shape(scene, shape) == 0 {
        let p: *const Shape = shape;
        println!(
            "Shape '{}' added to scene (reusing singleton at {})",
            shape.name,
            fmt_ptr(p)
        );
    } else {
        println!("Error adding shape");
    }
}

fn remove_shape_from_scene(stdin: &mut StdinReader, scenes: &mut [Box<Scene>]) {
    if scenes.is_empty() {
        println!("No scenes available");
        return;
    }

    print!("Select scene (0-{}): ", scenes.len() - 1);
    let scene_idx = match stdin.scanf_int() {
        Some(v) => v,
        None => {
            println!("Invalid input");
            stdin.consume_to_newline();
            return;
        }
    };
    stdin.consume_to_newline();

    if scene_idx < 0 || (scene_idx as usize) >= scenes.len() {
        println!("Invalid scene index");
        return;
    }

    let scene_count_in = scenes[scene_idx as usize].shapes.len();
    scene_list_shapes(Some(&scenes[scene_idx as usize]));

    if scene_count_in == 0 {
        println!("Scene is empty");
        return;
    }

    print!("Select shape to remove (1-{}): ", scene_count_in);
    let shape_idx = match stdin.scanf_int() {
        Some(v) => v,
        None => {
            println!("Invalid input");
            stdin.consume_to_newline();
            return;
        }
    };
    stdin.consume_to_newline();

    let scene = &mut scenes[scene_idx as usize];
    if scene_remove_shape(scene, shape_idx - 1) == 0 {
        println!("Shape removed");
    } else {
        println!("Error removing shape");
    }
}

fn view_scene(stdin: &mut StdinReader, scenes: &[Box<Scene>]) {
    if scenes.is_empty() {
        println!("No scenes available");
        return;
    }

    print!("Select scene (0-{}): ", scenes.len() - 1);
    let scene_idx = match stdin.scanf_int() {
        Some(v) => v,
        None => {
            println!("Invalid input");
            stdin.consume_to_newline();
            return;
        }
    };
    stdin.consume_to_newline();

    if scene_idx < 0 || (scene_idx as usize) >= scenes.len() {
        println!("Invalid scene index");
        return;
    }

    scene_print(Some(&scenes[scene_idx as usize]));
}

fn list_all_scenes(scenes: &[Box<Scene>]) {
    println!("\n=== All Scenes ===");
    if scenes.is_empty() {
        println!("No scenes created yet");
        return;
    }

    for (i, sc) in scenes.iter().enumerate() {
        println!("{}. {} ({} shapes)", i, sc.name, sc.shapes.len());
    }
}

fn save_scene_to_file(stdin: &mut StdinReader, scenes: &[Box<Scene>]) {
    if scenes.is_empty() {
        println!("No scenes available");
        return;
    }

    print!("Select scene (0-{}): ", scenes.len() - 1);
    let scene_idx = match stdin.scanf_int() {
        Some(v) => v,
        None => {
            println!("Invalid input");
            stdin.consume_to_newline();
            return;
        }
    };
    stdin.consume_to_newline();

    if scene_idx < 0 || (scene_idx as usize) >= scenes.len() {
        println!("Invalid scene index");
        return;
    }

    print!("Enter filename: ");
    let raw = match stdin.fgets(256) {
        Some(b) => b,
        None => return,
    };
    let trimmed = strip_trailing_newline(&raw);
    let filename = match std::str::from_utf8(trimmed) {
        Ok(s) => s.to_string(),
        Err(_) => String::from_utf8_lossy(trimmed).into_owned(),
    };

    let _ = scene_save(&scenes[scene_idx as usize], &filename);
}

fn load_scene_from_file(stdin: &mut StdinReader, scenes: &mut Vec<Box<Scene>>) {
    if scenes.len() >= MAX_SCENES {
        println!("Error: Maximum scenes reached");
        return;
    }

    print!("Enter filename: ");
    let raw = match stdin.fgets(256) {
        Some(b) => b,
        None => return,
    };
    let trimmed = strip_trailing_newline(&raw);
    let filename = match std::str::from_utf8(trimmed) {
        Ok(s) => s.to_string(),
        Err(_) => String::from_utf8_lossy(trimmed).into_owned(),
    };

    if let Some(scene) = scene_load(&filename) {
        scenes.push(scene);
        println!("Scene loaded (index {})", scenes.len() - 1);
    }
}

fn compare_shapes(stdin: &mut StdinReader) {
    println!("\nSelect first shape (0-{}):", SHAPE_COUNT - 1);
    for i in 0..SHAPE_COUNT {
        println!("{}. {}", i, shape_type_name(i));
    }
    print!("Choice: ");

    let type1 = match stdin.scanf_int() {
        Some(v) => v,
        None => {
            println!("Invalid input");
            stdin.consume_to_newline();
            return;
        }
    };
    stdin.consume_to_newline();

    print!("\nSelect second shape (0-{}): ", SHAPE_COUNT - 1);
    let type2 = match stdin.scanf_int() {
        Some(v) => v,
        None => {
            println!("Invalid input");
            stdin.consume_to_newline();
            return;
        }
    };
    stdin.consume_to_newline();

    if type1 < 0
        || (type1 as usize) >= SHAPE_COUNT
        || type2 < 0
        || (type2 as usize) >= SHAPE_COUNT
    {
        println!("Invalid shape type");
        return;
    }

    let s1 = shape_get(type1 as usize).unwrap();
    let s2 = shape_get(type2 as usize).unwrap();

    let p1: *const Shape = s1;
    let p2: *const Shape = s2;
    println!("\nShape 1: {} (ptr: {})", s1.name, fmt_ptr(p1));
    println!("Shape 2: {} (ptr: {})", s2.name, fmt_ptr(p2));
    let eq = (p1 == p2) as i32;
    println!("Comparison of pointers: {}", eq);

    if shape_equals_ptr(s1, s2) {
        println!("Result: Shapes are EQUAL (same instance)");
    } else {
        println!("Result: Shapes are NOT EQUAL (different instances)");
    }
}

fn compare_scenes(stdin: &mut StdinReader, scenes: &[Box<Scene>]) {
    if scenes.len() < 2 {
        println!("Need at least 2 scenes to compare");
        return;
    }

    print!("Select first scene (0-{}): ", scenes.len() - 1);
    let idx1 = match stdin.scanf_int() {
        Some(v) => v,
        None => {
            println!("Invalid input");
            stdin.consume_to_newline();
            return;
        }
    };
    stdin.consume_to_newline();

    print!("Select second scene (0-{}): ", scenes.len() - 1);
    let idx2 = match stdin.scanf_int() {
        Some(v) => v,
        None => {
            println!("Invalid input");
            stdin.consume_to_newline();
            return;
        }
    };
    stdin.consume_to_newline();

    if idx1 < 0
        || (idx1 as usize) >= scenes.len()
        || idx2 < 0
        || (idx2 as usize) >= scenes.len()
    {
        println!("Invalid scene index");
        return;
    }

    let sc1 = &scenes[idx1 as usize];
    let sc2 = &scenes[idx2 as usize];

    println!("\nScene 1: {} ({} shapes)", sc1.name, sc1.shapes.len());
    scene_list_shapes(Some(sc1));

    println!("\nScene 2: {} ({} shapes)", sc2.name, sc2.shapes.len());
    scene_list_shapes(Some(sc2));

    if scene_equals(sc1, sc2) {
        println!("\nResult: Scenes are EQUAL (1:1 correspondence)");
    } else {
        println!("\nResult: Scenes are NOT EQUAL");
    }
}

fn delete_scene(stdin: &mut StdinReader, scenes: &mut Vec<Box<Scene>>) {
    if scenes.is_empty() {
        println!("No scenes available");
        return;
    }

    print!("Select scene to delete (0-{}): ", scenes.len() - 1);
    let scene_idx = match stdin.scanf_int() {
        Some(v) => v,
        None => {
            println!("Invalid input");
            stdin.consume_to_newline();
            return;
        }
    };
    stdin.consume_to_newline();

    if scene_idx < 0 || (scene_idx as usize) >= scenes.len() {
        println!("Invalid scene index");
        return;
    }

    scenes.remove(scene_idx as usize);
    println!("Scene deleted");
}

// ---------- main ----------

fn main() {
    println!("\u{2554}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2557}");
    println!("\u{2551}  ASCII ART DRAWING APPLICATION        \u{2551}");
    println!("\u{2551}  Child-Friendly Shape Editor           \u{2551}");
    println!("\u{255A}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255D}");

    shape_manager_init();

    let mut scenes: Vec<Box<Scene>> = Vec::new();
    let mut stdin = StdinReader::new();

    loop {
        print_menu();

        let raw = match stdin.fgets(256) {
            Some(b) => b,
            None => break,
        };
        let line = String::from_utf8_lossy(&raw);

        let choice: i64 = match parse_first_int(&line) {
            Some(v) => v,
            None => {
                println!("Invalid input");
                continue;
            }
        };

        match choice {
            1 => view_all_shapes(),
            2 => create_new_scene(&mut stdin, &mut scenes),
            3 => add_shape_to_scene(&mut stdin, &mut scenes),
            4 => remove_shape_from_scene(&mut stdin, &mut scenes),
            5 => view_scene(&mut stdin, &scenes),
            6 => list_all_scenes(&scenes),
            7 => save_scene_to_file(&mut stdin, &scenes),
            8 => load_scene_from_file(&mut stdin, &mut scenes),
            9 => compare_shapes(&mut stdin),
            10 => compare_scenes(&mut stdin, &scenes),
            11 => delete_scene(&mut stdin, &mut scenes),
            12 => {
                println!("\nCleaning up and exiting...");
                scenes.clear();
                shape_manager_cleanup();
                println!("Goodbye!");
                let _ = io::stdout().flush();
                return;
            }
            _ => println!("Invalid choice"),
        }
    }

    scenes.clear();
    shape_manager_cleanup();
}

/// Mimic `sscanf(buf, "%d", &x)` -- returns the first integer token in the line,
/// skipping leading whitespace.  Returns None if no integer is found.
fn parse_first_int(s: &str) -> Option<i64> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    let start = i;
    if bytes[i] == b'+' || bytes[i] == b'-' {
        i += 1;
    }
    let digit_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if digit_start == i {
        return None;
    }
    std::str::from_utf8(&bytes[start..i])
        .ok()
        .and_then(|x| x.parse::<i64>().ok())
}

// silence unused import warning since Read is brought in for trait methods used elsewhere
#[allow(dead_code)]
fn _force_read_use(_r: &dyn Read) {}
