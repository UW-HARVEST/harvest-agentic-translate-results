use libloading::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.join("target/debug/libdriver.so")
}

/// Capture stdout from a closure by redirecting fd 1 to a pipe
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    use std::io::Read;
    // flush before
    unsafe { libc::fflush(std::ptr::null_mut()); }

    let mut pipes = [0i32; 2];
    unsafe { libc::pipe(pipes.as_mut_ptr()); }
    let old_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(pipes[1], 1); }

    f();

    unsafe {
        libc::fflush(std::ptr::null_mut());
        // Also flush Rust's stdout
        let _ = std::io::Write::flush(&mut std::io::stdout());
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);
        libc::close(pipes[1]);
    }

    let mut buf = String::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(pipes[0]) };
    reader.read_to_string(&mut buf).unwrap_or(0);
    buf
}

use std::os::unix::io::FromRawFd;

// ============ shape_type_name tests ============

#[test]
fn test_shape_type_name() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> *const c_char> =
            c_lib.get(b"shape_type_name").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int) -> *const c_char> =
            r_lib.get(b"shape_type_name").unwrap();

        // Test all valid types 0..9 and invalid -1, 10, 99
        for t in [-1i32, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 99] {
            let c_ptr = c_fn(t);
            let r_ptr = r_fn(t);
            let c_str = CStr::from_ptr(c_ptr).to_str().unwrap();
            let r_str = CStr::from_ptr(r_ptr).to_str().unwrap();
            assert_eq!(c_str, r_str, "shape_type_name({}) mismatch", t);
        }
    }
}

// ============ shape_manager_init / shape_get / shape_equals tests ============

#[test]
fn test_shape_manager_and_get() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_init: Symbol<unsafe extern "C" fn()> = c_lib.get(b"shape_manager_init").unwrap();
        let r_init: Symbol<unsafe extern "C" fn()> = r_lib.get(b"shape_manager_init").unwrap();
        let c_get: Symbol<unsafe extern "C" fn(c_int) -> *mut c_void> = c_lib.get(b"shape_get").unwrap();
        let r_get: Symbol<unsafe extern "C" fn(c_int) -> *mut c_void> = r_lib.get(b"shape_get").unwrap();
        let c_cleanup: Symbol<unsafe extern "C" fn()> = c_lib.get(b"shape_manager_cleanup").unwrap();
        let r_cleanup: Symbol<unsafe extern "C" fn()> = r_lib.get(b"shape_manager_cleanup").unwrap();

        c_init();
        r_init();

        // Valid types return non-null
        for t in 0..10i32 {
            let c_ptr = c_get(t);
            let r_ptr = r_get(t);
            assert!(!c_ptr.is_null(), "C shape_get({}) returned null", t);
            assert!(!r_ptr.is_null(), "Rust shape_get({}) returned null", t);
        }

        // Invalid types return null
        for t in [-1i32, 10, 99] {
            let c_ptr = c_get(t);
            let r_ptr = r_get(t);
            assert!(c_ptr.is_null(), "C shape_get({}) should be null", t);
            assert!(r_ptr.is_null(), "Rust shape_get({}) should be null", t);
        }

        c_cleanup();
        r_cleanup();
    }
}

