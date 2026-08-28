use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

const MAX_SCENES: usize = 10;
const MAX_SHAPES_IN_SCENE: usize = 50;
const SHAPE_COUNT: usize = 10;

struct Shape {
    name: &'static [u8],
    art: &'static [&'static [u8]],
}

struct Scene {
    name: Vec<u8>,
    shapes: Vec<usize>,
}

struct Input<R: BufRead> {
    reader: R,
}

impl<R: BufRead> Input<R> {
    fn new(reader: R) -> Self {
        Self { reader }
    }

    fn peek_byte(&mut self) -> Option<u8> {
        self.reader
            .fill_buf()
            .ok()
            .and_then(|buffer| buffer.first().copied())
    }

    fn read_byte(&mut self) -> Option<u8> {
        let byte = self.peek_byte()?;
        self.reader.consume(1);
        Some(byte)
    }

    fn fgets(&mut self, size: usize) -> Option<Vec<u8>> {
        if size <= 1 {
            return None;
        }

        let mut result = Vec::new();
        while result.len() < size - 1 {
            match self.read_byte() {
                Some(byte) => {
                    result.push(byte);
                    if byte == b'\n' {
                        break;
                    }
                }
                None if result.is_empty() => return None,
                None => break,
            }
        }
        Some(result)
    }

    fn scanf_i32(&mut self) -> Option<i32> {
        while self.peek_byte().is_some_and(is_c_whitespace) {
            self.read_byte();
        }

        let negative = match self.peek_byte() {
            Some(b'+') => {
                self.read_byte();
                false
            }
            Some(b'-') => {
                self.read_byte();
                true
            }
            _ => false,
        };

        let mut saw_digit = false;
        let limit = if negative {
            i64::MAX as u64 + 1
        } else {
            i64::MAX as u64
        };
        let mut value = 0u64;
        while let Some(byte @ b'0'..=b'9') = self.peek_byte() {
            saw_digit = true;
            self.read_byte();
            value = value
                .saturating_mul(10)
                .saturating_add((byte - b'0') as u64)
                .min(limit);
        }

        if !saw_digit {
            return None;
        }

        let signed = if negative && value == i64::MAX as u64 + 1 {
            i64::MIN
        } else if negative {
            -(value as i64)
        } else {
            value as i64
        };
        Some(signed as i32)
    }

    fn consume_through_newline(&mut self) {
        loop {
            match self.read_byte() {
                Some(b'\n') => return,
                Some(_) => {}
                None => std::hint::spin_loop(),
            }
        }
    }

    fn consume_whitespace(&mut self) {
        while self.peek_byte().is_some_and(is_c_whitespace) {
            self.read_byte();
        }
    }
}

fn is_c_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

fn c_string(line: &[u8]) -> Vec<u8> {
    let end = line
        .iter()
        .position(|&byte| byte == b'\0' || byte == b'\n')
        .unwrap_or(line.len());
    line[..end].to_vec()
}

fn sscanf_i32(line: &[u8]) -> Option<i32> {
    let visible_end = line
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(line.len());
    let mut input = Input::new(BufReader::new(&line[..visible_end]));
    input.scanf_i32()
}

fn write_bytes(output: &mut impl Write, bytes: &[u8]) {
    let _ = output.write_all(bytes);
}

fn print_pointer(output: &mut impl Write, shape: &Shape) {
    let _ = write!(output, "0x{:x}", shape as *const Shape as usize);
}

