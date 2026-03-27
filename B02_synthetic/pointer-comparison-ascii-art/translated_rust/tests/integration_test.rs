use libloading::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::io::Read;
use std::os::unix::io::FromRawFd;

#[repr(C)]
struct CShape {
    shape_type: c_int,
    name: [c_char; 32],
    art: [[c_char; 80]; 30],
    width: c_int,
    height: c_int,
}

#[repr(C)]
struct CScene {
    name: [c_char; 64],
    shapes: [*mut CShape; 50],
    shape_count: c_int,
}

fn c_lib_path() -> String {
    format!("{}/c_src/build/libdriver.so", env!("CARGO_MANIFEST_DIR"))
}

extern "C" { fn fflush(stream: *mut c_void) -> c_int; }

/// Capture C function stdout output via fd-level redirect.
/// Only works for C functions (printf), NOT for Rust print!.
fn capture_c_stdout<F: FnOnce()>(f: F) -> String {
    use std::io::Write;
    std::io::stdout().flush().ok();
    unsafe { fflush(std::ptr::null_mut()); }

    let mut pipe_fds = [0i32; 2];
    unsafe { libc::pipe(pipe_fds.as_mut_ptr()); }
    let (read_fd, write_fd) = (pipe_fds[0], pipe_fds[1]);
    let orig = unsafe { libc::dup(1) };
    unsafe { libc::dup2(write_fd, 1); libc::close(write_fd); }

    f();

    unsafe { fflush(std::ptr::null_mut()); }
    unsafe { libc::dup2(orig, 1); libc::close(orig); }

    unsafe {
        libc::fcntl(read_fd, libc::F_SETFL,
            libc::fcntl(read_fd, libc::F_GETFL) | libc::O_NONBLOCK);
    }
    let mut buf = String::new();
    let mut f = unsafe { std::fs::File::from_raw_fd(read_fd) };
    let _ = f.read_to_string(&mut buf);
    buf
}

/// Build the expected Rust shape_print output from the Rust data structures.
fn rust_shape_print_str(shape: *const ascii_art::shape::Shape) -> String {
    if shape.is_null() {
        return "(null shape)\n".to_string();
    }
    let s = unsafe { &*shape };
    let mut out = format!("{}:\n", s.name);
    for i in 0..s.height as usize {
        out.push_str(&s.art[i]);
        out.push('\n');
    }
    out
}

/// Build the expected Rust scene_print output.
fn rust_scene_print_str(scene: *const ascii_art::scene::Scene) -> String {
    if scene.is_null() {
        return "(null scene)\n".to_string();
    }
    let s = unsafe { &*scene };
    let mut out = format!("\n=== Scene: {} ===\n", s.name);
    out.push_str(&format!("Contains {} shape(s)\n\n", s.shape_count));
    for i in 0..s.shape_count as usize {
        out.push_str(&format!("Shape #{}:\n", i + 1));
        out.push_str(&rust_shape_print_str(s.shapes[i]));
        out.push('\n');
    }
    out
}

/// Build the expected Rust scene_list_shapes output (with ptr placeholder).
fn rust_scene_list_str(scene: *const ascii_art::scene::Scene) -> String {
    if scene.is_null() {
        return "(null scene)\n".to_string();
    }
    let s = unsafe { &*scene };
    let mut out = format!("\nScene: {}\n", s.name);
    out.push_str(&format!("Shapes ({}):\n", s.shape_count));
    for i in 0..s.shape_count as usize {
        let shape = unsafe { &*s.shapes[i] };
        out.push_str(&format!("  {}. {} (ptr: {:p})\n", i + 1, shape.name, s.shapes[i]));
    }
    out
}

/// Strip pointer values from scene_list output for comparison.
fn strip_ptrs(s: &str) -> String {
    s.lines().map(|l| {
        if let Some(i) = l.find("(ptr: ") {
            format!("{}(ptr: X)", &l[..i])
        } else { l.to_string() }
    }).collect::<Vec<_>>().join("\n")
}

// ============================================================
#[test]
fn test_shape_type_name() {
    unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> *const c_char> =
            lib.get(b"shape_type_name").unwrap();
        for i in 0..10 {
            let c = CStr::from_ptr(c_fn(i)).to_str().unwrap();
            let r = ascii_art::shape::shape_type_name(i as usize);
            assert_eq!(c, r, "shape_type_name({}) mismatch", i);
        }
        let c = CStr::from_ptr(c_fn(99)).to_str().unwrap();
        assert_eq!(c, ascii_art::shape::shape_type_name(99));
    }
}

