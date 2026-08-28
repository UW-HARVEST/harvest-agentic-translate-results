//! Translation of main.c
//!
//! Behaviour (including the quirks of the original) is preserved exactly:
//! prompts, the order of the validation checks, the mix of `fgets` and
//! `scanf`/`getchar` reads on a single shared stdin stream, and the pointer
//! values printed with `%p`.

mod cio;
mod scene;
mod shape;

use cio::{CStdin, COut};
use scene::{
    scene_add_shape, scene_create, scene_equals, scene_list_shapes, scene_load, scene_print,
    scene_remove_shape, scene_save, Scene, MAX_SCENE_NAME,
};
use shape::{
    shape_get, shape_manager_cleanup, shape_manager_init, shape_print, shape_ptr, shape_same,
    shape_type_name, SHAPE_COUNT,
};

const MAX_SCENES: usize = 10;

struct App {
    out: COut,
    input: CStdin,
    scenes: Vec<Scene>,
}

impl App {
    fn new() -> App {
        App {
            out: COut::new(),
            input: CStdin::new(),
            scenes: Vec::new(),
        }
    }

    fn scene_count(&self) -> i32 {
        self.scenes.len() as i32
    }

    fn print_menu(&mut self) {
        self.out.puts("\n");
        self.out.puts("=========================================\n");
        self.out.puts("  ASCII ART DRAWING APPLICATION\n");
        self.out.puts("=========================================\n");
        self.out.puts("1. View all available shapes\n");
        self.out.puts("2. Create new scene\n");
        self.out.puts("3. Add shape to scene\n");
        self.out.puts("4. Remove shape from scene\n");
        self.out.puts("5. View scene\n");
        self.out.puts("6. List all scenes\n");
        self.out.puts("7. Save scene\n");
        self.out.puts("8. Load scene\n");
        self.out.puts("9. Compare two shapes\n");
        self.out.puts("10. Compare two scenes\n");
        self.out.puts("11. Delete scene\n");
        self.out.puts("12. Exit\n");
        self.out.puts("=========================================\n");
        self.out.puts("Choice: ");
    }

    fn view_all_shapes(&mut self) {
        self.out.puts("\n=== Available Shapes ===\n");
        for i in 0..SHAPE_COUNT {
            self.out.puts(&format!("\n{}. ", i + 1));
            shape_print(&mut self.out, shape_get(i));
        }
    }

    fn create_new_scene(&mut self) {
        if self.scenes.len() >= MAX_SCENES {
            self.out.puts("Error: Maximum scenes reached\n");
            return;
        }

        self.out.puts("Enter scene name: ");
        let line = match self.input.fgets(MAX_SCENE_NAME) {
            None => return,
            Some(l) => l,
        };
        let name = cio::strip_newline(&line);

        // scene_create() only fails when malloc fails, which cannot happen here.
        let scene = scene_create(Some(&name));
        self.out.puts("Scene '");
        self.out.put(&name);
        self.out
            .puts(&format!("' created (index {})\n", self.scenes.len()));
        self.scenes.push(scene);
    }

    /// `if (scanf("%d", &x) != 1) { printf("Invalid input\n"); while (getchar()
    /// != '\n'); return; } while (getchar() != '\n');`
    fn read_int_or_bail(&mut self) -> Option<i32> {
        match self.input.scanf_int() {
            None => {
                self.out.puts("Invalid input\n");
                self.input.discard_line();
                None
            }
            Some(v) => {
                self.input.discard_line();
                Some(v)
            }
        }
    }

    fn add_shape_to_scene(&mut self) {
        if self.scenes.is_empty() {
            self.out.puts("No scenes available. Create a scene first.\n");
            return;
        }

        self.out
            .puts(&format!("Select scene (0-{}): ", self.scene_count() - 1));
        let scene_idx = match self.read_int_or_bail() {
            None => return,
            Some(v) => v,
        };

        if scene_idx < 0 || scene_idx >= self.scene_count() {
            self.out.puts("Invalid scene index\n");
            return;
        }

        self.out.puts("\nSelect shape to add:\n");
        for i in 0..SHAPE_COUNT {
            self.out
                .puts(&format!("{}. {}\n", i, shape_type_name(i)));
        }
        self.out.puts("Choice: ");

        let shape_type = match self.read_int_or_bail() {
            None => return,
            Some(v) => v,
        };

        if shape_type < 0 || shape_type >= SHAPE_COUNT {
            self.out.puts("Invalid shape type\n");
            return;
        }

        let shape = shape_get(shape_type);
        if scene_add_shape(&mut self.scenes[scene_idx as usize], shape) == 0 {
            let shape = shape.unwrap();
            self.out.puts("Shape '");
            self.out.put(&shape.name);
            self.out.puts(&format!(
                "' added to scene (reusing singleton at {})\n",
                shape_ptr(shape)
            ));
        } else {
            self.out.puts("Error adding shape\n");
        }
    }