fn make_shapes() -> Vec<Box<Shape>> {
    vec![
        Box::new(Shape {
            name: b"Tree",
            art: &[
                b"    /\\    ",
                b"   /  \\   ",
                b"  /____\\  ",
                b"  /    \\  ",
                b" /______\\ ",
                b"    ||    ",
                b"    ||    ",
            ],
        }),
        Box::new(Shape {
            name: b"Tractor",
            art: &[
                b"      ________     ",
                b"     |        |___ ",
                b"     |  []  []|   |",
                b"  ___|________|___|",
                b" /  o        o   \\",
                b"|___|        |___| ",
            ],
        }),
        Box::new(Shape {
            name: b"House",
            art: &[
                b"     /\\     ",
                b"    /  \\    ",
                b"   /____\\   ",
                b"   |    |   ",
                b"   | [] |   ",
                b"   |    |   ",
                b"   |____|   ",
            ],
        }),
        Box::new(Shape {
            name: b"Sun",
            art: &[
                b"  \\  |  / ",
                b"   \\ | /  ",
                b"--- (@) ---",
                b"   / | \\  ",
                b"  /  |  \\ ",
                b"          ",
                b"          ",
            ],
        }),
        Box::new(Shape {
            name: b"Cloud",
            art: &[
                b"   _____       ",
                b"  /     \\_    ",
                b" /  ___  _\\  ",
                b"(__/   \\_)   ",
            ],
        }),
        Box::new(Shape {
            name: b"Flower",
            art: &[
                b"  \\|/  ",
                b" -(@)- ",
                b"  /|\\  ",
                b"   |   ",
                b"   |   ",
                b"  / \\  ",
                b" /   \\ ",
            ],
        }),
        Box::new(Shape {
            name: b"Car",
            art: &[
                b"  ____       ",
                b" /|_||_\\____ ",
                b"( o     o  ) ",
                b" -----------  ",
            ],
        }),
        Box::new(Shape {
            name: b"Star",
            art: &[
                b"    *    ",
                b"   ***   ",
                b"  *****  ",
                b" ******* ",
                b"*********",
            ],
        }),
        Box::new(Shape {
            name: b"Heart",
            art: &[
                b" *** ***  ",
                b"*********  ",
                b"*********  ",
                b" ******* ",
                b"  *****  ",
                b"   ***   ",
            ],
        }),
        Box::new(Shape {
            name: b"Rainbow",
            art: &[
                b"      _______      ",
                b"    /         \\    ",
                b"   /           \\   ",
                b"  /             \\  ",
                b" /               \\ ",
            ],
        }),
    ]
}

fn shape_print(output: &mut impl Write, shape: &Shape) {
    write_bytes(output, shape.name);
    write_bytes(output, b":\n");
    for line in shape.art {
        write_bytes(output, line);
        write_bytes(output, b"\n");
    }
}

fn scene_add_shape(scene: &mut Scene, shape_type: usize, error: &mut impl Write) -> Result<(), ()> {
    if scene.shapes.len() >= MAX_SHAPES_IN_SCENE {
        write_bytes(error, b"Error: Scene is full\n");
        return Err(());
    }
    scene.shapes.push(shape_type);
    Ok(())
}

fn scene_remove_shape(scene: &mut Scene, index: i32) -> Result<(), ()> {
    if index < 0 || index as usize >= scene.shapes.len() {
        return Err(());
    }
    scene.shapes.remove(index as usize);
    Ok(())
}

fn scene_print(output: &mut impl Write, scene: &Scene, shapes: &[Box<Shape>]) {
    write_bytes(output, b"\n=== Scene: ");
    write_bytes(output, &scene.name);
    write_bytes(output, b" ===\n");
    let _ = write!(output, "Contains {} shape(s)\n\n", scene.shapes.len());

    for (index, &shape_type) in scene.shapes.iter().enumerate() {
        let _ = writeln!(output, "Shape #{}:", index + 1);
        shape_print(output, &shapes[shape_type]);
        write_bytes(output, b"\n");
    }
}

