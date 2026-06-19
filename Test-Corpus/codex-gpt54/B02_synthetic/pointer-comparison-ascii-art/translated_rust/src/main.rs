mod input;
mod scene;
mod shape;

use input::{stdin_reader, ByteReader, StreamReader};
use scene::{strip_first_newline, Scene, MAX_SCENE_NAME};
use shape::{shape_equals, shape_print, shape_type_name, Shape, ShapeManager, SHAPE_COUNT};

const MAX_SCENES: usize = 10;

struct App {
    scenes: Vec<Scene>,
    shapes: ShapeManager,
}

impl App {
    fn new() -> Self {
        Self {
            scenes: Vec::new(),
            shapes: ShapeManager::new(),
        }
    }

    fn print_menu(&self) {
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

    fn view_all_shapes(&self) {
        println!("\n=== Available Shapes ===");
        for index in 0..SHAPE_COUNT {
            print!("\n{}. ", index + 1);
            shape_print(self.shapes.get_by_i32(index as i32));
        }
    }

    fn create_new_scene<R: std::io::Read>(&mut self, reader: &mut StreamReader<R>) {
        if self.scenes.len() >= MAX_SCENES {
            println!("Error: Maximum scenes reached");
            return;
        }

        print!("Enter scene name: ");
        flush_stdout();
        let Some(mut name) = reader.fgets(MAX_SCENE_NAME) else {
            return;
        };
        strip_first_newline(&mut name);

        let scene = Scene::create(Some(&name));
        println!("Scene '{}' created (index {})", name, self.scenes.len());
        self.scenes.push(scene);
    }

    fn add_shape_to_scene<R: std::io::Read>(&mut self, reader: &mut StreamReader<R>) {
        if self.scenes.is_empty() {
            println!("No scenes available. Create a scene first.");
            return;
        }

        print!("Select scene (0-{}): ", self.scenes.len() - 1);
        flush_stdout();
        let Some(scene_idx) = scan_menu_int(reader) else {
            return;
        };

        if scene_idx < 0 || scene_idx as usize >= self.scenes.len() {
            println!("Invalid scene index");
            return;
        }

        println!("\nSelect shape to add:");
        for index in 0..SHAPE_COUNT {
            println!("{}. {}", index, shape_type_name(index as i32));
        }
        print!("Choice: ");
        flush_stdout();

        let Some(shape_type) = scan_menu_int(reader) else {
            return;
        };

        if shape_type < 0 || shape_type as usize >= SHAPE_COUNT {
            println!("Invalid shape type");
            return;
        }

        let shape = self.shapes.get_by_i32(shape_type).unwrap();
        if self.scenes[scene_idx as usize].add_shape(Some(shape)) == 0 {
            println!(
                "Shape '{}' added to scene (reusing singleton at {:p})",
                shape.name,
                shape as *const Shape
            );
        } else {
            println!("Error adding shape");
        }
    }

    fn remove_shape_from_scene<R: std::io::Read>(&mut self, reader: &mut StreamReader<R>) {
        if self.scenes.is_empty() {
            println!("No scenes available");
            return;
        }

        print!("Select scene (0-{}): ", self.scenes.len() - 1);
        flush_stdout();
        let Some(scene_idx) = scan_menu_int(reader) else {
            return;
        };

        if scene_idx < 0 || scene_idx as usize >= self.scenes.len() {
            println!("Invalid scene index");
            return;
        }

        self.scenes[scene_idx as usize].list_shapes(&self.shapes);

        if self.scenes[scene_idx as usize].shapes.is_empty() {
            println!("Scene is empty");
            return;
        }

        print!(
            "Select shape to remove (1-{}): ",
            self.scenes[scene_idx as usize].shapes.len()
        );
        flush_stdout();
        let Some(shape_idx) = scan_menu_int(reader) else {
            return;
        };

        if self.scenes[scene_idx as usize].remove_shape(shape_idx - 1) == 0 {
            println!("Shape removed");
        } else {
            println!("Error removing shape");
        }
    }

    fn view_scene<R: std::io::Read>(&self, reader: &mut StreamReader<R>) {
        if self.scenes.is_empty() {
            println!("No scenes available");
            return;
        }

        print!("Select scene (0-{}): ", self.scenes.len() - 1);
        flush_stdout();
        let Some(scene_idx) = scan_menu_int(reader) else {
            return;
        };

        if scene_idx < 0 || scene_idx as usize >= self.scenes.len() {
            println!("Invalid scene index");
            return;
        }

        self.scenes[scene_idx as usize].print(&self.shapes);
    }

    fn list_all_scenes(&self) {
        println!("\n=== All Scenes ===");
        if self.scenes.is_empty() {
            println!("No scenes created yet");
            return;
        }

        for (index, scene) in self.scenes.iter().enumerate() {
            println!("{}. {} ({} shapes)", index, scene.name, scene.shapes.len());
        }
    }

    fn save_scene_to_file<R: std::io::Read>(&self, reader: &mut StreamReader<R>) {
        if self.scenes.is_empty() {
            println!("No scenes available");
            return;
        }

        print!("Select scene (0-{}): ", self.scenes.len() - 1);
        flush_stdout();
        let Some(scene_idx) = scan_menu_int(reader) else {
            return;
        };

        if scene_idx < 0 || scene_idx as usize >= self.scenes.len() {
            println!("Invalid scene index");
            return;
        }

        print!("Enter filename: ");
        flush_stdout();
        let Some(mut filename) = reader.fgets(256) else {
            return;
        };
        strip_first_newline(&mut filename);

        let _ = self.scenes[scene_idx as usize].save(&filename);
    }

    fn load_scene_from_file<R: std::io::Read>(&mut self, reader: &mut StreamReader<R>) {
        if self.scenes.len() >= MAX_SCENES {
            println!("Error: Maximum scenes reached");
            return;
        }

        print!("Enter filename: ");
        flush_stdout();
        let Some(mut filename) = reader.fgets(256) else {
            return;
        };
        strip_first_newline(&mut filename);

        if let Some(scene) = Scene::load(&filename, &self.shapes) {
            self.scenes.push(scene);
            println!("Scene loaded (index {})", self.scenes.len() - 1);
        }
    }

    fn compare_shapes<R: std::io::Read>(&self, reader: &mut StreamReader<R>) {
        println!("\nSelect first shape (0-{}):", SHAPE_COUNT - 1);
        for index in 0..SHAPE_COUNT {
            println!("{}. {}", index, shape_type_name(index as i32));
        }
        print!("Choice: ");
        flush_stdout();

        let Some(type1) = scan_menu_int(reader) else {
            return;
        };

        print!("\nSelect second shape (0-{}): ", SHAPE_COUNT - 1);
        flush_stdout();
        let Some(type2) = scan_menu_int(reader) else {
            return;
        };

        if type1 < 0 || type1 as usize >= SHAPE_COUNT || type2 < 0 || type2 as usize >= SHAPE_COUNT
        {
            println!("Invalid shape type");
            return;
        }

        let s1 = self.shapes.get_by_i32(type1).unwrap();
        let s2 = self.shapes.get_by_i32(type2).unwrap();

        println!("\nShape 1: {} (ptr: {:p})", s1.name, s1 as *const Shape);
        println!("Shape 2: {} (ptr: {:p})", s2.name, s2 as *const Shape);
        println!("Comparison of pointers: {}", std::ptr::eq(s1, s2) as i32);

        if shape_equals(s1, s2) {
            println!("Result: Shapes are EQUAL (same instance)");
        } else {
            println!("Result: Shapes are NOT EQUAL (different instances)");
        }
    }

    fn compare_scenes<R: std::io::Read>(&self, reader: &mut StreamReader<R>) {
        if self.scenes.len() < 2 {
            println!("Need at least 2 scenes to compare");
            return;
        }

        print!("Select first scene (0-{}): ", self.scenes.len() - 1);
        flush_stdout();
        let Some(idx1) = scan_menu_int(reader) else {
            return;
        };

        print!("Select second scene (0-{}): ", self.scenes.len() - 1);
        flush_stdout();
        let Some(idx2) = scan_menu_int(reader) else {
            return;
        };

        if idx1 < 0 || idx1 as usize >= self.scenes.len() || idx2 < 0 || idx2 as usize >= self.scenes.len() {
            println!("Invalid scene index");
            return;
        }

        let sc1 = &self.scenes[idx1 as usize];
        let sc2 = &self.scenes[idx2 as usize];

        println!("\nScene 1: {} ({} shapes)", sc1.name, sc1.shapes.len());
        sc1.list_shapes(&self.shapes);

        println!("\nScene 2: {} ({} shapes)", sc2.name, sc2.shapes.len());
        sc2.list_shapes(&self.shapes);

        if sc1.equals(sc2, &self.shapes) {
            println!("\nResult: Scenes are EQUAL (1:1 correspondence)");
        } else {
            println!("\nResult: Scenes are NOT EQUAL");
        }
    }

    fn delete_scene<R: std::io::Read>(&mut self, reader: &mut StreamReader<R>) {
        if self.scenes.is_empty() {
            println!("No scenes available");
            return;
        }

        print!("Select scene to delete (0-{}): ", self.scenes.len() - 1);
        flush_stdout();
        let Some(scene_idx) = scan_menu_int(reader) else {
            return;
        };

        if scene_idx < 0 || scene_idx as usize >= self.scenes.len() {
            println!("Invalid scene index");
            return;
        }

        self.scenes.remove(scene_idx as usize);
        println!("Scene deleted");
    }
}

fn main() {
    println!("╔════════════════════════════════════════╗");
    println!("║  ASCII ART DRAWING APPLICATION        ║");
    println!("║  Child-Friendly Shape Editor           ║");
    println!("╚════════════════════════════════════════╝");

    let mut app = App::new();
    let mut reader = stdin_reader();

    loop {
        app.print_menu();

        let Some(input) = reader.fgets(256) else {
            break;
        };

        let Some(choice) = sscanf_d(&input) else {
            println!("Invalid input");
            continue;
        };

        match choice {
            1 => app.view_all_shapes(),
            2 => app.create_new_scene(&mut reader),
            3 => app.add_shape_to_scene(&mut reader),
            4 => app.remove_shape_from_scene(&mut reader),
            5 => app.view_scene(&mut reader),
            6 => app.list_all_scenes(),
            7 => app.save_scene_to_file(&mut reader),
            8 => app.load_scene_from_file(&mut reader),
            9 => app.compare_shapes(&mut reader),
            10 => app.compare_scenes(&mut reader),
            11 => app.delete_scene(&mut reader),
            12 => {
                println!("\nCleaning up and exiting...");
                println!("Goodbye!");
                return;
            }
            _ => println!("Invalid choice"),
        }
    }
}

fn scan_menu_int<R: std::io::Read>(reader: &mut StreamReader<R>) -> Option<i32> {
    let value = match reader.scanf_d() {
        Some(value) => value,
        None => {
            println!("Invalid input");
            reader.flush_until_newline();
            return None;
        }
    };
    reader.flush_until_newline();
    Some(value)
}

fn sscanf_d(input: &str) -> Option<i32> {
    let trimmed = input.trim_start_matches(char::is_whitespace);
    let mut chars = trimmed.chars();
    let mut buf = String::new();

    if let Some(ch @ ('+' | '-')) = chars.next() {
        buf.push(ch);
    } else {
        chars = trimmed.chars();
    }

    for ch in chars {
        if ch.is_ascii_digit() {
            buf.push(ch);
        } else {
            break;
        }
    }

    if buf.is_empty() || buf == "+" || buf == "-" {
        None
    } else {
        buf.parse::<i32>().ok()
    }
}

fn flush_stdout() {
    use std::io::Write;

    let _ = std::io::stdout().flush();
}