#[test]
fn test_shape_equals() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_init: Symbol<unsafe extern "C" fn()> = c_lib.get(b"shape_manager_init").unwrap();
        let r_init: Symbol<unsafe extern "C" fn()> = r_lib.get(b"shape_manager_init").unwrap();
        let c_get: Symbol<unsafe extern "C" fn(c_int) -> *mut c_void> = c_lib.get(b"shape_get").unwrap();
        let r_get: Symbol<unsafe extern "C" fn(c_int) -> *mut c_void> = r_lib.get(b"shape_get").unwrap();
        let c_eq: Symbol<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int> =
            c_lib.get(b"shape_equals").unwrap();
        let r_eq: Symbol<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int> =
            r_lib.get(b"shape_equals").unwrap();
        let c_cleanup: Symbol<unsafe extern "C" fn()> = c_lib.get(b"shape_manager_cleanup").unwrap();
        let r_cleanup: Symbol<unsafe extern "C" fn()> = r_lib.get(b"shape_manager_cleanup").unwrap();

        c_init();
        r_init();

        // Same shape == 1, different shapes == 0
        for i in 0..10i32 {
            for j in 0..10i32 {
                let c_s1 = c_get(i);
                let c_s2 = c_get(j);
                let r_s1 = r_get(i);
                let r_s2 = r_get(j);
                let c_result = c_eq(c_s1, c_s2);
                let r_result = r_eq(r_s1, r_s2);
                assert_eq!(c_result, r_result,
                    "shape_equals({}, {}) mismatch: C={}, Rust={}", i, j, c_result, r_result);
            }
        }

        c_cleanup();
        r_cleanup();
    }
}

// ============ shape_print tests ============

#[test]
fn test_shape_print() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_init: Symbol<unsafe extern "C" fn()> = c_lib.get(b"shape_manager_init").unwrap();
        let r_init: Symbol<unsafe extern "C" fn()> = r_lib.get(b"shape_manager_init").unwrap();
        let c_get: Symbol<unsafe extern "C" fn(c_int) -> *mut c_void> = c_lib.get(b"shape_get").unwrap();
        let r_get: Symbol<unsafe extern "C" fn(c_int) -> *mut c_void> = r_lib.get(b"shape_get").unwrap();
        let c_print: Symbol<unsafe extern "C" fn(*const c_void)> = c_lib.get(b"shape_print").unwrap();
        let r_print: Symbol<unsafe extern "C" fn(*const c_void)> = r_lib.get(b"shape_print").unwrap();
        let c_cleanup: Symbol<unsafe extern "C" fn()> = c_lib.get(b"shape_manager_cleanup").unwrap();
        let r_cleanup: Symbol<unsafe extern "C" fn()> = r_lib.get(b"shape_manager_cleanup").unwrap();

        c_init();
        r_init();

        // Test null shape
        let c_null = capture_stdout(|| c_print(std::ptr::null()));
        let r_null = capture_stdout(|| r_print(std::ptr::null()));
        assert_eq!(c_null, r_null, "shape_print(null) mismatch");

        // Test each shape
        for t in 0..10i32 {
            let c_shape = c_get(t);
            let r_shape = r_get(t);
            let c_out = capture_stdout(|| c_print(c_shape));
            let r_out = capture_stdout(|| r_print(r_shape));
            assert_eq!(c_out, r_out, "shape_print({}) mismatch:\nC:  {:?}\nRust: {:?}", t, c_out, r_out);
        }

        c_cleanup();
        r_cleanup();
    }
}

// ============ scene tests ============

#[test]
fn test_scene_create_destroy() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_create: Symbol<unsafe extern "C" fn(*const c_char) -> *mut c_void> =
            c_lib.get(b"scene_create").unwrap();
        let r_create: Symbol<unsafe extern "C" fn(*const c_char) -> *mut c_void> =
            r_lib.get(b"scene_create").unwrap();
        let c_destroy: Symbol<unsafe extern "C" fn(*mut c_void)> =
            c_lib.get(b"scene_destroy").unwrap();
        let r_destroy: Symbol<unsafe extern "C" fn(*mut c_void)> =
            r_lib.get(b"scene_destroy").unwrap();

        // Create with name
        let name = CString::new("Test Scene").unwrap();
        let c_scene = c_create(name.as_ptr());
        let r_scene = r_create(name.as_ptr());
        assert!(!c_scene.is_null());
        assert!(!r_scene.is_null());
        c_destroy(c_scene);
        r_destroy(r_scene);

        // Create with null name
        let c_scene = c_create(std::ptr::null());
        let r_scene = r_create(std::ptr::null());
        assert!(!c_scene.is_null());
        assert!(!r_scene.is_null());
        c_destroy(c_scene);
        r_destroy(r_scene);

        // Destroy null (should not crash)
        c_destroy(std::ptr::null_mut());
        r_destroy(std::ptr::null_mut());
    }
}