#[test]
fn test_shape_equals() {
    unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        let c_init: Symbol<unsafe extern "C" fn()> = lib.get(b"shape_manager_init").unwrap();
        let c_cleanup: Symbol<unsafe extern "C" fn()> = lib.get(b"shape_manager_cleanup").unwrap();
        let c_get: Symbol<unsafe extern "C" fn(c_int) -> *mut CShape> = lib.get(b"shape_get").unwrap();
        let c_eq: Symbol<unsafe extern "C" fn(*const CShape, *const CShape) -> c_int> =
            lib.get(b"shape_equals").unwrap();

        c_init();
        let s0 = c_get(0); let s1 = c_get(1);
        assert_eq!(c_eq(s0, s0), 1);
        assert_eq!(c_eq(s0, s1), 0);

        ascii_art::shape::shape_manager_init();
        let r0 = ascii_art::shape::shape_get(0);
        let r1 = ascii_art::shape::shape_get(1);
        // C returns int (1/0), Rust returns bool
        assert_eq!(ascii_art::shape::shape_equals(r0, r0), true);
        assert_eq!(ascii_art::shape::shape_equals(r0, r1), false);

        ascii_art::shape::shape_manager_cleanup();
        c_cleanup();
    }
}

#[test]
fn test_shape_print() {
    unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        let c_init: Symbol<unsafe extern "C" fn()> = lib.get(b"shape_manager_init").unwrap();
        let c_cleanup: Symbol<unsafe extern "C" fn()> = lib.get(b"shape_manager_cleanup").unwrap();
        let c_get: Symbol<unsafe extern "C" fn(c_int) -> *mut CShape> = lib.get(b"shape_get").unwrap();
        let c_print: Symbol<unsafe extern "C" fn(*const CShape)> = lib.get(b"shape_print").unwrap();

        c_init();
        ascii_art::shape::shape_manager_init();

        for i in 0..10 {
            let c_out = capture_c_stdout(|| { c_print(c_get(i)); });
            let rs_out = rust_shape_print_str(ascii_art::shape::shape_get(i as usize));
            assert_eq!(c_out.as_bytes(), rs_out.as_bytes(),
                "shape_print({}) mismatch.\nC:    {:?}\nRust: {:?}", i, c_out, rs_out);
        }

        // Null
        let c_null = capture_c_stdout(|| { c_print(std::ptr::null()); });
        let rs_null = rust_shape_print_str(std::ptr::null());
        assert_eq!(c_null.as_bytes(), rs_null.as_bytes(),
            "shape_print(null) mismatch.\nC: {:?}\nRust: {:?}", c_null, rs_null);

        ascii_art::shape::shape_manager_cleanup();
        c_cleanup();
    }
}

#[test]
fn test_scene_create_and_print() {
    unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        let c_create: Symbol<unsafe extern "C" fn(*const c_char) -> *mut CScene> =
            lib.get(b"scene_create").unwrap();
        let c_destroy: Symbol<unsafe extern "C" fn(*mut CScene)> =
            lib.get(b"scene_destroy").unwrap();
        let c_print: Symbol<unsafe extern "C" fn(*const CScene)> =
            lib.get(b"scene_print").unwrap();

        let name = CString::new("Test Scene").unwrap();
        let cs = c_create(name.as_ptr());
        let c_out = capture_c_stdout(|| { c_print(cs); });

        let rs = ascii_art::scene::scene_create("Test Scene");
        let rs_out = rust_scene_print_str(rs);

        assert_eq!(c_out.as_bytes(), rs_out.as_bytes(),
            "scene_print empty mismatch.\nC:    {:?}\nRust: {:?}", c_out, rs_out);

        // Null
        let c_null = capture_c_stdout(|| { c_print(std::ptr::null()); });
        let rs_null = rust_scene_print_str(std::ptr::null());
        assert_eq!(c_null.as_bytes(), rs_null.as_bytes());

        c_destroy(cs);
        ascii_art::scene::scene_destroy(rs);
    }
}

