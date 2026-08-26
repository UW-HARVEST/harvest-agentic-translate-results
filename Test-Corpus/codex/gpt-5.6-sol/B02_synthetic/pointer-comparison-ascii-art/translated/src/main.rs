mod input;
mod scene;
mod shape;

use input::{c_line_value, sscanf_i32, Input};
use scene::Scene;
use shape::{ShapeManager, SHAPE_COUNT};
use std::io::{self, Read, Write};

const MAX_SCENES: usize = 10;

struct App {
    manager: ShapeManager,
    scenes: Vec<Scene>,
}

impl App {
    fn new() -> Self {
        Self {
            manager: ShapeManager::new(),
            scenes: Vec::new(),
        }
    }

    fn view_all_shapes<W: Write>(&self, out: &mut W) -> io::Result<()> {
        out.write_all(b"\n=== Available Shapes ===\n")?;
        for type_id in 0..SHAPE_COUNT {
            write!(out, "\n{}. ", type_id + 1)?;
            shape::print(out, self.manager.get(type_id))?;
        }
        Ok(())
    }

    fn create_new_scene<W: Write>(&mut self, input: &mut Input, out: &mut W) -> io::Result<()> {
        if self.scenes.len() >= MAX_SCENES {
            out.write_all(b"Error: Maximum scenes reached\n")?;
            return Ok(());
        }

        out.write_all(b"Enter scene name: ")?;
        let Some(line) = input.fgets(64) else {
            return Ok(());
        };
        let name = c_line_value(&line);
        self.scenes.push(Scene::new(&name));
        out.write_all(b"Scene '")?;
        out.write_all(&name)?;
        write!(out, "' created (index {})\n", self.scenes.len() - 1)
    }

    fn add_shape_to_scene<W: Write, E: Write>(
        &mut self,
        input: &mut Input,
        out: &mut W,
        err: &mut E,
    ) -> io::Result<()> {
        if self.scenes.is_empty() {
            out.write_all(b"No scenes available. Create a scene first.\n")?;
            return Ok(());
        }

        write!(out, "Select scene (0-{}): ", self.scenes.len() - 1)?;
        let Some(scene_index) = scan_number(input, out)? else {
            return Ok(());
        };
        let Ok(scene_index) = usize::try_from(scene_index) else {
            out.write_all(b"Invalid scene index\n")?;
            return Ok(());
        };
        if scene_index >= self.scenes.len() {
            out.write_all(b"Invalid scene index\n")?;
            return Ok(());
        }

        out.write_all(b"\nSelect shape to add:\n")?;
        for type_id in 0..SHAPE_COUNT {
            write!(out, "{}. ", type_id)?;
            out.write_all(shape::type_name(type_id))?;
            out.write_all(b"\n")?;
        }
        out.write_all(b"Choice: ")?;

        let Some(type_id) = scan_number(input, out)? else {
            return Ok(());
        };
        if !(0..SHAPE_COUNT).contains(&type_id) {
            out.write_all(b"Invalid shape type\n")?;
            return Ok(());
        }

        let result = self.scenes[scene_index].add_shape(type_id, err);
        if result == 0 {
            out.write_all(b"Shape '")?;
            out.write_all(self.manager.get(type_id).unwrap().name)?;
            write!(
                out,
                "' added to scene (reusing singleton at {:p})\n",
                self.manager.ptr(type_id)
            )?;
        } else {
            out.write_all(b"Error adding shape\n")?;
        }
        Ok(())
    }

    fn remove_shape_from_scene<W: Write>(
        &mut self,
        input: &mut Input,
        out: &mut W,
    ) -> io::Result<()> {
        if self.scenes.is_empty() {
            out.write_all(b"No scenes available\n")?;
            return Ok(());
        }

        write!(out, "Select scene (0-{}): ", self.scenes.len() - 1)?;
        let Some(scene_index) = scan_number(input, out)? else {
            return Ok(());
        };
        let Ok(scene_index) = usize::try_from(scene_index) else {
            out.write_all(b"Invalid scene index\n")?;
            return Ok(());
        };
        if scene_index >= self.scenes.len() {
            out.write_all(b"Invalid scene index\n")?;
            return Ok(());
        }

        scene::list_shapes(out, &self.scenes[scene_index], &self.manager)?;
        if self.scenes[scene_index].shapes.is_empty() {
            out.write_all(b"Scene is empty\n")?;
            return Ok(());
        }

        write!(
            out,
            "Select shape to remove (1-{}): ",
            self.scenes[scene_index].shapes.len()
        )?;
        let Some(shape_index) = scan_number(input, out)? else {
            return Ok(());
        };
        if self.scenes[scene_index].remove_shape(shape_index.wrapping_sub(1)) == 0 {
            out.write_all(b"Shape removed\n")?;
        } else {
            out.write_all(b"Error removing shape\n")?;
        }
        Ok(())
    }