fn scene_equals(first: &Scene, second: &Scene) -> bool {
    if first.shapes.len() != second.shapes.len() {
        return false;
    }

    let mut matched = [false; MAX_SHAPES_IN_SCENE];
    for &first_type in &first.shapes {
        let mut found = false;
        for (index, &second_type) in second.shapes.iter().enumerate() {
            if !matched[index] && first_type == second_type {
                matched[index] = true;
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

fn scene_list_shapes(output: &mut impl Write, scene: &Scene, shapes: &[Box<Shape>]) {
    write_bytes(output, b"\nScene: ");
    write_bytes(output, &scene.name);
    let _ = write!(output, "\nShapes ({}):\n", scene.shapes.len());

    for (index, &shape_type) in scene.shapes.iter().enumerate() {
        let shape = &shapes[shape_type];
        let _ = write!(output, "  {}. ", index + 1);
        write_bytes(output, shape.name);
        write_bytes(output, b" (ptr: ");
        print_pointer(output, shape);
        write_bytes(output, b")\n");
    }
}

fn scene_save(
    output: &mut impl Write,
    error: &mut impl Write,
    scene: &Scene,
    filename: &[u8],
) -> Result<(), ()> {
    let path = Path::new(std::ffi::OsStr::from_bytes(filename));
    let mut file = match OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
    {
        Ok(file) => file,
        Err(_) => {
            write_bytes(error, b"Error: Could not open file '");
            write_bytes(error, filename);
            write_bytes(error, b"' for writing\n");
            return Err(());
        }
    };

    let _ = file.write_all(&scene.name);
    let _ = file.write_all(b"\n");
    let _ = writeln!(file, "{}", scene.shapes.len());
    for &shape_type in &scene.shapes {
        let _ = writeln!(file, "{shape_type}");
    }

    write_bytes(output, b"Scene saved to '");
    write_bytes(output, filename);
    write_bytes(output, b"'\n");
    Ok(())
}

fn scene_load(
    output: &mut impl Write,
    error: &mut impl Write,
    shapes: &[Box<Shape>],
    filename: &[u8],
) -> Option<Scene> {
    let path = Path::new(std::ffi::OsStr::from_bytes(filename));
    let file = match File::open(path) {
        Ok(file) => file,
        Err(_) => {
            write_bytes(error, b"Error: Could not open file '");
            write_bytes(error, filename);
            write_bytes(error, b"' for reading\n");
            return None;
        }
    };
    let mut input = Input::new(BufReader::new(file));

    let name = c_string(&input.fgets(64)?);
    let shape_count = input.scanf_i32()?;
    input.consume_whitespace();

    let mut scene = Scene {
        name,
        shapes: Vec::new(),
    };

    let mut index = 0;
    while index < shape_count {
        let shape_type = input.scanf_i32()?;
        input.consume_whitespace();
        if shape_type >= 0 && (shape_type as usize) < shapes.len() {
            let _ = scene_add_shape(&mut scene, shape_type as usize, error);
        }
        index += 1;
    }

    write_bytes(output, b"Scene loaded from '");
    write_bytes(output, filename);
    write_bytes(output, b"'\n");
    Some(scene)
}

fn print_menu(output: &mut impl Write) {
    write_bytes(
        output,
        b"\n=========================================\n  ASCII ART DRAWING APPLICATION\n\
=========================================\n\
1. View all available shapes\n\
2. Create new scene\n\
3. Add shape to scene\n\
4. Remove shape from scene\n\
5. View scene\n\
6. List all scenes\n\
7. Save scene\n\
8. Load scene\n\
9. Compare two shapes\n\
10. Compare two scenes\n\
11. Delete scene\n\
12. Exit\n\
=========================================\n\
Choice: ",
    );
}

fn view_all_shapes(output: &mut impl Write, shapes: &[Box<Shape>]) {
    write_bytes(output, b"\n=== Available Shapes ===\n");
    for (index, shape) in shapes.iter().enumerate() {
        let _ = write!(output, "\n{}. ", index + 1);
        shape_print(output, shape);
    }
}

fn create_new_scene<R: BufRead>(
    input: &mut Input<R>,
    output: &mut impl Write,
    scenes: &mut Vec<Scene>,
) {
    if scenes.len() >= MAX_SCENES {
        write_bytes(output, b"Error: Maximum scenes reached\n");
        return;
    }

    write_bytes(output, b"Enter scene name: ");
    let _ = output.flush();
    let Some(line) = input.fgets(64) else {
        return;
    };
    let name = c_string(&line);
    let index = scenes.len();
    scenes.push(Scene {
        name: name.clone(),
        shapes: Vec::new(),
    });
    write_bytes(output, b"Scene '");
    write_bytes(output, &name);
    let _ = writeln!(output, "' created (index {index})");
}

fn read_scanned_i32<R: BufRead>(input: &mut Input<R>, output: &mut impl Write) -> Option<i32> {
    let _ = output.flush();
    let result = input.scanf_i32();
    if result.is_none() {
        write_bytes(output, b"Invalid input\n");
    }
    input.consume_through_newline();
    result
}

fn add_shape_to_scene<R: BufRead>(
    input: &mut Input<R>,
    output: &mut impl Write,
    error: &mut impl Write,
    scenes: &mut [Scene],
    shapes: &[Box<Shape>],
) {
    if scenes.is_empty() {
        write_bytes(output, b"No scenes available. Create a scene first.\n");
        return;
    }

    let _ = write!(output, "Select scene (0-{}): ", scenes.len() - 1);
    let Some(scene_index) = read_scanned_i32(input, output) else {
        return;
    };
    if scene_index < 0 || scene_index as usize >= scenes.len() {
        write_bytes(output, b"Invalid scene index\n");
        return;
    }

    write_bytes(output, b"\nSelect shape to add:\n");
    for (index, shape) in shapes.iter().enumerate() {
        let _ = write!(output, "{index}. ");
        write_bytes(output, shape.name);
        write_bytes(output, b"\n");
    }
    write_bytes(output, b"Choice: ");

    let Some(shape_type) = read_scanned_i32(input, output) else {
        return;
    };
    if shape_type < 0 || shape_type as usize >= SHAPE_COUNT {
        write_bytes(output, b"Invalid shape type\n");
        return;
    }

    let shape_type = shape_type as usize;
    if scene_add_shape(&mut scenes[scene_index as usize], shape_type, error).is_ok() {
        write_bytes(output, b"Shape '");
        write_bytes(output, shapes[shape_type].name);
        write_bytes(output, b"' added to scene (reusing singleton at ");
        print_pointer(output, &shapes[shape_type]);
        write_bytes(output, b")\n");
    } else {
        write_bytes(output, b"Error adding shape\n");
    }
}

fn remove_shape_from_scene<R: BufRead>(
    input: &mut Input<R>,
    output: &mut impl Write,
    scenes: &mut [Scene],
    shapes: &[Box<Shape>],
) {
    if scenes.is_empty() {
        write_bytes(output, b"No scenes available\n");
        return;
    }

    let _ = write!(output, "Select scene (0-{}): ", scenes.len() - 1);
    let Some(scene_index) = read_scanned_i32(input, output) else {
        return;
    };
    if scene_index < 0 || scene_index as usize >= scenes.len() {
        write_bytes(output, b"Invalid scene index\n");
        return;
    }
    let scene_index = scene_index as usize;

    scene_list_shapes(output, &scenes[scene_index], shapes);
    if scenes[scene_index].shapes.is_empty() {
        write_bytes(output, b"Scene is empty\n");
        return;
    }

    let _ = write!(
        output,
        "Select shape to remove (1-{}): ",
        scenes[scene_index].shapes.len()
    );
    let Some(shape_index) = read_scanned_i32(input, output) else {
        return;
    };

    if scene_remove_shape(&mut scenes[scene_index], shape_index.wrapping_sub(1)).is_ok() {
        write_bytes(output, b"Shape removed\n");
    } else {
        write_bytes(output, b"Error removing shape\n");
    }
}

fn view_scene<R: BufRead>(
    input: &mut Input<R>,
    output: &mut impl Write,
    scenes: &[Scene],
    shapes: &[Box<Shape>],
) {
    if scenes.is_empty() {
        write_bytes(output, b"No scenes available\n");
        return;
    }

    let _ = write!(output, "Select scene (0-{}): ", scenes.len() - 1);
    let Some(scene_index) = read_scanned_i32(input, output) else {
        return;
    };
    if scene_index < 0 || scene_index as usize >= scenes.len() {
        write_bytes(output, b"Invalid scene index\n");
        return;
    }
    scene_print(output, &scenes[scene_index as usize], shapes);
}

fn list_all_scenes(output: &mut impl Write, scenes: &[Scene]) {
    write_bytes(output, b"\n=== All Scenes ===\n");
    if scenes.is_empty() {
        write_bytes(output, b"No scenes created yet\n");
        return;
    }

    for (index, scene) in scenes.iter().enumerate() {
        let _ = write!(output, "{index}. ");
        write_bytes(output, &scene.name);
        let _ = writeln!(output, " ({} shapes)", scene.shapes.len());
    }
}

fn save_scene_to_file<R: BufRead>(
    input: &mut Input<R>,
    output: &mut impl Write,
    error: &mut impl Write,
    scenes: &[Scene],
) {
    if scenes.is_empty() {
        write_bytes(output, b"No scenes available\n");
        return;
    }

    let _ = write!(output, "Select scene (0-{}): ", scenes.len() - 1);
    let Some(scene_index) = read_scanned_i32(input, output) else {
        return;
    };
    if scene_index < 0 || scene_index as usize >= scenes.len() {
        write_bytes(output, b"Invalid scene index\n");
        return;
    }

    write_bytes(output, b"Enter filename: ");
    let _ = output.flush();
    let Some(line) = input.fgets(256) else {
        return;
    };
    let filename = c_string(&line);
    let _ = scene_save(output, error, &scenes[scene_index as usize], &filename);
}

fn load_scene_from_file<R: BufRead>(
    input: &mut Input<R>,
    output: &mut impl Write,
    error: &mut impl Write,
    scenes: &mut Vec<Scene>,
    shapes: &[Box<Shape>],
) {
    if scenes.len() >= MAX_SCENES {
        write_bytes(output, b"Error: Maximum scenes reached\n");
        return;
    }

    write_bytes(output, b"Enter filename: ");
    let _ = output.flush();
    let Some(line) = input.fgets(256) else {
        return;
    };
    let filename = c_string(&line);
    if let Some(scene) = scene_load(output, error, shapes, &filename) {
        let index = scenes.len();
        scenes.push(scene);
        let _ = writeln!(output, "Scene loaded (index {index})");
    }
}

fn compare_shapes<R: BufRead>(
    input: &mut Input<R>,
    output: &mut impl Write,
    shapes: &[Box<Shape>],
) {
    let _ = writeln!(output, "\nSelect first shape (0-{}):", SHAPE_COUNT - 1);
    for (index, shape) in shapes.iter().enumerate() {
        let _ = write!(output, "{index}. ");
        write_bytes(output, shape.name);
        write_bytes(output, b"\n");
    }
    write_bytes(output, b"Choice: ");

    let Some(first_type) = read_scanned_i32(input, output) else {
        return;
    };

    let _ = write!(output, "\nSelect second shape (0-{}): ", SHAPE_COUNT - 1);
    let Some(second_type) = read_scanned_i32(input, output) else {
        return;
    };

    if first_type < 0
        || first_type as usize >= SHAPE_COUNT
        || second_type < 0
        || second_type as usize >= SHAPE_COUNT
    {
        write_bytes(output, b"Invalid shape type\n");
        return;
    }

    let first = &shapes[first_type as usize];
    let second = &shapes[second_type as usize];
    write_bytes(output, b"\nShape 1: ");
    write_bytes(output, first.name);
    write_bytes(output, b" (ptr: ");
    print_pointer(output, first);
    write_bytes(output, b")\nShape 2: ");
    write_bytes(output, second.name);
    write_bytes(output, b" (ptr: ");
    print_pointer(output, second);
    write_bytes(output, b")\n");
    let _ = writeln!(
        output,
        "Comparison of pointers: {}",
        usize::from(first_type == second_type)
    );

    if first_type == second_type {
        write_bytes(output, b"Result: Shapes are EQUAL (same instance)\n");
    } else {
        write_bytes(
            output,
            b"Result: Shapes are NOT EQUAL (different instances)\n",
        );
    }
}

fn compare_scenes<R: BufRead>(
    input: &mut Input<R>,
    output: &mut impl Write,
    scenes: &[Scene],
    shapes: &[Box<Shape>],
) {
    if scenes.len() < 2 {
        write_bytes(output, b"Need at least 2 scenes to compare\n");
        return;
    }

    let _ = write!(output, "Select first scene (0-{}): ", scenes.len() - 1);
    let Some(first_index) = read_scanned_i32(input, output) else {
        return;
    };
    let _ = write!(output, "Select second scene (0-{}): ", scenes.len() - 1);
    let Some(second_index) = read_scanned_i32(input, output) else {
        return;
    };

    if first_index < 0
        || first_index as usize >= scenes.len()
        || second_index < 0
        || second_index as usize >= scenes.len()
    {
        write_bytes(output, b"Invalid scene index\n");
        return;
    }

    let first = &scenes[first_index as usize];
    let second = &scenes[second_index as usize];
    write_bytes(output, b"\nScene 1: ");
    write_bytes(output, &first.name);
    let _ = writeln!(output, " ({} shapes)", first.shapes.len());
    scene_list_shapes(output, first, shapes);

    write_bytes(output, b"\nScene 2: ");
    write_bytes(output, &second.name);
    let _ = writeln!(output, " ({} shapes)", second.shapes.len());
    scene_list_shapes(output, second, shapes);

    if scene_equals(first, second) {
        write_bytes(output, b"\nResult: Scenes are EQUAL (1:1 correspondence)\n");
    } else {
        write_bytes(output, b"\nResult: Scenes are NOT EQUAL\n");
    }
}

fn delete_scene<R: BufRead>(
    input: &mut Input<R>,
    output: &mut impl Write,
    scenes: &mut Vec<Scene>,
) {
    if scenes.is_empty() {
        write_bytes(output, b"No scenes available\n");
        return;
    }

    let _ = write!(output, "Select scene to delete (0-{}): ", scenes.len() - 1);
    let Some(scene_index) = read_scanned_i32(input, output) else {
        return;
    };
    if scene_index < 0 || scene_index as usize >= scenes.len() {
        write_bytes(output, b"Invalid scene index\n");
        return;
    }

    scenes.remove(scene_index as usize);
    write_bytes(output, b"Scene deleted\n");
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let stderr = io::stderr();
    let mut input = Input::new(BufReader::new(stdin.lock()));
    let mut output = BufWriter::new(stdout.lock());
    let mut error = BufWriter::new(stderr.lock());

    write_bytes(
        &mut output,
        "╔════════════════════════════════════════╗\n\
║  ASCII ART DRAWING APPLICATION        ║\n\
║  Child-Friendly Shape Editor           ║\n\
╚════════════════════════════════════════╝\n"
            .as_bytes(),
    );

    let shapes = make_shapes();
    let mut scenes = Vec::new();

    loop {
        print_menu(&mut output);
        let _ = output.flush();
        let Some(line) = input.fgets(256) else {
            break;
        };
        let Some(choice) = sscanf_i32(&line) else {
            write_bytes(&mut output, b"Invalid input\n");
            continue;
        };

        match choice {
            1 => view_all_shapes(&mut output, &shapes),
            2 => create_new_scene(&mut input, &mut output, &mut scenes),
            3 => add_shape_to_scene(&mut input, &mut output, &mut error, &mut scenes, &shapes),
            4 => remove_shape_from_scene(&mut input, &mut output, &mut scenes, &shapes),
            5 => view_scene(&mut input, &mut output, &scenes, &shapes),
            6 => list_all_scenes(&mut output, &scenes),
            7 => save_scene_to_file(&mut input, &mut output, &mut error, &scenes),
            8 => load_scene_from_file(&mut input, &mut output, &mut error, &mut scenes, &shapes),
            9 => compare_shapes(&mut input, &mut output, &shapes),
            10 => compare_scenes(&mut input, &mut output, &scenes, &shapes),
            11 => delete_scene(&mut input, &mut output, &mut scenes),
            12 => {
                write_bytes(&mut output, b"\nCleaning up and exiting...\nGoodbye!\n");
                return;
            }
            _ => write_bytes(&mut output, b"Invalid choice\n"),
        }
    }
}
