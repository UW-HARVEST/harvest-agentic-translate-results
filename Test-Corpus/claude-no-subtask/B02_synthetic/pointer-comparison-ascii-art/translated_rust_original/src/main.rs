// Rust translation of the C ASCII art drawing application.
// Reproduces the original behavior, including any quirks.

use std::fs::File;
use std::io::{self, Read, Write};

// ------------- Constants -------------
const MAX_SHAPE_NAME: usize = 32;
const MAX_SCENE_NAME: usize = 64;
const MAX_SHAPES_IN_SCENE: usize = 50;
const MAX_SCENES: usize = 10;

const SHAPE_TREE: i32 = 0;
const SHAPE_TRACTOR: i32 = 1;
const SHAPE_HOUSE: i32 = 2;
const SHAPE_SUN: i32 = 3;
const SHAPE_CLOUD: i32 = 4;
const SHAPE_FLOWER: i32 = 5;
const SHAPE_CAR: i32 = 6;
const SHAPE_STAR: i32 = 7;
const SHAPE_HEART: i32 = 8;
const SHAPE_RAINBOW: i32 = 9;
const SHAPE_COUNT: i32 = 10;

// ------------- Types -------------
struct Shape {
    type_: i32,
    name: String,
    art: Vec<String>,
    width: i32,
    height: i32,
}

struct Scene {
    name: String,
    shapes: Vec<*mut Shape>,
}

// ------------- Globals -------------
static mut SHAPES: [*mut Shape; SHAPE_COUNT as usize] =
    [std::ptr::null_mut(); SHAPE_COUNT as usize];
static mut SCENES: [*mut Scene; MAX_SCENES] = [std::ptr::null_mut(); MAX_SCENES];
static mut SCENE_COUNT: usize = 0;

// ------------- Shape initializers -------------
fn make_shape(t: i32, name: &str, height: i32, width: i32, art: &[&str]) -> Shape {
    Shape {
        type_: t,
        name: name.to_string(),
        art: art.iter().map(|s| s.to_string()).collect(),
        width,
        height,
    }
}

fn init_tree() -> Shape {
    make_shape(
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
    )
}

fn init_tractor() -> Shape {
    make_shape(
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
    )
}

fn init_house() -> Shape {
    make_shape(
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
    )
}

fn init_sun() -> Shape {
    make_shape(
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
    )
}

fn init_cloud() -> Shape {
    make_shape(
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
    )
}

fn init_flower() -> Shape {
    make_shape(
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
    )
}

fn init_car() -> Shape {
    make_shape(
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
    )
}

fn init_star() -> Shape {
    make_shape(
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
    )
}

fn init_heart() -> Shape {
    make_shape(
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
    )
}

fn init_rainbow() -> Shape {
    make_shape(
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
    )
}

// ------------- Shape manager -------------
fn shape_manager_init() {
    unsafe {
        SHAPES[SHAPE_TREE as usize] = Box::into_raw(Box::new(init_tree()));
        SHAPES[SHAPE_TRACTOR as usize] = Box::into_raw(Box::new(init_tractor()));
        SHAPES[SHAPE_HOUSE as usize] = Box::into_raw(Box::new(init_house()));
        SHAPES[SHAPE_SUN as usize] = Box::into_raw(Box::new(init_sun()));
        SHAPES[SHAPE_CLOUD as usize] = Box::into_raw(Box::new(init_cloud()));
        SHAPES[SHAPE_FLOWER as usize] = Box::into_raw(Box::new(init_flower()));
        SHAPES[SHAPE_CAR as usize] = Box::into_raw(Box::new(init_car()));
        SHAPES[SHAPE_STAR as usize] = Box::into_raw(Box::new(init_star()));
        SHAPES[SHAPE_HEART as usize] = Box::into_raw(Box::new(init_heart()));
        SHAPES[SHAPE_RAINBOW as usize] = Box::into_raw(Box::new(init_rainbow()));
    }
}