    fn view_scene<W: Write>(&self, input: &mut Input, out: &mut W) -> io::Result<()> {
        if self.scenes.is_empty() {
            out.write_all(b"No scenes available\n")?;
            return Ok(());
        }

        write!(out, "Select scene (0-{}): ", self.scenes.len() - 1)?;
        let Some(scene_index) = scan_number(input, out)? else {
            return Ok(());
        };
        let Ok(scene_index) = usize::try_from(scene_index) else {
            out.write_all(b"Invalid scene index\n")?;
            return Ok(());
        };
        if scene_index >= self.scenes.len() {
            out.write_all(b"Invalid scene index\n")?;
            return Ok(());
        }
        scene::print(out, &self.scenes[scene_index], &self.manager)
    }

    fn list_all_scenes<W: Write>(&self, out: &mut W) -> io::Result<()> {
        out.write_all(b"\n=== All Scenes ===\n")?;
        if self.scenes.is_empty() {
            out.write_all(b"No scenes created yet\n")?;
            return Ok(());
        }

        for (index, scene) in self.scenes.iter().enumerate() {
            write!(out, "{}. ", index)?;
            out.write_all(&scene.name)?;
            writeln!(out, " ({} shapes)", scene.shapes.len())?;
        }
        Ok(())
    }

    fn save_scene_to_file<W: Write, E: Write>(
        &self,
        input: &mut Input,
        out: &mut W,
        err: &mut E,
    ) -> io::Result<()> {
        if self.scenes.is_empty() {
            out.write_all(b"No scenes available\n")?;
            return Ok(());
        }

        write!(out, "Select scene (0-{}): ", self.scenes.len() - 1)?;
        let Some(scene_index) = scan_number(input, out)? else {
            return Ok(());
        };
        let Ok(scene_index) = usize::try_from(scene_index) else {
            out.write_all(b"Invalid scene index\n")?;
            return Ok(());
        };
        if scene_index >= self.scenes.len() {
            out.write_all(b"Invalid scene index\n")?;
            return Ok(());
        }

        out.write_all(b"Enter filename: ")?;
        let Some(line) = input.fgets(256) else {
            return Ok(());
        };
        let filename = c_line_value(&line);
        scene::save(
            &self.scenes[scene_index],
            &filename,
            &self.manager,
            out,
            err,
        );
        Ok(())
    }

    fn load_scene_from_file<W: Write, E: Write>(
        &mut self,
        input: &mut Input,
        out: &mut W,
        err: &mut E,
    ) -> io::Result<()> {
        if self.scenes.len() >= MAX_SCENES {
            out.write_all(b"Error: Maximum scenes reached\n")?;
            return Ok(());
        }

        out.write_all(b"Enter filename: ")?;
        let Some(line) = input.fgets(256) else {
            return Ok(());
        };
        let filename = c_line_value(&line);
        if let Some(loaded) = scene::load(&filename, &self.manager, out, err) {
            self.scenes.push(loaded);
            writeln!(out, "Scene loaded (index {})", self.scenes.len() - 1)?;
        }
        Ok(())
    }

    fn compare_shapes<W: Write>(&self, input: &mut Input, out: &mut W) -> io::Result<()> {
        write!(out, "\nSelect first shape (0-{}):\n", SHAPE_COUNT - 1)?;
        for type_id in 0..SHAPE_COUNT {
            write!(out, "{}. ", type_id)?;
            out.write_all(shape::type_name(type_id))?;
            out.write_all(b"\n")?;
        }
        out.write_all(b"Choice: ")?;
        let Some(first) = scan_number(input, out)? else {
            return Ok(());
        };

        write!(out, "\nSelect second shape (0-{}): ", SHAPE_COUNT - 1)?;
        let Some(second) = scan_number(input, out)? else {
            return Ok(());
        };

        if !(0..SHAPE_COUNT).contains(&first) || !(0..SHAPE_COUNT).contains(&second) {
            out.write_all(b"Invalid shape type\n")?;
            return Ok(());
        }

        out.write_all(b"\nShape 1: ")?;
        out.write_all(self.manager.get(first).unwrap().name)?;
        writeln!(out, " (ptr: {:p})", self.manager.ptr(first))?;
        out.write_all(b"Shape 2: ")?;
        out.write_all(self.manager.get(second).unwrap().name)?;
        writeln!(out, " (ptr: {:p})", self.manager.ptr(second))?;
        writeln!(
            out,
            "Comparison of pointers: {}",
            i32::from(first == second)
        )?;
        if first == second {
            out.write_all(b"Result: Shapes are EQUAL (same instance)\n")?;
        } else {
            out.write_all(b"Result: Shapes are NOT EQUAL (different instances)\n")?;
        }
        Ok(())
    }

