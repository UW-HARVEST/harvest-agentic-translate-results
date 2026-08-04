use std::fs::File;
use std::io::{self, Read, Write};
use std::path::PathBuf;

#[cfg(unix)]
use std::ffi::OsString;
#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;

const MAX_SCENES: usize = 10;
const MAX_SHAPES_IN_SCENE: usize = 50;
const MAX_SCENE_NAME: usize = 64;
const SHAPE_COUNT: usize = 10;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ShapeType {
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

struct Shape {
    typ: ShapeType,
    name: &'static [u8],
    art: &'static [&'static [u8]],
}

struct Scene {
    name: Vec<u8>,
    shapes: Vec<&'static Shape>,
}

struct Input {
    data: Vec<u8>,
    pos: usize,
}

impl Input {
    fn new(data: Vec<u8>) -> Self {
        Self { data, pos: 0 }
    }

    fn fgets(&mut self, size: usize) -> Option<Vec<u8>> {
        if self.pos >= self.data.len() {
            return None;
        }
        let mut out = Vec::new();
        let max = size.saturating_sub(1);
        while out.len() < max && self.pos < self.data.len() {
            let b = self.data[self.pos];
            self.pos += 1;
            out.push(b);
            if b == b'\n' {
                break;
            }
        }
        Some(out)
    }

    fn scanf_int(&mut self) -> Option<i32> {
        while self.pos < self.data.len() && is_space(self.data[self.pos]) {
            self.pos += 1;
        }
        let start = self.pos;
        let mut sign = 1i64;
        if self.pos < self.data.len()
            && (self.data[self.pos] == b'+' || self.data[self.pos] == b'-')
        {
            if self.data[self.pos] == b'-' {
                sign = -1;
            }
            self.pos += 1;
        }
        let digit_start = self.pos;
        let mut value = 0i64;
        while self.pos < self.data.len() && self.data[self.pos].is_ascii_digit() {
            value = value
                .saturating_mul(10)
                .saturating_add((self.data[self.pos] - b'0') as i64);
            self.pos += 1;
        }
        if self.pos == digit_start {
            self.pos = if start == self.data.len() {
                start
            } else {
                digit_start
            };
            return None;
        }
        Some((value.saturating_mul(sign)) as i32)
    }

    fn consume_until_newline(&mut self) {
        while self.pos < self.data.len() {
            let b = self.data[self.pos];
            self.pos += 1;
            if b == b'\n' {
                break;
            }
        }
    }
}

fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\n' | b'\t' | b'\r' | 0x0b | 0x0c)
}

fn sscanf_int(line: &[u8]) -> Option<i32> {
    let mut pos = 0usize;
    while pos < line.len() && is_space(line[pos]) {
        pos += 1;
    }
    let mut sign = 1i64;
    if pos < line.len() && (line[pos] == b'+' || line[pos] == b'-') {
        if line[pos] == b'-' {
            sign = -1;
        }
        pos += 1;
    }
    let digit_start = pos;
    let mut value = 0i64;
    while pos < line.len() && line[pos].is_ascii_digit() {
        value = value
            .saturating_mul(10)
            .saturating_add((line[pos] - b'0') as i64);
        pos += 1;
    }
    if pos == digit_start {
        None
    } else {
        Some((value.saturating_mul(sign)) as i32)
    }
}

fn strip_newline(mut s: Vec<u8>) -> Vec<u8> {
    if let Some(pos) = s.iter().position(|&b| b == b'\n') {
        s.truncate(pos);
    }
    s
}

fn c_prefix(bytes: &[u8]) -> &[u8] {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    &bytes[..end]
}

fn scene_create(name: Option<&[u8]>) -> Scene {
    let mut scene_name = match name {
        Some(n) => c_prefix(n).to_vec(),
        None => b"Untitled Scene".to_vec(),
    };
    if scene_name.len() > MAX_SCENE_NAME - 1 {
        scene_name.truncate(MAX_SCENE_NAME - 1);
    }
    Scene {
        name: scene_name,
        shapes: Vec::new(),
    }
}