#[test]
fn test_scene_add_remove_shape() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_init: Symbol<unsafe extern "C" fn()> = c_lib.get(b"shape_manager_init").unwrap();
        let r_init: Symbol<unsafe extern "C" fn()> = r_lib.get(b"shape_manager_init").unwrap();
        let c_get: Symbol<unsafe extern "C" fn(c_int) -> *mut c_void> = c_lib.get(b"shape_get").unwrap();
        let r_get: Symbol<unsafe extern "C" fn(c_int) -> *mut c_void> = r_lib.get(b"shape_get").unwrap();
        let c_create: Symbol<unsafe extern "C" fn(*const c_char) -> *mut c_void> =
            c_lib.get(b"scene_create").unwrap();
        let r_create: Symbol<unsafe extern "C" fn(*const c_char) -> *mut c_void> =
            r_lib.get(b"scene_create").unwrap();
        let c_add: Symbol<unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int> =
            c_lib.get(b"scene_add_shape").unwrap();
        let r_add: Symbol<unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int> =
            r_lib.get(b"scene_add_shape").unwrap();
        let c_remove: Symbol<unsafe extern "C" fn(*mut c_void, c_int) -> c_int> =
            c_lib.get(b"scene_remove_shape").unwrap();
        let r_remove: Symbol<unsafe extern "C" fn(*mut c_void, c_int) -> c_int> =
            r_lib.get(b"scene_remove_shape").unwrap();
        let c_destroy: Symbol<unsafe extern "C" fn(*mut c_void)> =
            c_lib.get(b"scene_destroy").unwrap();
        let r_destroy: Symbol<unsafe extern "C" fn(*mut c_void)> =
            r_lib.get(b"scene_destroy").unwrap();
        let c_cleanup: Symbol<unsafe extern "C" fn()> = c_lib.get(b"shape_manager_cleanup").unwrap();
        let r_cleanup: Symbol<unsafe extern "C" fn()> = r_lib.get(b"shape_manager_cleanup").unwrap();

        c_init();
        r_init();

        let name = CString::new("Test").unwrap();
        let c_scene = c_create(name.as_ptr());
        let r_scene = r_create(name.as_ptr());

        // Add null shape/scene
        assert_eq!(c_add(std::ptr::null_mut(), c_get(0)), r_add(std::ptr::null_mut(), r_get(0)));
        assert_eq!(c_add(c_scene, std::ptr::null_mut()), r_add(r_scene, std::ptr::null_mut()));

        // Add shapes
        for t in [0, 1, 2, 5, 9] {
            let c_ret = c_add(c_scene, c_get(t));
            let r_ret = r_add(r_scene, r_get(t));
            assert_eq!(c_ret, r_ret, "scene_add_shape return mismatch for type {}", t);
        }

        // Remove invalid indices
        for idx in [-1, 5, 99] {
            let c_ret = c_remove(c_scene, idx);
            let r_ret = r_remove(r_scene, idx);
            assert_eq!(c_ret, r_ret, "scene_remove_shape({}) return mismatch", idx);
        }

        // Remove null scene
        assert_eq!(c_remove(std::ptr::null_mut(), 0), r_remove(std::ptr::null_mut(), 0));

        // Remove valid index
        let c_ret = c_remove(c_scene, 1);
        let r_ret = r_remove(r_scene, 1);
        assert_eq!(c_ret, r_ret, "scene_remove_shape(1) return mismatch");

        c_destroy(c_scene);
        r_destroy(r_scene);
        c_cleanup();
        r_cleanup();
    }
}

// ============ scene_print tests ============