#[test]
fn test_scene_with_shapes() {
    unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        let c_init: Symbol<unsafe extern "C" fn()> = lib.get(b"shape_manager_init").unwrap();
        let c_cleanup: Symbol<unsafe extern "C" fn()> = lib.get(b"shape_manager_cleanup").unwrap();
        let c_get: Symbol<unsafe extern "C" fn(c_int) -> *mut CShape> = lib.get(b"shape_get").unwrap();
        let c_create: Symbol<unsafe extern "C" fn(*const c_char) -> *mut CScene> =
            lib.get(b"scene_create").unwrap();
        let c_destroy: Symbol<unsafe extern "C" fn(*mut CScene)> =
            lib.get(b"scene_destroy").unwrap();
        let c_add: Symbol<unsafe extern "C" fn(*mut CScene, *mut CShape) -> c_int> =
            lib.get(b"scene_add_shape").unwrap();
        let c_print: Symbol<unsafe extern "C" fn(*const CScene)> =
            lib.get(b"scene_print").unwrap();

        c_init();
        ascii_art::shape::shape_manager_init();

        let name = CString::new("My Scene").unwrap();
        let cs = c_create(name.as_ptr());
        c_add(cs, c_get(0));
        c_add(cs, c_get(2));

        let rs = ascii_art::scene::scene_create("My Scene");
        ascii_art::scene::scene_add_shape(rs, ascii_art::shape::shape_get(0));
        ascii_art::scene::scene_add_shape(rs, ascii_art::shape::shape_get(2));

        let c_out = capture_c_stdout(|| { c_print(cs); });
        let rs_out = rust_scene_print_str(rs);

        assert_eq!(c_out.as_bytes(), rs_out.as_bytes(),
            "scene_print with shapes mismatch.\nC:    {:?}\nRust: {:?}", c_out, rs_out);

        c_destroy(cs);
        ascii_art::scene::scene_destroy(rs);
        ascii_art::shape::shape_manager_cleanup();
        c_cleanup();
    }
}

#[test]
fn test_scene_remove_shape() {
    unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        let c_init: Symbol<unsafe extern "C" fn()> = lib.get(b"shape_manager_init").unwrap();
        let c_cleanup: Symbol<unsafe extern "C" fn()> = lib.get(b"shape_manager_cleanup").unwrap();
        let c_get: Symbol<unsafe extern "C" fn(c_int) -> *mut CShape> = lib.get(b"shape_get").unwrap();
        let c_create: Symbol<unsafe extern "C" fn(*const c_char) -> *mut CScene> =
            lib.get(b"scene_create").unwrap();
        let c_destroy: Symbol<unsafe extern "C" fn(*mut CScene)> =
            lib.get(b"scene_destroy").unwrap();
        let c_add: Symbol<unsafe extern "C" fn(*mut CScene, *mut CShape) -> c_int> =
            lib.get(b"scene_add_shape").unwrap();
        let c_remove: Symbol<unsafe extern "C" fn(*mut CScene, c_int) -> c_int> =
            lib.get(b"scene_remove_shape").unwrap();
        let c_print: Symbol<unsafe extern "C" fn(*const CScene)> =
            lib.get(b"scene_print").unwrap();

        c_init();
        ascii_art::shape::shape_manager_init();

        let name = CString::new("Remove Test").unwrap();
        let cs = c_create(name.as_ptr());
        let rs = ascii_art::scene::scene_create("Remove Test");

        for t in &[0i32, 1, 2] {
            c_add(cs, c_get(*t));
            ascii_art::scene::scene_add_shape(rs, ascii_art::shape::shape_get(*t as usize));
        }

        let c_ret = c_remove(cs, 1);
        let rs_ret = ascii_art::scene::scene_remove_shape(rs, 1);
        assert_eq!(c_ret, rs_ret, "remove return mismatch");

        let c_out = capture_c_stdout(|| { c_print(cs); });
        let rs_out = rust_scene_print_str(rs);
        assert_eq!(c_out.as_bytes(), rs_out.as_bytes(),
            "scene after remove mismatch.\nC:    {:?}\nRust: {:?}", c_out, rs_out);

        assert_eq!(c_remove(cs, 99), ascii_art::scene::scene_remove_shape(rs, 99));

        c_destroy(cs);
        ascii_art::scene::scene_destroy(rs);
        ascii_art::shape::shape_manager_cleanup();
        c_cleanup();
    }
}