static TREE_ART: [&[u8]; 7] = [
    b"    /\\    ",
    b"   /  \\   ",
    b"  /____\\  ",
    b"  /    \\  ",
    b" /______\\ ",
    b"    ||    ",
    b"    ||    ",
];
static TRACTOR_ART: [&[u8]; 6] = [
    b"      ________     ",
    b"     |        |___ ",
    b"     |  []  []|   |",
    b"  ___|________|___|",
    b" /  o        o   \\",
    b"|___|        |___| ",
];
static HOUSE_ART: [&[u8]; 7] = [
    b"     /\\     ",
    b"    /  \\    ",
    b"   /____\\   ",
    b"   |    |   ",
    b"   | [] |   ",
    b"   |    |   ",
    b"   |____|   ",
];
static SUN_ART: [&[u8]; 7] = [
    b"  \\  |  / ",
    b"   \\ | /  ",
    b"--- (@) ---",
    b"   / | \\  ",
    b"  /  |  \\ ",
    b"          ",
    b"          ",
];
static CLOUD_ART: [&[u8]; 4] = [
    b"   _____       ",
    b"  /     \\_    ",
    b" /  ___  _\\  ",
    b"(__/   \\_)   ",
];
static FLOWER_ART: [&[u8]; 7] = [
    b"  \\|/  ",
    b" -(@)- ",
    b"  /|\\  ",
    b"   |   ",
    b"   |   ",
    b"  / \\  ",
    b" /   \\ ",
];
static CAR_ART: [&[u8]; 4] = [
    b"  ____       ",
    b" /|_||_\\____ ",
    b"( o     o  ) ",
    b" -----------  ",
];
static STAR_ART: [&[u8]; 5] = [
    b"    *    ",
    b"   ***   ",
    b"  *****  ",
    b" ******* ",
    b"*********",
];
static HEART_ART: [&[u8]; 6] = [
    b" *** ***  ",
    b"*********  ",
    b"*********  ",
    b" ******* ",
    b"  *****  ",
    b"   ***   ",
];
static RAINBOW_ART: [&[u8]; 5] = [
    b"      _______      ",
    b"    /         \\    ",
    b"   /           \\   ",
    b"  /             \\  ",
    b" /               \\ ",
];

static SHAPES: [Shape; SHAPE_COUNT] = [
    Shape {
        typ: ShapeType::Tree,
        name: b"Tree",
        art: &TREE_ART,
    },
    Shape {
        typ: ShapeType::Tractor,
        name: b"Tractor",
        art: &TRACTOR_ART,
    },
    Shape {
        typ: ShapeType::House,
        name: b"House",
        art: &HOUSE_ART,
    },
    Shape {
        typ: ShapeType::Sun,
        name: b"Sun",
        art: &SUN_ART,
    },
    Shape {
        typ: ShapeType::Cloud,
        name: b"Cloud",
        art: &CLOUD_ART,
    },
    Shape {
        typ: ShapeType::Flower,
        name: b"Flower",
        art: &FLOWER_ART,
    },
    Shape {
        typ: ShapeType::Car,
        name: b"Car",
        art: &CAR_ART,
    },
    Shape {
        typ: ShapeType::Star,
        name: b"Star",
        art: &STAR_ART,
    },
    Shape {
        typ: ShapeType::Heart,
        name: b"Heart",
        art: &HEART_ART,
    },
    Shape {
        typ: ShapeType::Rainbow,
        name: b"Rainbow",
        art: &RAINBOW_ART,
    },
];

fn shape_get(typ: i32) -> Option<&'static Shape> {
    if typ < 0 || typ as usize >= SHAPE_COUNT {
        None
    } else {
        Some(&SHAPES[typ as usize])
    }
}

fn shape_type_name(typ: i32) -> &'static [u8] {
    match typ {
        0 => b"Tree",
        1 => b"Tractor",
        2 => b"House",
        3 => b"Sun",
        4 => b"Cloud",
        5 => b"Flower",
        6 => b"Car",
        7 => b"Star",
        8 => b"Heart",
        9 => b"Rainbow",
        _ => b"Unknown",
    }
}

fn shape_print(out: &mut impl Write, shape: Option<&Shape>) -> io::Result<()> {
    match shape {
        Some(shape) => {
            out.write_all(shape.name)?;
            out.write_all(b":\n")?;
            for line in shape.art {
                out.write_all(line)?;
                out.write_all(b"\n")?;
            }
        }
        None => out.write_all(b"(null shape)\n")?,
    }
    Ok(())
}