#[test]
fn test_scene_print() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_init: Symbol<unsafe extern "C" fn()> = c_lib.get(b"shape_manager_init").unwrap();
        let r_init: Symbol<unsafe extern "C" fn()> = r_lib.get(b"shape_manager_init").unwrap();
        let c_get: Symbol<unsafe extern "C" fn(c_int) -> *mut c_void> = c_lib.get(b"shape_get").unwrap();
        let r_get: Symbol<unsafe extern "C" fn(c_int) -> *mut c_void> = r_lib.get(b"shape_get").unwrap();
        let c_create: Symbol<unsafe extern "C" fn(*const c_char) -> *mut c_void> =
            c_lib.get(b"scene_create").unwrap();
        let r_create: Symbol<unsafe extern "C" fn(*const c_char) -> *mut c_void> =
            r_lib.get(b"scene_create").unwrap();
        let c_add: Symbol<unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int> =
            c_lib.get(b"scene_add_shape").unwrap();
        let r_add: Symbol<unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int> =
            r_lib.get(b"scene_add_shape").unwrap();
        let c_print: Symbol<unsafe extern "C" fn(*const c_void)> =
            c_lib.get(b"scene_print").unwrap();
        let r_print: Symbol<unsafe extern "C" fn(*const c_void)> =
            r_lib.get(b"scene_print").unwrap();
        let c_destroy: Symbol<unsafe extern "C" fn(*mut c_void)> =
            c_lib.get(b"scene_destroy").unwrap();
        let r_destroy: Symbol<unsafe extern "C" fn(*mut c_void)> =
            r_lib.get(b"scene_destroy").unwrap();
        let c_cleanup: Symbol<unsafe extern "C" fn()> = c_lib.get(b"shape_manager_cleanup").unwrap();
        let r_cleanup: Symbol<unsafe extern "C" fn()> = r_lib.get(b"shape_manager_cleanup").unwrap();

        c_init();
        r_init();

        // Test null scene
        let c_out = capture_stdout(|| c_print(std::ptr::null()));
        let r_out = capture_stdout(|| r_print(std::ptr::null()));
        assert_eq!(c_out, r_out, "scene_print(null) mismatch");

        // Test empty scene
        let name = CString::new("Empty").unwrap();
        let c_scene = c_create(name.as_ptr());
        let r_scene = r_create(name.as_ptr());
        let c_out = capture_stdout(|| c_print(c_scene));
        let r_out = capture_stdout(|| r_print(r_scene));
        assert_eq!(c_out, r_out, "scene_print(empty) mismatch");

        // Add shapes and test
        c_add(c_scene, c_get(0));
        r_add(r_scene, r_get(0));
        c_add(c_scene, c_get(3));
        r_add(r_scene, r_get(3));
        let c_out = capture_stdout(|| c_print(c_scene));
        let r_out = capture_stdout(|| r_print(r_scene));
        assert_eq!(c_out, r_out, "scene_print(with shapes) mismatch");

        c_destroy(c_scene);
        r_destroy(r_scene);
        c_cleanup();
        r_cleanup();
    }
}

// ============ scene_equals tests ============

