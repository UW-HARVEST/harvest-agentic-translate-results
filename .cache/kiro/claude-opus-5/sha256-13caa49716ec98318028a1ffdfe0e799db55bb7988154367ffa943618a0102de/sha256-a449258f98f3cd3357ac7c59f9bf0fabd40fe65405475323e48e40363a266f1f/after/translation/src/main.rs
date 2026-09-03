// Translation of main.c
//
// The program is a faithful port of the original C: the same prompts in the
// same order, the same validation order, the same stdin consumption pattern
// (fgets for line-oriented reads, scanf plus a "drain to newline" loop for
// numeric reads) and the same stdout/stderr buffering behaviour.

mod cio;
mod scene;
mod shape;

use cio::{In, Out, Scan};
use scene::{
    scene_add_shape, scene_equals, scene_list_shapes, scene_load, scene_print, scene_remove_shape,
    scene_save, Scene, MAX_SCENE_NAME,
};
use shape::{shape_print, shape_type_name, ShapeManager, SHAPE_COUNT};

const MAX_SCENES: i32 = 10;

/// The Rust runtime sets SIGPIPE to SIG_IGN before `main` runs, so a write to a
/// closed pipe returns EPIPE and the process keeps going. A C program keeps the
/// default disposition and is killed by the signal instead. Restore SIG_DFL so
/// that the exit status matches the C program's when stdout or stderr goes away.
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

/// Turns a NUL-terminated C byte string into a path without going through UTF-8.
pub fn os_path(bytes: &[u8]) -> std::path::PathBuf {
    use std::os::unix::ffi::OsStrExt;
    std::path::PathBuf::from(std::ffi::OsStr::from_bytes(cio::c_str_bytes(bytes)))
}

struct App {
    out: Out,
    inp: In<std::io::BufReader<std::io::Stdin>>,
    mgr: ShapeManager,
    scenes: Vec<Box<Scene>>,
}

impl App {
    fn scene_count(&self) -> i32 {
        self.scenes.len() as i32
    }

    fn print_menu(&mut self) {
        let o = &mut self.out;
        o.put("\n");
        o.put("=========================================\n");
        o.put("  ASCII ART DRAWING APPLICATION\n");
        o.put("=========================================\n");
        o.put("1. View all available shapes\n");
        o.put("2. Create new scene\n");
        o.put("3. Add shape to scene\n");
        o.put("4. Remove shape from scene\n");
        o.put("5. View scene\n");
        o.put("6. List all scenes\n");
        o.put("7. Save scene\n");
        o.put("8. Load scene\n");
        o.put("9. Compare two shapes\n");
        o.put("10. Compare two scenes\n");
        o.put("11. Delete scene\n");
        o.put("12. Exit\n");
        o.put("=========================================\n");
        o.put("Choice: ");
    }

    fn view_all_shapes(&mut self) {
        self.out.put("\n=== Available Shapes ===\n");
        for i in 0..SHAPE_COUNT {
            self.out.put(&format!("\n{}. ", i + 1));
            shape_print(&mut self.out, self.mgr.get(i));
        }
    }

    fn create_new_scene(&mut self) {
        if self.scene_count() >= MAX_SCENES {
            self.out.put("Error: Maximum scenes reached\n");
            return;
        }

        self.out.put("Enter scene name: ");
        let raw = match self.inp.fgets(MAX_SCENE_NAME) {
            None => return,
            Some(b) => b,
        };
        let name = cio::trim_at_newline(&raw);

        let scene = Scene::create(Some(&name));
        let index = self.scene_count();
        self.scenes.push(scene);
        self.out.put("Scene '");
        self.out.put_bytes(&name);
        self.out.put(&format!("' created (index {})\n", index));
    }

    /// The `scanf("%d", &x); while (getchar() != '\n');` idiom. Returns None
    /// when the conversion failed (after emitting "Invalid input" and draining
    /// the line, exactly as the C code does).
    fn read_int_or_complain(&mut self) -> Option<i32> {
        match self.inp.scan_int() {
            Scan::Fail => {
                self.out.put("Invalid input\n");
                self.inp.eat_until_newline();
                None
            }
            Scan::Val(v) => {
                self.inp.eat_until_newline();
                Some(v)
            }
        }
    }

    fn add_shape_to_scene(&mut self) {
        if self.scene_count() == 0 {
            self.out
                .put("No scenes available. Create a scene first.\n");
            return;
        }

        self.out
            .put(&format!("Select scene (0-{}): ", self.scene_count() - 1));
        let scene_idx = match self.read_int_or_complain() {
            None => return,
            Some(v) => v,
        };

        if scene_idx < 0 || scene_idx >= self.scene_count() {
            self.out.put("Invalid scene index\n");
            return;
        }

        self.out.put("\nSelect shape to add:\n");
        for i in 0..SHAPE_COUNT {
            self.out.put(&format!("{}. {}\n", i, shape_type_name(i)));
        }
        self.out.put("Choice: ");

        let shape_type = match self.read_int_or_complain() {
            None => return,
            Some(v) => v,
        };

        if shape_type < 0 || shape_type >= SHAPE_COUNT {
            self.out.put("Invalid shape type\n");
            return;
        }

        let ptr = self.mgr.ptr_of(shape_type);
        let name: Vec<u8> = match self.mgr.get(shape_type) {
            Some(s) => cio::c_str_bytes(&s.name).to_vec(),
            None => Vec::new(),
        };

        if scene_add_shape(&mut self.scenes[scene_idx as usize], Some(shape_type)) == 0 {
            self.out.put("Shape '");
            self.out.put_bytes(&name);
            self.out.put(&format!(
                "' added to scene (reusing singleton at {})\n",
                cio::fmt_ptr(ptr)
            ));
        } else {
            self.out.put("Error adding shape\n");
        }
    }