#[test]
fn test_scene_equals() {
    unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        let c_init: Symbol<unsafe extern "C" fn()> = lib.get(b"shape_manager_init").unwrap();
        let c_cleanup: Symbol<unsafe extern "C" fn()> = lib.get(b"shape_manager_cleanup").unwrap();
        let c_get: Symbol<unsafe extern "C" fn(c_int) -> *mut CShape> = lib.get(b"shape_get").unwrap();
        let c_create: Symbol<unsafe extern "C" fn(*const c_char) -> *mut CScene> =
            lib.get(b"scene_create").unwrap();
        let c_destroy: Symbol<unsafe extern "C" fn(*mut CScene)> =
            lib.get(b"scene_destroy").unwrap();
        let c_add: Symbol<unsafe extern "C" fn(*mut CScene, *mut CShape) -> c_int> =
            lib.get(b"scene_add_shape").unwrap();
        let c_eq: Symbol<unsafe extern "C" fn(*const CScene, *const CScene) -> c_int> =
            lib.get(b"scene_equals").unwrap();

        c_init();
        ascii_art::shape::shape_manager_init();

        let n1 = CString::new("S1").unwrap();
        let n2 = CString::new("S2").unwrap();
        let cs1 = c_create(n1.as_ptr());
        let cs2 = c_create(n2.as_ptr());
        c_add(cs1, c_get(0)); c_add(cs1, c_get(1));
        c_add(cs2, c_get(0)); c_add(cs2, c_get(1));
        assert_eq!(c_eq(cs1, cs2), 1);

        let rs1 = ascii_art::scene::scene_create("S1");
        let rs2 = ascii_art::scene::scene_create("S2");
        ascii_art::scene::scene_add_shape(rs1, ascii_art::shape::shape_get(0));
        ascii_art::scene::scene_add_shape(rs1, ascii_art::shape::shape_get(1));
        ascii_art::scene::scene_add_shape(rs2, ascii_art::shape::shape_get(0));
        ascii_art::scene::scene_add_shape(rs2, ascii_art::shape::shape_get(1));
        assert_eq!(ascii_art::scene::scene_equals(rs1, rs2), true);

        // Different
        let cs3 = c_create(n1.as_ptr());
        c_add(cs3, c_get(5));
        assert_eq!(c_eq(cs1, cs3), 0);
        let rs3 = ascii_art::scene::scene_create("S1");
        ascii_art::scene::scene_add_shape(rs3, ascii_art::shape::shape_get(5));
        assert_eq!(ascii_art::scene::scene_equals(rs1, rs3), false);

        c_destroy(cs1); c_destroy(cs2); c_destroy(cs3);
        ascii_art::scene::scene_destroy(rs1);
        ascii_art::scene::scene_destroy(rs2);
        ascii_art::scene::scene_destroy(rs3);
        ascii_art::shape::shape_manager_cleanup();
        c_cleanup();
    }
}