#[test]
fn test_scene_equals() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_init: Symbol<unsafe extern "C" fn()> = c_lib.get(b"shape_manager_init").unwrap();
        let r_init: Symbol<unsafe extern "C" fn()> = r_lib.get(b"shape_manager_init").unwrap();
        let c_get: Symbol<unsafe extern "C" fn(c_int) -> *mut c_void> = c_lib.get(b"shape_get").unwrap();
        let r_get: Symbol<unsafe extern "C" fn(c_int) -> *mut c_void> = r_lib.get(b"shape_get").unwrap();
        let c_create: Symbol<unsafe extern "C" fn(*const c_char) -> *mut c_void> =
            c_lib.get(b"scene_create").unwrap();
        let r_create: Symbol<unsafe extern "C" fn(*const c_char) -> *mut c_void> =
            r_lib.get(b"scene_create").unwrap();
        let c_add: Symbol<unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int> =
            c_lib.get(b"scene_add_shape").unwrap();
        let r_add: Symbol<unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int> =
            r_lib.get(b"scene_add_shape").unwrap();
        let c_eq: Symbol<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int> =
            c_lib.get(b"scene_equals").unwrap();
        let r_eq: Symbol<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int> =
            r_lib.get(b"scene_equals").unwrap();
        let c_destroy: Symbol<unsafe extern "C" fn(*mut c_void)> =
            c_lib.get(b"scene_destroy").unwrap();
        let r_destroy: Symbol<unsafe extern "C" fn(*mut c_void)> =
            r_lib.get(b"scene_destroy").unwrap();
        let c_cleanup: Symbol<unsafe extern "C" fn()> = c_lib.get(b"shape_manager_cleanup").unwrap();
        let r_cleanup: Symbol<unsafe extern "C" fn()> = r_lib.get(b"shape_manager_cleanup").unwrap();

        c_init();
        r_init();

        // null tests
        assert_eq!(c_eq(std::ptr::null(), std::ptr::null()), r_eq(std::ptr::null(), std::ptr::null()));

        let n1 = CString::new("S1").unwrap();
        let n2 = CString::new("S2").unwrap();

        // Equal empty scenes
        let c_s1 = c_create(n1.as_ptr());
        let c_s2 = c_create(n2.as_ptr());
        let r_s1 = r_create(n1.as_ptr());
        let r_s2 = r_create(n2.as_ptr());
        assert_eq!(c_eq(c_s1, c_s2), r_eq(r_s1, r_s2), "empty scenes equals mismatch");

        // Add same shapes to both
        c_add(c_s1, c_get(0)); c_add(c_s2, c_get(0));
        r_add(r_s1, r_get(0)); r_add(r_s2, r_get(0));
        assert_eq!(c_eq(c_s1, c_s2), r_eq(r_s1, r_s2), "same shape equals mismatch");

        // Add different shape to s1 only
        c_add(c_s1, c_get(1));
        r_add(r_s1, r_get(1));
        assert_eq!(c_eq(c_s1, c_s2), r_eq(r_s1, r_s2), "different count equals mismatch");

        // Add different shape to s2
        c_add(c_s2, c_get(2));
        r_add(r_s2, r_get(2));
        assert_eq!(c_eq(c_s1, c_s2), r_eq(r_s1, r_s2), "different shapes equals mismatch");

        c_destroy(c_s1); c_destroy(c_s2);
        r_destroy(r_s1); r_destroy(r_s2);
        c_cleanup();
        r_cleanup();
    }
}

// ============ scene_save / scene_load tests ============