fn shape_equals(s1: &Shape, s2: &Shape) -> bool {
    std::ptr::eq(s1, s2)
}

fn scene_add_shape(scene: &mut Scene, shape: Option<&'static Shape>) -> i32 {
    let Some(shape) = shape else {
        return -1;
    };
    if scene.shapes.len() >= MAX_SHAPES_IN_SCENE {
        let _ = io::stderr().write_all(b"Error: Scene is full\n");
        return -1;
    }
    scene.shapes.push(shape);
    0
}

fn scene_remove_shape(scene: &mut Scene, index: i32) -> i32 {
    if index < 0 || index as usize >= scene.shapes.len() {
        return -1;
    }
    scene.shapes.remove(index as usize);
    0
}

fn scene_print(out: &mut impl Write, scene: Option<&Scene>) -> io::Result<()> {
    match scene {
        Some(scene) => {
            out.write_all(b"\n=== Scene: ")?;
            out.write_all(c_prefix(&scene.name))?;
            writeln!(out, " ===")?;
            writeln!(out, "Contains {} shape(s)\n", scene.shapes.len())?;
            for (i, shape) in scene.shapes.iter().enumerate() {
                writeln!(out, "Shape #{}:", i + 1)?;
                shape_print(out, Some(shape))?;
                out.write_all(b"\n")?;
            }
        }
        None => out.write_all(b"(null scene)\n")?,
    }
    Ok(())
}