    fn remove_shape_from_scene(&mut self) {
        if self.scene_count() == 0 {
            self.out.put("No scenes available\n");
            return;
        }

        self.out
            .put(&format!("Select scene (0-{}): ", self.scene_count() - 1));
        let scene_idx = match self.read_int_or_complain() {
            None => return,
            Some(v) => v,
        };

        if scene_idx < 0 || scene_idx >= self.scene_count() {
            self.out.put("Invalid scene index\n");
            return;
        }

        let idx = scene_idx as usize;
        scene_list_shapes(&mut self.out, &self.mgr, &self.scenes[idx]);

        if self.scenes[idx].shape_count == 0 {
            self.out.put("Scene is empty\n");
            return;
        }

        self.out.put(&format!(
            "Select shape to remove (1-{}): ",
            self.scenes[idx].shape_count
        ));
        let shape_idx = match self.read_int_or_complain() {
            None => return,
            Some(v) => v,
        };

        // The C code evaluates `shape_idx - 1`, which wraps for INT_MIN on the
        // platforms this targets. `wrapping_sub` reproduces that and keeps the
        // debug and release builds behaving identically.
        if scene_remove_shape(&mut self.scenes[idx], shape_idx.wrapping_sub(1)) == 0 {
            self.out.put("Shape removed\n");
        } else {
            self.out.put("Error removing shape\n");
        }
    }

    fn view_scene(&mut self) {
        if self.scene_count() == 0 {
            self.out.put("No scenes available\n");
            return;
        }

        self.out
            .put(&format!("Select scene (0-{}): ", self.scene_count() - 1));
        let scene_idx = match self.read_int_or_complain() {
            None => return,
            Some(v) => v,
        };

        if scene_idx < 0 || scene_idx >= self.scene_count() {
            self.out.put("Invalid scene index\n");
            return;
        }

        scene_print(&mut self.out, &self.mgr, &self.scenes[scene_idx as usize]);
    }

    fn list_all_scenes(&mut self) {
        self.out.put("\n=== All Scenes ===\n");
        if self.scene_count() == 0 {
            self.out.put("No scenes created yet\n");
            return;
        }

        for i in 0..self.scenes.len() {
            self.out.put(&format!("{}. ", i));
            let name = self.scenes[i].name_str().to_vec();
            self.out.put_bytes(&name);
            self.out
                .put(&format!(" ({} shapes)\n", self.scenes[i].shape_count));
        }
    }

    fn save_scene_to_file(&mut self) {
        if self.scene_count() == 0 {
            self.out.put("No scenes available\n");
            return;
        }

        self.out
            .put(&format!("Select scene (0-{}): ", self.scene_count() - 1));
        let scene_idx = match self.read_int_or_complain() {
            None => return,
            Some(v) => v,
        };

        if scene_idx < 0 || scene_idx >= self.scene_count() {
            self.out.put("Invalid scene index\n");
            return;
        }

        self.out.put("Enter filename: ");
        let raw = match self.inp.fgets(256) {
            None => return,
            Some(b) => b,
        };
        let filename = cio::trim_at_newline(&raw);

        scene_save(
            &mut self.out,
            &self.scenes[scene_idx as usize],
            &filename,
        );
    }

    fn load_scene_from_file(&mut self) {
        if self.scene_count() >= MAX_SCENES {
            self.out.put("Error: Maximum scenes reached\n");
            return;
        }

        self.out.put("Enter filename: ");
        let raw = match self.inp.fgets(256) {
            None => return,
            Some(b) => b,
        };
        let filename = cio::trim_at_newline(&raw);

        if let Some(scene) = scene_load(&mut self.out, &self.mgr, &filename) {
            self.scenes.push(scene);
            let index = self.scene_count() - 1;
            self.out.put(&format!("Scene loaded (index {})\n", index));
        }
    }