fn shape_manager_cleanup() {
    unsafe {
        for i in 0..(SHAPE_COUNT as usize) {
            if !SHAPES[i].is_null() {
                drop(Box::from_raw(SHAPES[i]));
                SHAPES[i] = std::ptr::null_mut();
            }
        }
    }
}

fn shape_get(t: i32) -> *mut Shape {
    if t < 0 || t >= SHAPE_COUNT {
        return std::ptr::null_mut();
    }
    unsafe { SHAPES[t as usize] }
}

fn shape_print(shape: *const Shape) {
    if shape.is_null() {
        println!("(null shape)");
        return;
    }
    let s = unsafe { &*shape };
    println!("{}:", s.name);
    for i in 0..(s.height as usize) {
        if i < s.art.len() {
            println!("{}", s.art[i]);
        }
    }
}

fn shape_equals(s1: *const Shape, s2: *const Shape) -> i32 {
    if s1 == s2 {
        1
    } else {
        0
    }
}

fn shape_type_name(t: i32) -> &'static str {
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

// ------------- Scene functions -------------
fn truncate_to_max_scene_name(s: &str) -> String {
    // Match strncpy(buf, src, MAX_SCENE_NAME-1) followed by null term:
    // copies up to MAX_SCENE_NAME-1 bytes of s.
    let bytes = s.as_bytes();
    let n = bytes.len().min(MAX_SCENE_NAME - 1);
    String::from_utf8_lossy(&bytes[..n]).into_owned()
}

fn scene_create(name: &str) -> *mut Scene {
    let scene = Scene {
        name: if name.is_empty() {
            // C uses strcpy with "Untitled Scene" only if name == NULL.
            // Empty C string is still a valid pointer, so it gets copied (resulting in "").
            // Reproduce: empty input becomes empty name.
            String::new()
        } else {
            truncate_to_max_scene_name(name)
        },
        shapes: Vec::new(),
    };
    Box::into_raw(Box::new(scene))
}

fn scene_destroy(scene: *mut Scene) {
    if !scene.is_null() {
        unsafe {
            drop(Box::from_raw(scene));
        }
    }
}

fn scene_add_shape(scene: *mut Scene, shape: *mut Shape) -> i32 {
    if scene.is_null() || shape.is_null() {
        return -1;
    }
    let s = unsafe { &mut *scene };
    if s.shapes.len() >= MAX_SHAPES_IN_SCENE {
        eprintln!("Error: Scene is full");
        return -1;
    }
    s.shapes.push(shape);
    0
}

fn scene_remove_shape(scene: *mut Scene, index: i32) -> i32 {
    if scene.is_null() {
        return -1;
    }
    let s = unsafe { &mut *scene };
    if index < 0 || (index as usize) >= s.shapes.len() {
        return -1;
    }
    s.shapes.remove(index as usize);
    0
}

fn scene_print(scene: *const Scene) {
    if scene.is_null() {
        println!("(null scene)");
        return;
    }
    let s = unsafe { &*scene };
    println!("\n=== Scene: {} ===", s.name);
    println!("Contains {} shape(s)\n", s.shapes.len());
    for (i, &sp) in s.shapes.iter().enumerate() {
        println!("Shape #{}:", i + 1);
        shape_print(sp);
        println!();
    }
}