fn scene_equals(s1: Option<&Scene>, s2: Option<&Scene>) -> bool {
    let (Some(s1), Some(s2)) = (s1, s2) else {
        return false;
    };
    if s1.shapes.len() != s2.shapes.len() {
        return false;
    }
    let mut matched = [false; MAX_SHAPES_IN_SCENE];
    for shape1 in &s1.shapes {
        let mut found = false;
        for (j, shape2) in s2.shapes.iter().enumerate() {
            if !matched[j] && shape_equals(shape1, shape2) {
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

#[cfg(unix)]
fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    PathBuf::from(OsString::from_vec(bytes.to_vec()))
}

#[cfg(not(unix))]
fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
}

fn scene_save(out: &mut impl Write, scene: Option<&Scene>, filename: &[u8]) -> i32 {
    let Some(scene) = scene else {
        return -1;
    };
    let mut file = match File::create(path_from_bytes(c_prefix(filename))) {
        Ok(file) => file,
        Err(_) => {
            let mut err = io::stderr();
            let _ = err.write_all(b"Error: Could not open file '");
            let _ = err.write_all(c_prefix(filename));
            let _ = err.write_all(b"' for writing\n");
            return -1;
        }
    };

    let _ = file.write_all(c_prefix(&scene.name));
    let _ = file.write_all(b"\n");
    let _ = writeln!(file, "{}", scene.shapes.len());
    for shape in &scene.shapes {
        let _ = writeln!(file, "{}", shape_type_number(shape));
    }
    drop(file);

    out.write_all(b"Scene saved to '").ok();
    out.write_all(c_prefix(filename)).ok();
    out.write_all(b"'\n").ok();
    0
}

fn shape_type_number(shape: &Shape) -> i32 {
    shape.typ as i32
}

fn read_file_int(input: &[u8], pos: &mut usize) -> Option<i32> {
    while *pos < input.len() && is_space(input[*pos]) {
        *pos += 1;
    }
    let mut sign = 1i64;
    if *pos < input.len() && (input[*pos] == b'+' || input[*pos] == b'-') {
        if input[*pos] == b'-' {
            sign = -1;
        }
        *pos += 1;
    }
    let digit_start = *pos;
    let mut value = 0i64;
    while *pos < input.len() && input[*pos].is_ascii_digit() {
        value = value
            .saturating_mul(10)
            .saturating_add((input[*pos] - b'0') as i64);
        *pos += 1;
    }
    if *pos == digit_start {
        return None;
    }
    while *pos < input.len() && is_space(input[*pos]) {
        *pos += 1;
    }
    Some((value.saturating_mul(sign)) as i32)
}

fn scene_load(out: &mut impl Write, filename: &[u8]) -> Option<Scene> {
    let mut file = match File::open(path_from_bytes(c_prefix(filename))) {
        Ok(file) => file,
        Err(_) => {
            let mut err = io::stderr();
            let _ = err.write_all(b"Error: Could not open file '");
            let _ = err.write_all(c_prefix(filename));
            let _ = err.write_all(b"' for reading\n");
            return None;
        }
    };
    let mut data = Vec::new();
    if file.read_to_end(&mut data).is_err() {
        return None;
    }
    if data.is_empty() {
        return None;
    }
    let mut pos = 0usize;
    let mut name = Vec::new();
    while name.len() < MAX_SCENE_NAME - 1 && pos < data.len() {
        let b = data[pos];
        pos += 1;
        name.push(b);
        if b == b'\n' {
            break;
        }
    }
    if name.is_empty() {
        return None;
    }
    name = strip_newline(name);
    let mut scene = scene_create(Some(&name));
    let shape_count = match read_file_int(&data, &mut pos) {
        Some(count) => count,
        None => return None,
    };
    for _ in 0..shape_count {
        let typ = match read_file_int(&data, &mut pos) {
            Some(typ) => typ,
            None => return None,
        };
        if let Some(shape) = shape_get(typ) {
            scene_add_shape(&mut scene, Some(shape));
        }
    }
    out.write_all(b"Scene loaded from '").ok();
    out.write_all(c_prefix(filename)).ok();
    out.write_all(b"'\n").ok();
    Some(scene)
}

fn scene_list_shapes(out: &mut impl Write, scene: Option<&Scene>) -> io::Result<()> {
    match scene {
        Some(scene) => {
            out.write_all(b"\nScene: ")?;
            out.write_all(c_prefix(&scene.name))?;
            writeln!(out)?;
            writeln!(out, "Shapes ({}):", scene.shapes.len())?;
            for (i, shape) in scene.shapes.iter().enumerate() {
                write!(out, "  {}. ", i + 1)?;
                out.write_all(shape.name)?;
                writeln!(out, " (ptr: {:p})", *shape as *const Shape)?;
            }
        }
        None => out.write_all(b"(null scene)\n")?,
    }
    Ok(())
}

fn print_menu(out: &mut impl Write) -> io::Result<()> {
    out.write_all(b"\n")?;
    out.write_all(b"=========================================\n")?;
    out.write_all(b"  ASCII ART DRAWING APPLICATION\n")?;
    out.write_all(b"=========================================\n")?;
    out.write_all(b"1. View all available shapes\n")?;
    out.write_all(b"2. Create new scene\n")?;
    out.write_all(b"3. Add shape to scene\n")?;
    out.write_all(b"4. Remove shape from scene\n")?;
    out.write_all(b"5. View scene\n")?;
    out.write_all(b"6. List all scenes\n")?;
    out.write_all(b"7. Save scene\n")?;
    out.write_all(b"8. Load scene\n")?;
    out.write_all(b"9. Compare two shapes\n")?;
    out.write_all(b"10. Compare two scenes\n")?;
    out.write_all(b"11. Delete scene\n")?;
    out.write_all(b"12. Exit\n")?;
    out.write_all(b"=========================================\n")?;
    out.write_all(b"Choice: ")?;
    Ok(())
}

struct App {
    scenes: Vec<Scene>,
}

impl App {
    fn view_all_shapes(&self, out: &mut impl Write) -> io::Result<()> {
        out.write_all(b"\n=== Available Shapes ===\n")?;
        for i in 0..SHAPE_COUNT {
            write!(out, "\n{}. ", i + 1)?;
            shape_print(out, shape_get(i as i32))?;
        }
        Ok(())
    }

    fn create_new_scene(&mut self, input: &mut Input, out: &mut impl Write) -> io::Result<()> {
        if self.scenes.len() >= MAX_SCENES {
            out.write_all(b"Error: Maximum scenes reached\n")?;
            return Ok(());
        }
        out.write_all(b"Enter scene name: ")?;
        let Some(name) = input.fgets(MAX_SCENE_NAME) else {
            return Ok(());
        };
        let name = strip_newline(name);
        let scene = scene_create(Some(&name));
        self.scenes.push(scene);
        out.write_all(b"Scene '")?;
        out.write_all(c_prefix(&name))?;
        writeln!(out, "' created (index {})", self.scenes.len() - 1)?;
        Ok(())
    }

    fn add_shape_to_scene(&mut self, input: &mut Input, out: &mut impl Write) -> io::Result<()> {
        if self.scenes.is_empty() {
            out.write_all(b"No scenes available. Create a scene first.\n")?;
            return Ok(());
        }
        write!(out, "Select scene (0-{}): ", self.scenes.len() - 1)?;
        let Some(scene_idx) = input.scanf_int() else {
            out.write_all(b"Invalid input\n")?;
            input.consume_until_newline();
            return Ok(());
        };
        input.consume_until_newline();
        if scene_idx < 0 || scene_idx as usize >= self.scenes.len() {
            out.write_all(b"Invalid scene index\n")?;
            return Ok(());
        }
        out.write_all(b"\nSelect shape to add:\n")?;
        for i in 0..SHAPE_COUNT {
            write!(out, "{}. ", i)?;
            out.write_all(shape_type_name(i as i32))?;
            out.write_all(b"\n")?;
        }
        out.write_all(b"Choice: ")?;
        let Some(shape_type) = input.scanf_int() else {
            out.write_all(b"Invalid input\n")?;
            input.consume_until_newline();
            return Ok(());
        };
        input.consume_until_newline();
        if shape_type < 0 || shape_type as usize >= SHAPE_COUNT {
            out.write_all(b"Invalid shape type\n")?;
            return Ok(());
        }
        let shape = shape_get(shape_type).unwrap();
        if scene_add_shape(&mut self.scenes[scene_idx as usize], Some(shape)) == 0 {
            out.write_all(b"Shape '")?;
            out.write_all(shape.name)?;
            writeln!(
                out,
                "' added to scene (reusing singleton at {:p})",
                shape as *const Shape
            )?;
        } else {
            out.write_all(b"Error adding shape\n")?;
        }
        Ok(())
    }

    fn remove_shape_from_scene(
        &mut self,
        input: &mut Input,
        out: &mut impl Write,
    ) -> io::Result<()> {
        if self.scenes.is_empty() {
            out.write_all(b"No scenes available\n")?;
            return Ok(());
        }
        write!(out, "Select scene (0-{}): ", self.scenes.len() - 1)?;
        let Some(scene_idx) = input.scanf_int() else {
            out.write_all(b"Invalid input\n")?;
            input.consume_until_newline();
            return Ok(());
        };
        input.consume_until_newline();
        if scene_idx < 0 || scene_idx as usize >= self.scenes.len() {
            out.write_all(b"Invalid scene index\n")?;
            return Ok(());
        }
        scene_list_shapes(out, Some(&self.scenes[scene_idx as usize]))?;
        if self.scenes[scene_idx as usize].shapes.is_empty() {
            out.write_all(b"Scene is empty\n")?;
            return Ok(());
        }
        write!(
            out,
            "Select shape to remove (1-{}): ",
            self.scenes[scene_idx as usize].shapes.len()
        )?;
        let Some(shape_idx) = input.scanf_int() else {
            out.write_all(b"Invalid input\n")?;
            input.consume_until_newline();
            return Ok(());
        };
        input.consume_until_newline();
        if scene_remove_shape(&mut self.scenes[scene_idx as usize], shape_idx - 1) == 0 {
            out.write_all(b"Shape removed\n")?;
        } else {
            out.write_all(b"Error removing shape\n")?;
        }
        Ok(())
    }

    fn view_scene(&mut self, input: &mut Input, out: &mut impl Write) -> io::Result<()> {
        if self.scenes.is_empty() {
            out.write_all(b"No scenes available\n")?;
            return Ok(());
        }
        write!(out, "Select scene (0-{}): ", self.scenes.len() - 1)?;
        let Some(scene_idx) = input.scanf_int() else {
            out.write_all(b"Invalid input\n")?;
            input.consume_until_newline();
            return Ok(());
        };
        input.consume_until_newline();
        if scene_idx < 0 || scene_idx as usize >= self.scenes.len() {
            out.write_all(b"Invalid scene index\n")?;
            return Ok(());
        }
        scene_print(out, Some(&self.scenes[scene_idx as usize]))?;
        Ok(())
    }

    fn list_all_scenes(&self, out: &mut impl Write) -> io::Result<()> {
        out.write_all(b"\n=== All Scenes ===\n")?;
        if self.scenes.is_empty() {
            out.write_all(b"No scenes created yet\n")?;
            return Ok(());
        }
        for (i, scene) in self.scenes.iter().enumerate() {
            write!(out, "{}. ", i)?;
            out.write_all(c_prefix(&scene.name))?;
            writeln!(out, " ({} shapes)", scene.shapes.len())?;
        }
        Ok(())
    }

    fn save_scene_to_file(&mut self, input: &mut Input, out: &mut impl Write) -> io::Result<()> {
        if self.scenes.is_empty() {
            out.write_all(b"No scenes available\n")?;
            return Ok(());
        }
        write!(out, "Select scene (0-{}): ", self.scenes.len() - 1)?;
        let Some(scene_idx) = input.scanf_int() else {
            out.write_all(b"Invalid input\n")?;
            input.consume_until_newline();
            return Ok(());
        };
        input.consume_until_newline();
        if scene_idx < 0 || scene_idx as usize >= self.scenes.len() {
            out.write_all(b"Invalid scene index\n")?;
            return Ok(());
        }
        out.write_all(b"Enter filename: ")?;
        let Some(filename) = input.fgets(256) else {
            return Ok(());
        };
        let filename = strip_newline(filename);
        scene_save(out, Some(&self.scenes[scene_idx as usize]), &filename);
        Ok(())
    }

    fn load_scene_from_file(&mut self, input: &mut Input, out: &mut impl Write) -> io::Result<()> {
        if self.scenes.len() >= MAX_SCENES {
            out.write_all(b"Error: Maximum scenes reached\n")?;
            return Ok(());
        }
        out.write_all(b"Enter filename: ")?;
        let Some(filename) = input.fgets(256) else {
            return Ok(());
        };
        let filename = strip_newline(filename);
        if let Some(scene) = scene_load(out, &filename) {
            self.scenes.push(scene);
            writeln!(out, "Scene loaded (index {})", self.scenes.len() - 1)?;
        }
        Ok(())
    }

    fn compare_shapes(&mut self, input: &mut Input, out: &mut impl Write) -> io::Result<()> {
        writeln!(out, "\nSelect first shape (0-{}):", SHAPE_COUNT - 1)?;
        for i in 0..SHAPE_COUNT {
            write!(out, "{}. ", i)?;
            out.write_all(shape_type_name(i as i32))?;
            out.write_all(b"\n")?;
        }
        out.write_all(b"Choice: ")?;
        let Some(type1) = input.scanf_int() else {
            out.write_all(b"Invalid input\n")?;
            input.consume_until_newline();
            return Ok(());
        };
        input.consume_until_newline();
        write!(out, "\nSelect second shape (0-{}): ", SHAPE_COUNT - 1)?;
        let Some(type2) = input.scanf_int() else {
            out.write_all(b"Invalid input\n")?;
            input.consume_until_newline();
            return Ok(());
        };
        input.consume_until_newline();
        if type1 < 0 || type1 as usize >= SHAPE_COUNT || type2 < 0 || type2 as usize >= SHAPE_COUNT
        {
            out.write_all(b"Invalid shape type\n")?;
            return Ok(());
        }
        let s1 = shape_get(type1).unwrap();
        let s2 = shape_get(type2).unwrap();
        out.write_all(b"\nShape 1: ")?;
        out.write_all(s1.name)?;
        writeln!(out, " (ptr: {:p})", s1 as *const Shape)?;
        out.write_all(b"Shape 2: ")?;
        out.write_all(s2.name)?;
        writeln!(out, " (ptr: {:p})", s2 as *const Shape)?;
        writeln!(
            out,
            "Comparison of pointers: {}",
            if shape_equals(s1, s2) { 1 } else { 0 }
        )?;
        if shape_equals(s1, s2) {
            out.write_all(b"Result: Shapes are EQUAL (same instance)\n")?;
        } else {
            out.write_all(b"Result: Shapes are NOT EQUAL (different instances)\n")?;
        }
        Ok(())
    }

    fn compare_scenes(&mut self, input: &mut Input, out: &mut impl Write) -> io::Result<()> {
        if self.scenes.len() < 2 {
            out.write_all(b"Need at least 2 scenes to compare\n")?;
            return Ok(());
        }
        write!(out, "Select first scene (0-{}): ", self.scenes.len() - 1)?;
        let Some(idx1) = input.scanf_int() else {
            out.write_all(b"Invalid input\n")?;
            input.consume_until_newline();
            return Ok(());
        };
        input.consume_until_newline();
        write!(out, "Select second scene (0-{}): ", self.scenes.len() - 1)?;
        let Some(idx2) = input.scanf_int() else {
            out.write_all(b"Invalid input\n")?;
            input.consume_until_newline();
            return Ok(());
        };
        input.consume_until_newline();
        if idx1 < 0
            || idx1 as usize >= self.scenes.len()
            || idx2 < 0
            || idx2 as usize >= self.scenes.len()
        {
            out.write_all(b"Invalid scene index\n")?;
            return Ok(());
        }
        let sc1 = &self.scenes[idx1 as usize];
        let sc2 = &self.scenes[idx2 as usize];
        out.write_all(b"\nScene 1: ")?;
        out.write_all(c_prefix(&sc1.name))?;
        writeln!(out, " ({} shapes)", sc1.shapes.len())?;
        scene_list_shapes(out, Some(sc1))?;
        out.write_all(b"\nScene 2: ")?;
        out.write_all(c_prefix(&sc2.name))?;
        writeln!(out, " ({} shapes)", sc2.shapes.len())?;
        scene_list_shapes(out, Some(sc2))?;
        if scene_equals(Some(sc1), Some(sc2)) {
            out.write_all(b"\nResult: Scenes are EQUAL (1:1 correspondence)\n")?;
        } else {
            out.write_all(b"\nResult: Scenes are NOT EQUAL\n")?;
        }
        Ok(())
    }

    fn delete_scene(&mut self, input: &mut Input, out: &mut impl Write) -> io::Result<()> {
        if self.scenes.is_empty() {
            out.write_all(b"No scenes available\n")?;
            return Ok(());
        }
        write!(
            out,
            "Select scene to delete (0-{}): ",
            self.scenes.len() - 1
        )?;
        let Some(scene_idx) = input.scanf_int() else {
            out.write_all(b"Invalid input\n")?;
            input.consume_until_newline();
            return Ok(());
        };
        input.consume_until_newline();
        if scene_idx < 0 || scene_idx as usize >= self.scenes.len() {
            out.write_all(b"Invalid scene index\n")?;
            return Ok(());
        }
        self.scenes.remove(scene_idx as usize);
        out.write_all(b"Scene deleted\n")?;
        Ok(())
    }
}

fn main() -> io::Result<()> {
    let mut stdin_data = Vec::new();
    io::stdin().read_to_end(&mut stdin_data)?;
    let mut input = Input::new(stdin_data);
    let mut out = io::BufWriter::new(io::stdout());

    out.write_all("╔════════════════════════════════════════╗\n".as_bytes())?;
    out.write_all("║  ASCII ART DRAWING APPLICATION        ║\n".as_bytes())?;
    out.write_all("║  Child-Friendly Shape Editor           ║\n".as_bytes())?;
    out.write_all("╚════════════════════════════════════════╝\n".as_bytes())?;

    let mut app = App { scenes: Vec::new() };

    loop {
        print_menu(&mut out)?;
        let Some(line) = input.fgets(256) else {
            break;
        };
        let Some(choice) = sscanf_int(&line) else {
            out.write_all(b"Invalid input\n")?;
            continue;
        };
        match choice {
            1 => app.view_all_shapes(&mut out)?,
            2 => app.create_new_scene(&mut input, &mut out)?,
            3 => app.add_shape_to_scene(&mut input, &mut out)?,
            4 => app.remove_shape_from_scene(&mut input, &mut out)?,
            5 => app.view_scene(&mut input, &mut out)?,
            6 => app.list_all_scenes(&mut out)?,
            7 => app.save_scene_to_file(&mut input, &mut out)?,
            8 => app.load_scene_from_file(&mut input, &mut out)?,
            9 => app.compare_shapes(&mut input, &mut out)?,
            10 => app.compare_scenes(&mut input, &mut out)?,
            11 => app.delete_scene(&mut input, &mut out)?,
            12 => {
                out.write_all(b"\nCleaning up and exiting...\n")?;
                out.write_all(b"Goodbye!\n")?;
                return Ok(());
            }
            _ => out.write_all(b"Invalid choice\n")?,
        }
    }

    Ok(())
}