    fn compare_shapes(&mut self) {
        self.out
            .put(&format!("\nSelect first shape (0-{}):\n", SHAPE_COUNT - 1));
        for i in 0..SHAPE_COUNT {
            self.out.put(&format!("{}. {}\n", i, shape_type_name(i)));
        }
        self.out.put("Choice: ");

        let type1 = match self.read_int_or_complain() {
            None => return,
            Some(v) => v,
        };

        self.out
            .put(&format!("\nSelect second shape (0-{}): ", SHAPE_COUNT - 1));
        let type2 = match self.read_int_or_complain() {
            None => return,
            Some(v) => v,
        };

        if type1 < 0 || type1 >= SHAPE_COUNT || type2 < 0 || type2 >= SHAPE_COUNT {
            self.out.put("Invalid shape type\n");
            return;
        }

        let p1 = self.mgr.ptr_of(type1);
        let p2 = self.mgr.ptr_of(type2);
        let n1 = self
            .mgr
            .get(type1)
            .map(|s| cio::c_str_bytes(&s.name).to_vec())
            .unwrap_or_default();
        let n2 = self
            .mgr
            .get(type2)
            .map(|s| cio::c_str_bytes(&s.name).to_vec())
            .unwrap_or_default();

        self.out.put("\nShape 1: ");
        self.out.put_bytes(&n1);
        self.out.put(&format!(" (ptr: {})\n", cio::fmt_ptr(p1)));
        self.out.put("Shape 2: ");
        self.out.put_bytes(&n2);
        self.out.put(&format!(" (ptr: {})\n", cio::fmt_ptr(p2)));
        let same = if p1 == p2 { 1 } else { 0 };
        self.out
            .put(&format!("Comparison of pointers: {}\n", same));

        if same == 1 {
            self.out.put("Result: Shapes are EQUAL (same instance)\n");
        } else {
            self.out
                .put("Result: Shapes are NOT EQUAL (different instances)\n");
        }
    }

    fn compare_scenes(&mut self) {
        if self.scene_count() < 2 {
            self.out.put("Need at least 2 scenes to compare\n");
            return;
        }

        self.out.put(&format!(
            "Select first scene (0-{}): ",
            self.scene_count() - 1
        ));
        let idx1 = match self.read_int_or_complain() {
            None => return,
            Some(v) => v,
        };

        self.out.put(&format!(
            "Select second scene (0-{}): ",
            self.scene_count() - 1
        ));
        let idx2 = match self.read_int_or_complain() {
            None => return,
            Some(v) => v,
        };

        if idx1 < 0 || idx1 >= self.scene_count() || idx2 < 0 || idx2 >= self.scene_count() {
            self.out.put("Invalid scene index\n");
            return;
        }

        let i1 = idx1 as usize;
        let i2 = idx2 as usize;

        let name1 = self.scenes[i1].name_str().to_vec();
        let count1 = self.scenes[i1].shape_count;
        self.out.put("\nScene 1: ");
        self.out.put_bytes(&name1);
        self.out.put(&format!(" ({} shapes)\n", count1));
        scene_list_shapes(&mut self.out, &self.mgr, &self.scenes[i1]);

        let name2 = self.scenes[i2].name_str().to_vec();
        let count2 = self.scenes[i2].shape_count;
        self.out.put("\nScene 2: ");
        self.out.put_bytes(&name2);
        self.out.put(&format!(" ({} shapes)\n", count2));
        scene_list_shapes(&mut self.out, &self.mgr, &self.scenes[i2]);

        if scene_equals(&self.scenes[i1], &self.scenes[i2]) != 0 {
            self.out
                .put("\nResult: Scenes are EQUAL (1:1 correspondence)\n");
        } else {
            self.out.put("\nResult: Scenes are NOT EQUAL\n");
        }
    }

    fn delete_scene(&mut self) {
        if self.scene_count() == 0 {
            self.out.put("No scenes available\n");
            return;
        }

        self.out.put(&format!(
            "Select scene to delete (0-{}): ",
            self.scene_count() - 1
        ));
        let scene_idx = match self.read_int_or_complain() {
            None => return,
            Some(v) => v,
        };

        if scene_idx < 0 || scene_idx >= self.scene_count() {
            self.out.put("Invalid scene index\n");
            return;
        }

        // scene_destroy() plus the shift of the remaining entries.
        self.scenes.remove(scene_idx as usize);
        self.out.put("Scene deleted\n");
    }
}

fn main() {
    restore_default_sigpipe();

    let mut app = App {
        out: Out::new(),
        inp: In::stdin(),
        mgr: ShapeManager::init(),
        scenes: Vec::new(),
    };

    app.out
        .put("╔════════════════════════════════════════╗\n");
    app.out
        .put("║  ASCII ART DRAWING APPLICATION        ║\n");
    app.out
        .put("║  Child-Friendly Shape Editor           ║\n");
    app.out
        .put("╚════════════════════════════════════════╝\n");

    loop {
        app.print_menu();

        let input = match app.inp.fgets(256) {
            None => break,
            Some(b) => b,
        };

        let choice = match cio::sscanf_int(&input) {
            None => {
                app.out.put("Invalid input\n");
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
                app.out.put("\nCleaning up and exiting...\n");
                app.scenes.clear();
                app.mgr.cleanup();
                app.out.put("Goodbye!\n");
                app.out.flush();
                return;
            }
            _ => app.out.put("Invalid choice\n"),
        }
    }

    // Cleanup
    app.scenes.clear();
    app.mgr.cleanup();
    app.out.flush();
}