    fn compare_scenes<W: Write>(&self, input: &mut Input, out: &mut W) -> io::Result<()> {
        if self.scenes.len() < 2 {
            out.write_all(b"Need at least 2 scenes to compare\n")?;
            return Ok(());
        }

        write!(out, "Select first scene (0-{}): ", self.scenes.len() - 1)?;
        let Some(first) = scan_number(input, out)? else {
            return Ok(());
        };
        write!(out, "Select second scene (0-{}): ", self.scenes.len() - 1)?;
        let Some(second) = scan_number(input, out)? else {
            return Ok(());
        };

        let (Ok(first), Ok(second)) = (usize::try_from(first), usize::try_from(second)) else {
            out.write_all(b"Invalid scene index\n")?;
            return Ok(());
        };
        if first >= self.scenes.len() || second >= self.scenes.len() {
            out.write_all(b"Invalid scene index\n")?;
            return Ok(());
        }

        let first_scene = &self.scenes[first];
        let second_scene = &self.scenes[second];
        out.write_all(b"\nScene 1: ")?;
        out.write_all(&first_scene.name)?;
        writeln!(out, " ({} shapes)", first_scene.shapes.len())?;
        scene::list_shapes(out, first_scene, &self.manager)?;

        out.write_all(b"\nScene 2: ")?;
        out.write_all(&second_scene.name)?;
        writeln!(out, " ({} shapes)", second_scene.shapes.len())?;
        scene::list_shapes(out, second_scene, &self.manager)?;

        if scene::equals(first_scene, second_scene) {
            out.write_all(b"\nResult: Scenes are EQUAL (1:1 correspondence)\n")?;
        } else {
            out.write_all(b"\nResult: Scenes are NOT EQUAL\n")?;
        }
        Ok(())
    }

    fn delete_scene<W: Write>(&mut self, input: &mut Input, out: &mut W) -> io::Result<()> {
        if self.scenes.is_empty() {
            out.write_all(b"No scenes available\n")?;
            return Ok(());
        }

        write!(
            out,
            "Select scene to delete (0-{}): ",
            self.scenes.len() - 1
        )?;
        let Some(scene_index) = scan_number(input, out)? else {
            return Ok(());
        };
        let Ok(scene_index) = usize::try_from(scene_index) else {
            out.write_all(b"Invalid scene index\n")?;
            return Ok(());
        };
        if scene_index >= self.scenes.len() {
            out.write_all(b"Invalid scene index\n")?;
            return Ok(());
        }

        self.scenes.remove(scene_index);
        out.write_all(b"Scene deleted\n")
    }
}

fn scan_number<W: Write>(input: &mut Input, out: &mut W) -> io::Result<Option<i32>> {
    let value = input.scanf_i32();
    if value.is_none() {
        out.write_all(b"Invalid input\n")?;
    }
    input.discard_through_newline();
    Ok(value)
}

fn print_menu<W: Write>(out: &mut W) -> io::Result<()> {
    out.write_all(
        b"\n=========================================\n  ASCII ART DRAWING APPLICATION\n=========================================\n1. View all available shapes\n2. Create new scene\n3. Add shape to scene\n4. Remove shape from scene\n5. View scene\n6. List all scenes\n7. Save scene\n8. Load scene\n9. Compare two shapes\n10. Compare two scenes\n11. Delete scene\n12. Exit\n=========================================\nChoice: ",
    )
}

fn run() -> io::Result<()> {
    let mut stdin_data = Vec::new();
    io::stdin().read_to_end(&mut stdin_data)?;
    let mut input = Input::new(stdin_data);
    let stdout = io::stdout();
    let stderr = io::stderr();
    let mut out = stdout.lock();
    let mut err = stderr.lock();

    out.write_all(
        "╔════════════════════════════════════════╗\n\
         ║  ASCII ART DRAWING APPLICATION        ║\n\
         ║  Child-Friendly Shape Editor           ║\n\
         ╚════════════════════════════════════════╝\n"
            .as_bytes(),
    )?;

    let mut app = App::new();
    loop {
        print_menu(&mut out)?;
        let Some(line) = input.fgets(256) else {
            break;
        };
        let Some(choice) = sscanf_i32(&line) else {
            out.write_all(b"Invalid input\n")?;
            continue;
        };

        match choice {
            1 => app.view_all_shapes(&mut out)?,
            2 => app.create_new_scene(&mut input, &mut out)?,
            3 => app.add_shape_to_scene(&mut input, &mut out, &mut err)?,
            4 => app.remove_shape_from_scene(&mut input, &mut out)?,
            5 => app.view_scene(&mut input, &mut out)?,
            6 => app.list_all_scenes(&mut out)?,
            7 => app.save_scene_to_file(&mut input, &mut out, &mut err)?,
            8 => app.load_scene_from_file(&mut input, &mut out, &mut err)?,
            9 => app.compare_shapes(&mut input, &mut out)?,
            10 => app.compare_scenes(&mut input, &mut out)?,
            11 => app.delete_scene(&mut input, &mut out)?,
            12 => {
                out.write_all(b"\nCleaning up and exiting...\nGoodbye!\n")?;
                return Ok(());
            }
            _ => out.write_all(b"Invalid choice\n")?,
        }
    }
    Ok(())
}

fn main() {
    let _ = run();
}