#[test]
fn test_scene_save_load() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_init: Symbol<unsafe extern "C" fn()> = c_lib.get(b"shape_manager_init").unwrap();
        let r_init: Symbol<unsafe extern "C" fn()> = r_lib.get(b"shape_manager_init").unwrap();
        let c_get: Symbol<unsafe extern "C" fn(c_int) -> *mut c_void> = c_lib.get(b"shape_get").unwrap();
        let r_get: Symbol<unsafe extern "C" fn(c_int) -> *mut c_void> = r_lib.get(b"shape_get").unwrap();
        let c_create: Symbol<unsafe extern "C" fn(*const c_char) -> *mut c_void> =
            c_lib.get(b"scene_create").unwrap();
        let r_create: Symbol<unsafe extern "C" fn(*const c_char) -> *mut c_void> =
            r_lib.get(b"scene_create").unwrap();
        let c_add: Symbol<unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int> =
            c_lib.get(b"scene_add_shape").unwrap();
        let r_add: Symbol<unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int> =
            r_lib.get(b"scene_add_shape").unwrap();
        let c_save: Symbol<unsafe extern "C" fn(*const c_void, *const c_char) -> c_int> =
            c_lib.get(b"scene_save").unwrap();
        let r_save: Symbol<unsafe extern "C" fn(*const c_void, *const c_char) -> c_int> =
            r_lib.get(b"scene_save").unwrap();
        let c_load: Symbol<unsafe extern "C" fn(*const c_char) -> *mut c_void> =
            c_lib.get(b"scene_load").unwrap();
        let r_load: Symbol<unsafe extern "C" fn(*const c_char) -> *mut c_void> =
            r_lib.get(b"scene_load").unwrap();
        let c_destroy: Symbol<unsafe extern "C" fn(*mut c_void)> =
            c_lib.get(b"scene_destroy").unwrap();
        let r_destroy: Symbol<unsafe extern "C" fn(*mut c_void)> =
            r_lib.get(b"scene_destroy").unwrap();
        let c_cleanup: Symbol<unsafe extern "C" fn()> = c_lib.get(b"shape_manager_cleanup").unwrap();
        let r_cleanup: Symbol<unsafe extern "C" fn()> = r_lib.get(b"shape_manager_cleanup").unwrap();

        c_init();
        r_init();

        // Save null scene
        let fname = CString::new("/tmp/test_scene_save_null.dat").unwrap();
        let c_ret = c_save(std::ptr::null(), fname.as_ptr());
        let r_ret = r_save(std::ptr::null(), fname.as_ptr());
        assert_eq!(c_ret, r_ret, "scene_save(null) return mismatch");

        // Save with null filename
        let name = CString::new("Test").unwrap();
        let c_scene = c_create(name.as_ptr());
        let r_scene = r_create(name.as_ptr());
        let c_ret = c_save(c_scene, std::ptr::null());
        let r_ret = r_save(r_scene, std::ptr::null());
        assert_eq!(c_ret, r_ret, "scene_save(null filename) return mismatch");

        // Add shapes and save
        c_add(c_scene, c_get(0));
        c_add(c_scene, c_get(3));
        c_add(c_scene, c_get(7));
        r_add(r_scene, r_get(0));
        r_add(r_scene, r_get(3));
        r_add(r_scene, r_get(7));

        let c_fname = CString::new("/tmp/test_c_scene.dat").unwrap();
        let r_fname = CString::new("/tmp/test_r_scene.dat").unwrap();
        let _ = capture_stdout(|| { c_save(c_scene, c_fname.as_ptr()); });
        let _ = capture_stdout(|| { r_save(r_scene, r_fname.as_ptr()); });

        // Compare file contents
        let c_content = std::fs::read_to_string("/tmp/test_c_scene.dat").unwrap();
        let r_content = std::fs::read_to_string("/tmp/test_r_scene.dat").unwrap();
        assert_eq!(c_content, r_content, "scene_save file content mismatch:\nC:  {:?}\nRust: {:?}", c_content, r_content);

        // Load from C-saved file using both libs
        let c_loaded = capture_stdout(|| { c_load(c_fname.as_ptr()); });
        let r_loaded = capture_stdout(|| { r_load(c_fname.as_ptr()); });
        assert_eq!(c_loaded, r_loaded, "scene_load stdout mismatch");

        // Load null filename
        let c_ptr = c_load(std::ptr::null());
        let r_ptr = r_load(std::ptr::null());
        assert!(c_ptr.is_null());
        assert!(r_ptr.is_null());

        c_destroy(c_scene);
        r_destroy(r_scene);
        c_cleanup();
        r_cleanup();

        // Cleanup temp files
        let _ = std::fs::remove_file("/tmp/test_c_scene.dat");
        let _ = std::fs::remove_file("/tmp/test_r_scene.dat");
        let _ = std::fs::remove_file("/tmp/test_scene_save_null.dat");
    }
}

// ============ scene_list_shapes tests ============
// Note: scene_list_shapes prints pointer values which differ between C and Rust.
// We compare the output structure but replace pointer values.