#[test]
fn test_scene_save_load() {
    unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        let c_init: Symbol<unsafe extern "C" fn()> = lib.get(b"shape_manager_init").unwrap();
        let c_cleanup: Symbol<unsafe extern "C" fn()> = lib.get(b"shape_manager_cleanup").unwrap();
        let c_get: Symbol<unsafe extern "C" fn(c_int) -> *mut CShape> = lib.get(b"shape_get").unwrap();
        let c_create: Symbol<unsafe extern "C" fn(*const c_char) -> *mut CScene> =
            lib.get(b"scene_create").unwrap();
        let c_destroy: Symbol<unsafe extern "C" fn(*mut CScene)> =
            lib.get(b"scene_destroy").unwrap();
        let c_add: Symbol<unsafe extern "C" fn(*mut CScene, *mut CShape) -> c_int> =
            lib.get(b"scene_add_shape").unwrap();
        let c_save: Symbol<unsafe extern "C" fn(*const CScene, *const c_char) -> c_int> =
            lib.get(b"scene_save").unwrap();
        let c_load: Symbol<unsafe extern "C" fn(*const c_char) -> *mut CScene> =
            lib.get(b"scene_load").unwrap();
        let c_print: Symbol<unsafe extern "C" fn(*const CScene)> =
            lib.get(b"scene_print").unwrap();

        c_init();
        ascii_art::shape::shape_manager_init();

        // Save from C
        let name = CString::new("Save Test").unwrap();
        let cs = c_create(name.as_ptr());
        c_add(cs, c_get(0)); c_add(cs, c_get(3));
        let cf = CString::new("/tmp/test_c_save.txt").unwrap();
        let _ = capture_c_stdout(|| { c_save(cs, cf.as_ptr()); });

        // Save from Rust
        let rs = ascii_art::scene::scene_create("Save Test");
        ascii_art::scene::scene_add_shape(rs, ascii_art::shape::shape_get(0));
        ascii_art::scene::scene_add_shape(rs, ascii_art::shape::shape_get(3));
        ascii_art::scene::scene_save(rs, "/tmp/test_rs_save.txt");

        // Compare file contents
        let c_file = std::fs::read("/tmp/test_c_save.txt").unwrap();
        let rs_file = std::fs::read("/tmp/test_rs_save.txt").unwrap();
        assert_eq!(c_file, rs_file,
            "scene_save file mismatch.\nC:  {:?}\nRust: {:?}",
            String::from_utf8_lossy(&c_file), String::from_utf8_lossy(&rs_file));

        // Load from C-saved file in Rust, and from Rust-saved file in C
        let loaded_rs = ascii_art::scene::scene_load("/tmp/test_c_save.txt");
        assert!(!loaded_rs.is_null());
        let rs_loaded_out = rust_scene_print_str(loaded_rs);

        let rf = CString::new("/tmp/test_rs_save.txt").unwrap();
        let loaded_cs = capture_c_stdout(|| { c_load(rf.as_ptr()); });
        // Load returns a scene, let's just verify the file round-trips
        let loaded_c = c_load(cf.as_ptr());
        let c_loaded_out = capture_c_stdout(|| { c_print(loaded_c); });

        assert_eq!(c_loaded_out.as_bytes(), rs_loaded_out.as_bytes(),
            "scene load round-trip mismatch.\nC:    {:?}\nRust: {:?}", c_loaded_out, rs_loaded_out);

        c_destroy(cs); c_destroy(loaded_c);
        ascii_art::scene::scene_destroy(rs);
        ascii_art::scene::scene_destroy(loaded_rs);
        let _ = std::fs::remove_file("/tmp/test_c_save.txt");
        let _ = std::fs::remove_file("/tmp/test_rs_save.txt");
        ascii_art::shape::shape_manager_cleanup();
        c_cleanup();
    }
}

#[test]
fn test_scene_list_shapes_format() {
    unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        let c_init: Symbol<unsafe extern "C" fn()> = lib.get(b"shape_manager_init").unwrap();
        let c_cleanup: Symbol<unsafe extern "C" fn()> = lib.get(b"shape_manager_cleanup").unwrap();
        let c_get: Symbol<unsafe extern "C" fn(c_int) -> *mut CShape> = lib.get(b"shape_get").unwrap();
        let c_create: Symbol<unsafe extern "C" fn(*const c_char) -> *mut CScene> =
            lib.get(b"scene_create").unwrap();
        let c_destroy: Symbol<unsafe extern "C" fn(*mut CScene)> =
            lib.get(b"scene_destroy").unwrap();
        let c_add: Symbol<unsafe extern "C" fn(*mut CScene, *mut CShape) -> c_int> =
            lib.get(b"scene_add_shape").unwrap();
        let c_list: Symbol<unsafe extern "C" fn(*const CScene)> =
            lib.get(b"scene_list_shapes").unwrap();

        c_init();
        ascii_art::shape::shape_manager_init();

        let name = CString::new("List Test").unwrap();
        let cs = c_create(name.as_ptr());
        let rs = ascii_art::scene::scene_create("List Test");

        c_add(cs, c_get(0));
        ascii_art::scene::scene_add_shape(rs, ascii_art::shape::shape_get(0));

        let c_out = capture_c_stdout(|| { c_list(cs); });
        let rs_out = rust_scene_list_str(rs);

        assert_eq!(strip_ptrs(&c_out), strip_ptrs(&rs_out),
            "scene_list_shapes format mismatch.\nC:    {:?}\nRust: {:?}", c_out, rs_out);

        // Null
        let c_null = capture_c_stdout(|| { c_list(std::ptr::null()); });
        let rs_null = rust_scene_list_str(std::ptr::null());
        assert_eq!(c_null.as_bytes(), rs_null.as_bytes());

        c_destroy(cs);
        ascii_art::scene::scene_destroy(rs);
        ascii_art::shape::shape_manager_cleanup();
        c_cleanup();
    }
}