    fn remove_shape_from_scene(&mut self) {
        if self.scenes.is_empty() {
            self.out.puts("No scenes available\n");
            return;
        }

        self.out
            .puts(&format!("Select scene (0-{}): ", self.scene_count() - 1));
        let scene_idx = match self.read_int_or_bail() {
            None => return,
            Some(v) => v,
        };

        if scene_idx < 0 || scene_idx >= self.scene_count() {
            self.out.puts("Invalid scene index\n");
            return;
        }

        let idx = scene_idx as usize;
        scene_list_shapes(&mut self.out, &self.scenes[idx]);

        if self.scenes[idx].shapes.is_empty() {
            self.out.puts("Scene is empty\n");
            return;
        }

        self.out.puts(&format!(
            "Select shape to remove (1-{}): ",
            self.scenes[idx].shape_count()
        ));
        let shape_idx = match self.read_int_or_bail() {
            None => return,
            Some(v) => v,
        };

        if scene_remove_shape(&mut self.scenes[idx], shape_idx.wrapping_sub(1)) == 0 {
            self.out.puts("Shape removed\n");
        } else {
            self.out.puts("Error removing shape\n");
        }
    }

    fn view_scene(&mut self) {
        if self.scenes.is_empty() {
            self.out.puts("No scenes available\n");
            return;
        }

        self.out
            .puts(&format!("Select scene (0-{}): ", self.scene_count() - 1));
        let scene_idx = match self.read_int_or_bail() {
            None => return,
            Some(v) => v,
        };

        if scene_idx < 0 || scene_idx >= self.scene_count() {
            self.out.puts("Invalid scene index\n");
            return;
        }

        scene_print(&mut self.out, &self.scenes[scene_idx as usize]);
    }

    fn list_all_scenes(&mut self) {
        self.out.puts("\n=== All Scenes ===\n");
        if self.scenes.is_empty() {
            self.out.puts("No scenes created yet\n");
            return;
        }

        for i in 0..self.scenes.len() {
            self.out.puts(&format!("{}. ", i));
            self.out.put(&self.scenes[i].name);
            self.out
                .puts(&format!(" ({} shapes)\n", self.scenes[i].shape_count()));
        }
    }

    fn save_scene_to_file(&mut self) {
        if self.scenes.is_empty() {
            self.out.puts("No scenes available\n");
            return;
        }

        self.out
            .puts(&format!("Select scene (0-{}): ", self.scene_count() - 1));
        let scene_idx = match self.read_int_or_bail() {
            None => return,
            Some(v) => v,
        };

        if scene_idx < 0 || scene_idx >= self.scene_count() {
            self.out.puts("Invalid scene index\n");
            return;
        }

        self.out.puts("Enter filename: ");
        let line = match self.input.fgets(256) {
            None => return,
            Some(l) => l,
        };
        let filename = cio::strip_newline(&line);

        scene_save(
            &mut self.out,
            &self.scenes[scene_idx as usize],
            &filename,
        );
    }

    fn load_scene_from_file(&mut self) {
        if self.scenes.len() >= MAX_SCENES {
            self.out.puts("Error: Maximum scenes reached\n");
            return;
        }

        self.out.puts("Enter filename: ");
        let line = match self.input.fgets(256) {
            None => return,
            Some(l) => l,
        };
        let filename = cio::strip_newline(&line);

        if let Some(scene) = scene_load(&mut self.out, &filename) {
            self.scenes.push(scene);
            self.out
                .puts(&format!("Scene loaded (index {})\n", self.scenes.len() - 1));
        }
    }

    fn compare_shapes(&mut self) {
        self.out
            .puts(&format!("\nSelect first shape (0-{}):\n", SHAPE_COUNT - 1));
        for i in 0..SHAPE_COUNT {
            self.out
                .puts(&format!("{}. {}\n", i, shape_type_name(i)));
        }
        self.out.puts("Choice: ");

        let type1 = match self.read_int_or_bail() {
            None => return,
            Some(v) => v,
        };

        self.out.puts(&format!(
            "\nSelect second shape (0-{}): ",
            SHAPE_COUNT - 1
        ));
        let type2 = match self.read_int_or_bail() {
            None => return,
            Some(v) => v,
        };

        if type1 < 0 || type1 >= SHAPE_COUNT || type2 < 0 || type2 >= SHAPE_COUNT {
            self.out.puts("Invalid shape type\n");
            return;
        }

        let s1 = shape_get(type1);
        let s2 = shape_get(type2);

        let s1r = s1.unwrap();
        let s2r = s2.unwrap();

        self.out.puts("\nShape 1: ");
        self.out.put(&s1r.name);
        self.out
            .puts(&format!(" (ptr: {})\n", shape_ptr(s1r)));
        self.out.puts("Shape 2: ");
        self.out.put(&s2r.name);
        self.out
            .puts(&format!(" (ptr: {})\n", shape_ptr(s2r)));
        self.out.puts(&format!(
            "Comparison of pointers: {}\n",
            if shape_same(s1, s2) { 1 } else { 0 }
        ));

        if shape_same(s1, s2) {
            self.out.puts("Result: Shapes are EQUAL (same instance)\n");
        } else {
            self.out
                .puts("Result: Shapes are NOT EQUAL (different instances)\n");
        }
    }