fn scene_equals(s1: *const Scene, s2: *const Scene) -> i32 {
    if s1.is_null() || s2.is_null() {
        return 0;
    }
    let a = unsafe { &*s1 };
    let b = unsafe { &*s2 };
    if a.shapes.len() != b.shapes.len() {
        return 0;
    }
    let mut matched = vec![false; b.shapes.len()];
    for i in 0..a.shapes.len() {
        let mut found = false;
        for j in 0..b.shapes.len() {
            if !matched[j] && shape_equals(a.shapes[i], b.shapes[j]) != 0 {
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

fn scene_save(scene: *const Scene, filename: &str) -> i32 {
    if scene.is_null() || filename.is_empty() {
        // C only checks for NULL pointers. Empty filename will fail to open below; matches behavior.
    }
    if scene.is_null() {
        return -1;
    }
    let s = unsafe { &*scene };
    let mut file = match File::create(filename) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Error: Could not open file '{}' for writing", filename);
            return -1;
        }
    };
    if writeln!(file, "{}", s.name).is_err() {
        return -1;
    }
    if writeln!(file, "{}", s.shapes.len()).is_err() {
        return -1;
    }
    for &sp in &s.shapes {
        let t = unsafe { (*sp).type_ };
        if writeln!(file, "{}", t).is_err() {
            return -1;
        }
    }
    drop(file);
    println!("Scene saved to '{}'", filename);
    0
}

fn parse_int_at(bytes: &[u8], pos: &mut usize) -> Option<i32> {
    // Mimic fscanf("%d") with optional preceding whitespace.
    while *pos < bytes.len() && (bytes[*pos] as char).is_ascii_whitespace() {
        *pos += 1;
    }
    let mut neg = false;
    if *pos < bytes.len() && (bytes[*pos] == b'+' || bytes[*pos] == b'-') {
        if bytes[*pos] == b'-' {
            neg = true;
        }
        *pos += 1;
    }
    let start = *pos;
    while *pos < bytes.len() && bytes[*pos].is_ascii_digit() {
        *pos += 1;
    }
    if start == *pos {
        return None;
    }
    let s = std::str::from_utf8(&bytes[start..*pos]).ok()?;
    let mut v: i32 = s.parse().ok()?;
    if neg {
        v = -v;
    }
    Some(v)
}

fn scene_load(filename: &str) -> *mut Scene {
    if filename.is_empty() {
        // C only checks NULL pointer.
    }
    let bytes = match std::fs::read(filename) {
        Ok(b) => b,
        Err(_) => {
            eprintln!("Error: Could not open file '{}' for reading", filename);
            return std::ptr::null_mut();
        }
    };
    let mut pos = 0usize;
    // fgets(name, MAX_SCENE_NAME, file): read up to MAX_SCENE_NAME-1 bytes or until newline (inclusive).
    let mut name_buf: Vec<u8> = Vec::new();
    let max_read = MAX_SCENE_NAME - 1;
    while name_buf.len() < max_read && pos < bytes.len() {
        let b = bytes[pos];
        pos += 1;
        name_buf.push(b);
        if b == b'\n' {
            break;
        }
    }
    if name_buf.is_empty() {
        return std::ptr::null_mut();
    }
    // strcspn(name, "\n") => 0 the newline
    if let Some(i) = name_buf.iter().position(|&c| c == b'\n') {
        name_buf.truncate(i);
    }
    let name = String::from_utf8_lossy(&name_buf).into_owned();

    let scene_ptr = scene_create(&name);
    if scene_ptr.is_null() {
        return std::ptr::null_mut();
    }

    let shape_count = match parse_int_at(&bytes, &mut pos) {
        Some(n) => n,
        None => {
            scene_destroy(scene_ptr);
            return std::ptr::null_mut();
        }
    };

    for _ in 0..shape_count {
        let t = match parse_int_at(&bytes, &mut pos) {
            Some(n) => n,
            None => {
                scene_destroy(scene_ptr);
                return std::ptr::null_mut();
            }
        };
        let shape_ptr = shape_get(t);
        if !shape_ptr.is_null() {
            scene_add_shape(scene_ptr, shape_ptr);
        }
    }

    println!("Scene loaded from '{}'", filename);
    scene_ptr
}

fn scene_list_shapes(scene: *const Scene) {
    if scene.is_null() {
        println!("(null scene)");
        return;
    }
    let s = unsafe { &*scene };
    println!("\nScene: {}", s.name);
    println!("Shapes ({}):", s.shapes.len());
    for (i, &sp) in s.shapes.iter().enumerate() {
        let name = unsafe { &(*sp).name };
        // Match C %p formatting: lowercase hex with 0x prefix.
        println!("  {}. {} (ptr: {:p})", i + 1, name, sp);
    }
}

// ------------- Stdin reader -------------
struct StdinReader {
    reader: io::BufReader<io::Stdin>,
    peeked: Option<u8>,
}

impl StdinReader {
    fn new() -> Self {
        Self {
            reader: io::BufReader::new(io::stdin()),
            peeked: None,
        }
    }

    fn read_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked.take() {
            return Some(b);
        }
        let mut buf = [0u8; 1];
        match self.reader.read(&mut buf) {
            Ok(0) => None,
            Ok(_) => Some(buf[0]),
            Err(_) => None,
        }
    }

    fn peek_byte(&mut self) -> Option<u8> {
        if self.peeked.is_some() {
            return self.peeked;
        }
        let b = self.read_byte()?;
        self.peeked = Some(b);
        Some(b)
    }

    /// fgets-like: read up to max-1 bytes or until '\n' (inclusive).
    /// Returns None on immediate EOF (no bytes read).
    fn fgets(&mut self, max: usize) -> Option<String> {
        if max == 0 {
            return None;
        }
        let mut bytes = Vec::new();
        let limit = max - 1;
        loop {
            if bytes.len() >= limit {
                break;
            }
            match self.read_byte() {
                None => break,
                Some(b) => {
                    bytes.push(b);
                    if b == b'\n' {
                        break;
                    }
                }
            }
        }
        if bytes.is_empty() {
            return None;
        }
        Some(String::from_utf8_lossy(&bytes).into_owned())
    }

    /// scanf("%d", ...) – returns None if no integer parsed.
    fn scanf_int(&mut self) -> Option<i32> {
        loop {
            match self.peek_byte() {
                Some(b) if (b as char).is_ascii_whitespace() => {
                    self.read_byte();
                }
                _ => break,
            }
        }
        let mut neg = false;
        match self.peek_byte() {
            Some(b'-') => {
                self.read_byte();
                neg = true;
            }
            Some(b'+') => {
                self.read_byte();
            }
            _ => {}
        }
        let mut digits = String::new();
        while let Some(b) = self.peek_byte() {
            if (b as char).is_ascii_digit() {
                digits.push(b as char);
                self.read_byte();
            } else {
                break;
            }
        }
        if digits.is_empty() {
            return None;
        }
        let mut v: i32 = digits.parse().ok()?;
        if neg {
            v = -v;
        }
        Some(v)
    }

    /// Equivalent to: while (getchar() != '\n');
    /// Reads bytes until '\n' or EOF.
    fn consume_until_newline(&mut self) {
        loop {
            match self.read_byte() {
                None => break, // C would loop forever on EOF; we exit gracefully.
                Some(b'\n') => break,
                Some(_) => continue,
            }
        }
    }
}

fn strip_newline(s: &str) -> String {
    // strcspn(name, "\n") effectively zero-terminates at '\n'.
    match s.find('\n') {
        Some(i) => s[..i].to_string(),
        None => s.to_string(),
    }
}

fn sscanf_int(s: &str) -> Option<i32> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    let mut neg = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            neg = true;
        }
        i += 1;
    }
    let start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if start == i {
        return None;
    }
    let digits = std::str::from_utf8(&bytes[start..i]).ok()?;
    let mut v: i32 = digits.parse().ok()?;
    if neg {
        v = -v;
    }
    Some(v)
}