fn normalize_ptrs(s: &str) -> String {
    // Replace hex pointer values like 0x7f... with PTR
    let re = regex_lite_replace(s);
    re
}

fn regex_lite_replace(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '0' && chars.peek() == Some(&'x') {
            chars.next(); // consume 'x'
            // consume hex digits
            while chars.peek().map_or(false, |c| c.is_ascii_hexdigit()) {
                chars.next();
            }
            result.push_str("PTR");
        } else if c == '(' && chars.peek() == Some(&'n') {
            // Check for "(nil)"
            let rest: String = chars.clone().take(3).collect();
            if rest == "il)" {
                for _ in 0..3 { chars.next(); }
                result.push_str("(PTR)");
            } else {
                result.push(c);
            }
        } else {
            result.push(c);
        }
    }
    result
}

#[test]
fn test_scene_list_shapes() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_init: Symbol<unsafe extern "C" fn()> = c_lib.get(b"shape_manager_init").unwrap();
        let r_init: Symbol<unsafe extern "C" fn()> = r_lib.get(b"shape_manager_init").unwrap();
        let c_get: Symbol<unsafe extern "C" fn(c_int) -> *mut c_void> = c_lib.get(b"shape_get").unwrap();
        let r_get: Symbol<unsafe extern "C" fn(c_int) -> *mut c_void> = r_lib.get(b"shape_get").unwrap();
        let c_create: Symbol<unsafe extern "C" fn(*const c_char) -> *mut c_void> =
            c_lib.get(b"scene_create").unwrap();
        let r_create: Symbol<unsafe extern "C" fn(*const c_char) -> *mut c_void> =
            r_lib.get(b"scene_create").unwrap();
        let c_add: Symbol<unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int> =
            c_lib.get(b"scene_add_shape").unwrap();
        let r_add: Symbol<unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int> =
            r_lib.get(b"scene_add_shape").unwrap();
        let c_list: Symbol<unsafe extern "C" fn(*const c_void)> =
            c_lib.get(b"scene_list_shapes").unwrap();
        let r_list: Symbol<unsafe extern "C" fn(*const c_void)> =
            r_lib.get(b"scene_list_shapes").unwrap();
        let c_destroy: Symbol<unsafe extern "C" fn(*mut c_void)> =
            c_lib.get(b"scene_destroy").unwrap();
        let r_destroy: Symbol<unsafe extern "C" fn(*mut c_void)> =
            r_lib.get(b"scene_destroy").unwrap();
        let c_cleanup: Symbol<unsafe extern "C" fn()> = c_lib.get(b"shape_manager_cleanup").unwrap();
        let r_cleanup: Symbol<unsafe extern "C" fn()> = r_lib.get(b"shape_manager_cleanup").unwrap();

        c_init();
        r_init();

        // Test null scene
        let c_out = capture_stdout(|| c_list(std::ptr::null()));
        let r_out = capture_stdout(|| r_list(std::ptr::null()));
        assert_eq!(c_out, r_out, "scene_list_shapes(null) mismatch");

        // Test with shapes
        let name = CString::new("MyScene").unwrap();
        let c_scene = c_create(name.as_ptr());
        let r_scene = r_create(name.as_ptr());
        c_add(c_scene, c_get(0));
        c_add(c_scene, c_get(5));
        r_add(r_scene, r_get(0));
        r_add(r_scene, r_get(5));

        let c_out = capture_stdout(|| c_list(c_scene));
        let r_out = capture_stdout(|| r_list(r_scene));
        // Normalize pointer values before comparing
        assert_eq!(normalize_ptrs(&c_out), normalize_ptrs(&r_out),
            "scene_list_shapes mismatch:\nC:  {:?}\nRust: {:?}", c_out, r_out);

        c_destroy(c_scene);
        r_destroy(r_scene);
        c_cleanup();
        r_cleanup();
    }
}