    fn compare_scenes(&mut self) {
        if self.scene_count() < 2 {
            self.out.puts("Need at least 2 scenes to compare\n");
            return;
        }

        self.out
            .puts(&format!("Select first scene (0-{}): ", self.scene_count() - 1));
        let idx1 = match self.read_int_or_bail() {
            None => return,
            Some(v) => v,
        };

        self.out.puts(&format!(
            "Select second scene (0-{}): ",
            self.scene_count() - 1
        ));
        let idx2 = match self.read_int_or_bail() {
            None => return,
            Some(v) => v,
        };

        if idx1 < 0 || idx1 >= self.scene_count() || idx2 < 0 || idx2 >= self.scene_count() {
            self.out.puts("Invalid scene index\n");
            return;
        }

        let i1 = idx1 as usize;
        let i2 = idx2 as usize;

        self.out.puts("\nScene 1: ");
        self.out.put(&self.scenes[i1].name);
        self.out
            .puts(&format!(" ({} shapes)\n", self.scenes[i1].shape_count()));
        scene_list_shapes(&mut self.out, &self.scenes[i1]);

        self.out.puts("\nScene 2: ");
        self.out.put(&self.scenes[i2].name);
        self.out
            .puts(&format!(" ({} shapes)\n", self.scenes[i2].shape_count()));
        scene_list_shapes(&mut self.out, &self.scenes[i2]);

        if scene_equals(&self.scenes[i1], &self.scenes[i2]) != 0 {
            self.out
                .puts("\nResult: Scenes are EQUAL (1:1 correspondence)\n");
        } else {
            self.out.puts("\nResult: Scenes are NOT EQUAL\n");
        }
    }

    fn delete_scene(&mut self) {
        if self.scenes.is_empty() {
            self.out.puts("No scenes available\n");
            return;
        }

        self.out.puts(&format!(
            "Select scene to delete (0-{}): ",
            self.scene_count() - 1
        ));
        let scene_idx = match self.read_int_or_bail() {
            None => return,
            Some(v) => v,
        };

        if scene_idx < 0 || scene_idx >= self.scene_count() {
            self.out.puts("Invalid scene index\n");
            return;
        }

        // scene_destroy() + shifting the remaining scenes down
        self.scenes.remove(scene_idx as usize);
        self.out.puts("Scene deleted\n");
    }
}

fn main() {
    let mut app = App::new();

    app.out
        .puts("╔════════════════════════════════════════╗\n");
    app.out
        .puts("║  ASCII ART DRAWING APPLICATION        ║\n");
    app.out
        .puts("║  Child-Friendly Shape Editor           ║\n");
    app.out
        .puts("╚════════════════════════════════════════╝\n");

    // Initialize shape manager (allocate all shapes once)
    shape_manager_init();

    loop {
        app.print_menu();

        let input = match app.input.fgets(256) {
            None => break,
            Some(l) => l,
        };

        let choice = match cio::sscanf_int(cio::c_str(&input)) {
            None => {
                app.out.puts("Invalid input\n");
                continue;
            }
            Some(v) => v,
        };

        match choice {
            1 => app.view_all_shapes(),
            2 => app.create_new_scene(),
            3 => app.add_shape_to_scene(),
            4 => app.remove_shape_from_scene(),
            5 => app.view_scene(),
            6 => app.list_all_scenes(),
            7 => app.save_scene_to_file(),
            8 => app.load_scene_from_file(),
            9 => app.compare_shapes(),
            10 => app.compare_scenes(),
            11 => app.delete_scene(),
            12 => {
                app.out.puts("\nCleaning up and exiting...\n");
                app.scenes.clear();
                shape_manager_cleanup();
                app.out.puts("Goodbye!\n");
                app.out.flush();
                return;
            }
            _ => app.out.puts("Invalid choice\n"),
        }
    }

    // Cleanup
    app.scenes.clear();
    shape_manager_cleanup();
    app.out.flush();
}