// ------------- Menu functions -------------
fn flush_stdout() {
    let _ = io::stdout().flush();
}

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
    flush_stdout();
}

fn view_all_shapes() {
    println!("\n=== Available Shapes ===");
    for i in 0..SHAPE_COUNT {
        print!("\n{}. ", i + 1);
        // Note: shape_print uses println for the shape name line.
        // C does: printf("\n%d. ", i+1); shape_print(...);
        // shape_print first prints "%s:\n" → name line.
        shape_print(shape_get(i));
    }
}

fn create_new_scene(reader: &mut StdinReader) {
    unsafe {
        if SCENE_COUNT >= MAX_SCENES {
            println!("Error: Maximum scenes reached");
            return;
        }
    }
    print!("Enter scene name: ");
    flush_stdout();
    let line = match reader.fgets(MAX_SCENE_NAME) {
        Some(l) => l,
        None => return,
    };
    let name = strip_newline(&line);
    unsafe {
        let new_scene = scene_create(&name);
        SCENES[SCENE_COUNT] = new_scene;
        if !SCENES[SCENE_COUNT].is_null() {
            println!("Scene '{}' created (index {})", name, SCENE_COUNT);
            SCENE_COUNT += 1;
        } else {
            println!("Error creating scene");
        }
    }
}

fn add_shape_to_scene(reader: &mut StdinReader) {
    unsafe {
        if SCENE_COUNT == 0 {
            println!("No scenes available. Create a scene first.");
            return;
        }
        print!("Select scene (0-{}): ", SCENE_COUNT - 1);
        flush_stdout();
        let scene_idx = match reader.scanf_int() {
            Some(n) => n,
            None => {
                println!("Invalid input");
                reader.consume_until_newline();
                return;
            }
        };
        reader.consume_until_newline();

        if scene_idx < 0 || (scene_idx as usize) >= SCENE_COUNT {
            println!("Invalid scene index");
            return;
        }

        println!("\nSelect shape to add:");
        for i in 0..SHAPE_COUNT {
            println!("{}. {}", i, shape_type_name(i));
        }
        print!("Choice: ");
        flush_stdout();

        let shape_type = match reader.scanf_int() {
            Some(n) => n,
            None => {
                println!("Invalid input");
                reader.consume_until_newline();
                return;
            }
        };
        reader.consume_until_newline();

        if shape_type < 0 || shape_type >= SHAPE_COUNT {
            println!("Invalid shape type");
            return;
        }

        let shape = shape_get(shape_type);
        if scene_add_shape(SCENES[scene_idx as usize], shape) == 0 {
            let name = &(*shape).name;
            println!(
                "Shape '{}' added to scene (reusing singleton at {:p})",
                name, shape
            );
        } else {
            println!("Error adding shape");
        }
    }
}

fn remove_shape_from_scene(reader: &mut StdinReader) {
    unsafe {
        if SCENE_COUNT == 0 {
            println!("No scenes available");
            return;
        }
        print!("Select scene (0-{}): ", SCENE_COUNT - 1);
        flush_stdout();
        let scene_idx = match reader.scanf_int() {
            Some(n) => n,
            None => {
                println!("Invalid input");
                reader.consume_until_newline();
                return;
            }
        };
        reader.consume_until_newline();

        if scene_idx < 0 || (scene_idx as usize) >= SCENE_COUNT {
            println!("Invalid scene index");
            return;
        }

        scene_list_shapes(SCENES[scene_idx as usize]);

        let count = (*SCENES[scene_idx as usize]).shapes.len();
        if count == 0 {
            println!("Scene is empty");
            return;
        }

        print!("Select shape to remove (1-{}): ", count);
        flush_stdout();
        let shape_idx = match reader.scanf_int() {
            Some(n) => n,
            None => {
                println!("Invalid input");
                reader.consume_until_newline();
                return;
            }
        };
        reader.consume_until_newline();

        if scene_remove_shape(SCENES[scene_idx as usize], shape_idx - 1) == 0 {
            println!("Shape removed");
        } else {
            println!("Error removing shape");
        }
    }
}

fn view_scene(reader: &mut StdinReader) {
    unsafe {
        if SCENE_COUNT == 0 {
            println!("No scenes available");
            return;
        }
        print!("Select scene (0-{}): ", SCENE_COUNT - 1);
        flush_stdout();
        let scene_idx = match reader.scanf_int() {
            Some(n) => n,
            None => {
                println!("Invalid input");
                reader.consume_until_newline();
                return;
            }
        };
        reader.consume_until_newline();

        if scene_idx < 0 || (scene_idx as usize) >= SCENE_COUNT {
            println!("Invalid scene index");
            return;
        }
        scene_print(SCENES[scene_idx as usize]);
    }
}

fn list_all_scenes() {
    unsafe {
        println!("\n=== All Scenes ===");
        if SCENE_COUNT == 0 {
            println!("No scenes created yet");
            return;
        }
        for i in 0..SCENE_COUNT {
            let s = &*SCENES[i];
            println!("{}. {} ({} shapes)", i, s.name, s.shapes.len());
        }
    }
}

fn save_scene_to_file(reader: &mut StdinReader) {
    unsafe {
        if SCENE_COUNT == 0 {
            println!("No scenes available");
            return;
        }
        print!("Select scene (0-{}): ", SCENE_COUNT - 1);
        flush_stdout();
        let scene_idx = match reader.scanf_int() {
            Some(n) => n,
            None => {
                println!("Invalid input");
                reader.consume_until_newline();
                return;
            }
        };
        reader.consume_until_newline();

        if scene_idx < 0 || (scene_idx as usize) >= SCENE_COUNT {
            println!("Invalid scene index");
            return;
        }

        print!("Enter filename: ");
        flush_stdout();
        let line = match reader.fgets(256) {
            Some(l) => l,
            None => return,
        };
        let filename = strip_newline(&line);
        scene_save(SCENES[scene_idx as usize], &filename);
    }
}

fn load_scene_from_file(reader: &mut StdinReader) {
    unsafe {
        if SCENE_COUNT >= MAX_SCENES {
            println!("Error: Maximum scenes reached");
            return;
        }
        print!("Enter filename: ");
        flush_stdout();
        let line = match reader.fgets(256) {
            Some(l) => l,
            None => return,
        };
        let filename = strip_newline(&line);

        let scene = scene_load(&filename);
        if !scene.is_null() {
            SCENES[SCENE_COUNT] = scene;
            SCENE_COUNT += 1;
            println!("Scene loaded (index {})", SCENE_COUNT - 1);
        }
    }
}

fn compare_shapes(reader: &mut StdinReader) {
    println!("\nSelect first shape (0-{}):", SHAPE_COUNT - 1);
    for i in 0..SHAPE_COUNT {
        println!("{}. {}", i, shape_type_name(i));
    }
    print!("Choice: ");
    flush_stdout();

    let type1 = match reader.scanf_int() {
        Some(n) => n,
        None => {
            println!("Invalid input");
            reader.consume_until_newline();
            return;
        }
    };
    reader.consume_until_newline();

    print!("\nSelect second shape (0-{}): ", SHAPE_COUNT - 1);
    flush_stdout();
    let type2 = match reader.scanf_int() {
        Some(n) => n,
        None => {
            println!("Invalid input");
            reader.consume_until_newline();
            return;
        }
    };
    reader.consume_until_newline();

    if type1 < 0 || type1 >= SHAPE_COUNT || type2 < 0 || type2 >= SHAPE_COUNT {
        println!("Invalid shape type");
        return;
    }

    let s1 = shape_get(type1);
    let s2 = shape_get(type2);

    unsafe {
        println!("\nShape 1: {} (ptr: {:p})", (*s1).name, s1);
        println!("Shape 2: {} (ptr: {:p})", (*s2).name, s2);
    }
    println!(
        "Comparison of pointers: {}",
        if s1 as *const _ == s2 as *const _ { 1 } else { 0 }
    );

    if shape_equals(s1, s2) != 0 {
        println!("Result: Shapes are EQUAL (same instance)");
    } else {
        println!("Result: Shapes are NOT EQUAL (different instances)");
    }
}

fn compare_scenes(reader: &mut StdinReader) {
    unsafe {
        if SCENE_COUNT < 2 {
            println!("Need at least 2 scenes to compare");
            return;
        }
        print!("Select first scene (0-{}): ", SCENE_COUNT - 1);
        flush_stdout();
        let idx1 = match reader.scanf_int() {
            Some(n) => n,
            None => {
                println!("Invalid input");
                reader.consume_until_newline();
                return;
            }
        };
        reader.consume_until_newline();

        print!("Select second scene (0-{}): ", SCENE_COUNT - 1);
        flush_stdout();
        let idx2 = match reader.scanf_int() {
            Some(n) => n,
            None => {
                println!("Invalid input");
                reader.consume_until_newline();
                return;
            }
        };
        reader.consume_until_newline();

        if idx1 < 0
            || (idx1 as usize) >= SCENE_COUNT
            || idx2 < 0
            || (idx2 as usize) >= SCENE_COUNT
        {
            println!("Invalid scene index");
            return;
        }

        let sc1 = SCENES[idx1 as usize];
        let sc2 = SCENES[idx2 as usize];

        println!(
            "\nScene 1: {} ({} shapes)",
            (*sc1).name,
            (*sc1).shapes.len()
        );
        scene_list_shapes(sc1);

        println!(
            "\nScene 2: {} ({} shapes)",
            (*sc2).name,
            (*sc2).shapes.len()
        );
        scene_list_shapes(sc2);

        if scene_equals(sc1, sc2) != 0 {
            println!("\nResult: Scenes are EQUAL (1:1 correspondence)");
        } else {
            println!("\nResult: Scenes are NOT EQUAL");
        }
    }
}

fn delete_scene(reader: &mut StdinReader) {
    unsafe {
        if SCENE_COUNT == 0 {
            println!("No scenes available");
            return;
        }
        print!("Select scene to delete (0-{}): ", SCENE_COUNT - 1);
        flush_stdout();
        let scene_idx = match reader.scanf_int() {
            Some(n) => n,
            None => {
                println!("Invalid input");
                reader.consume_until_newline();
                return;
            }
        };
        reader.consume_until_newline();

        if scene_idx < 0 || (scene_idx as usize) >= SCENE_COUNT {
            println!("Invalid scene index");
            return;
        }

        scene_destroy(SCENES[scene_idx as usize]);
        for i in (scene_idx as usize)..(SCENE_COUNT - 1) {
            SCENES[i] = SCENES[i + 1];
        }
        SCENE_COUNT -= 1;
        println!("Scene deleted");
    }
}

fn main() {
    println!("╔════════════════════════════════════════╗");
    println!("║  ASCII ART DRAWING APPLICATION        ║");
    println!("║  Child-Friendly Shape Editor           ║");
    println!("╚════════════════════════════════════════╝");

    shape_manager_init();

    let mut reader = StdinReader::new();

    loop {
        print_menu();

        let line = match reader.fgets(256) {
            Some(l) => l,
            None => break,
        };

        let choice = match sscanf_int(&line) {
            Some(n) => n,
            None => {
                println!("Invalid input");
                continue;
            }
        };

        match choice {
            1 => view_all_shapes(),
            2 => create_new_scene(&mut reader),
            3 => add_shape_to_scene(&mut reader),
            4 => remove_shape_from_scene(&mut reader),
            5 => view_scene(&mut reader),
            6 => list_all_scenes(),
            7 => save_scene_to_file(&mut reader),
            8 => load_scene_from_file(&mut reader),
            9 => compare_shapes(&mut reader),
            10 => compare_scenes(&mut reader),
            11 => delete_scene(&mut reader),
            12 => {
                println!("\nCleaning up and exiting...");
                unsafe {
                    for i in 0..SCENE_COUNT {
                        scene_destroy(SCENES[i]);
                    }
                }
                shape_manager_cleanup();
                println!("Goodbye!");
                flush_stdout();
                return;
            }
            _ => {
                println!("Invalid choice");
            }
        }
    }

    unsafe {
        for i in 0..SCENE_COUNT {
            scene_destroy(SCENES[i]);
        }
    }
    shape_manager_cleanup();
    flush_stdout();
}
