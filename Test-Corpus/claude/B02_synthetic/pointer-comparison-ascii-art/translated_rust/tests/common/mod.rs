// tests/common/mod.rs
//
// Shared test infrastructure: loads both the C and Rust shared libraries via
// libloading and exposes their symbols for byte-level comparison tests.

// Items in this module are used by some test files but not others; integration
// tests are compiled per-file so unused-warnings will be reported in any file
// that doesn't reference every helper. Suppress those.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// Global lock — both the C and Rust libraries keep process-wide singleton
/// state (shape_manager) and tests must run serialized.
fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Acquire the global test lock for the duration of the test.
pub fn acquire_lock() -> MutexGuard<'static, ()> {
    match test_lock().lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(), // continue even on poison
    }
}

pub const MAX_SCENE_NAME: usize = 64;
pub const MAX_SHAPES_IN_SCENE: usize = 50;
pub const MAX_SHAPE_NAME: usize = 32;
pub const MAX_SHAPE_WIDTH: usize = 80;
pub const MAX_SHAPE_HEIGHT: usize = 30;
pub const SHAPE_COUNT: c_int = 10;

#[repr(C)]
pub struct ShapeC {
    pub type_: c_int,
    pub name: [c_char; MAX_SHAPE_NAME],
    pub art: [[c_char; MAX_SHAPE_WIDTH]; MAX_SHAPE_HEIGHT],
    pub width: c_int,
    pub height: c_int,
}

#[repr(C)]
pub struct SceneC {
    pub name: [c_char; MAX_SCENE_NAME],
    pub shapes: [*mut ShapeC; MAX_SHAPES_IN_SCENE],
    pub shape_count: c_int,
}

pub struct LoadedLib {
    pub lib: &'static Library,
}

#[allow(dead_code)]
impl LoadedLib {
    pub fn shape_manager_init(&self) {
        unsafe {
            let f: Symbol<unsafe extern "C" fn()> = self.lib.get(b"shape_manager_init").unwrap();
            f();
        }
    }

    pub fn shape_manager_cleanup(&self) {
        unsafe {
            let f: Symbol<unsafe extern "C" fn()> = self.lib.get(b"shape_manager_cleanup").unwrap();
            f();
        }
    }

    pub fn shape_get(&self, t: c_int) -> *mut ShapeC {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int) -> *mut ShapeC> =
                self.lib.get(b"shape_get").unwrap();
            f(t)
        }
    }

    pub fn shape_print(&self, s: *const ShapeC) {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(*const ShapeC)> =
                self.lib.get(b"shape_print").unwrap();
            f(s);
        }
    }

    pub fn shape_equals(&self, a: *const ShapeC, b: *const ShapeC) -> c_int {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(*const ShapeC, *const ShapeC) -> c_int> =
                self.lib.get(b"shape_equals").unwrap();
            f(a, b)
        }
    }

    pub fn shape_type_name(&self, t: c_int) -> *const c_char {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int) -> *const c_char> =
                self.lib.get(b"shape_type_name").unwrap();
            f(t)
        }
    }

    pub fn scene_create(&self, name: *const c_char) -> *mut SceneC {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(*const c_char) -> *mut SceneC> =
                self.lib.get(b"scene_create").unwrap();
            f(name)
        }
    }

    pub fn scene_destroy(&self, s: *mut SceneC) {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(*mut SceneC)> =
                self.lib.get(b"scene_destroy").unwrap();
            f(s);
        }
    }

    pub fn scene_add_shape(&self, s: *mut SceneC, sh: *mut ShapeC) -> c_int {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(*mut SceneC, *mut ShapeC) -> c_int> =
                self.lib.get(b"scene_add_shape").unwrap();
            f(s, sh)
        }
    }

    pub fn scene_remove_shape(&self, s: *mut SceneC, idx: c_int) -> c_int {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(*mut SceneC, c_int) -> c_int> =
                self.lib.get(b"scene_remove_shape").unwrap();
            f(s, idx)
        }
    }

    pub fn scene_print(&self, s: *const SceneC) {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(*const SceneC)> =
                self.lib.get(b"scene_print").unwrap();
            f(s);
        }
    }

    pub fn scene_equals(&self, a: *const SceneC, b: *const SceneC) -> c_int {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(*const SceneC, *const SceneC) -> c_int> =
                self.lib.get(b"scene_equals").unwrap();
            f(a, b)
        }
    }

    pub fn scene_save(&self, s: *const SceneC, fname: *const c_char) -> c_int {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(*const SceneC, *const c_char) -> c_int> =
                self.lib.get(b"scene_save").unwrap();
            f(s, fname)
        }
    }

    pub fn scene_load(&self, fname: *const c_char) -> *mut SceneC {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(*const c_char) -> *mut SceneC> =
                self.lib.get(b"scene_load").unwrap();
            f(fname)
        }
    }

    pub fn scene_list_shapes(&self, s: *const SceneC) {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(*const SceneC)> =
                self.lib.get(b"scene_list_shapes").unwrap();
            f(s);
        }
    }
}

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Load a library and intentionally leak it so it remains mapped for the
/// lifetime of the process. Both libraries register process-wide singleton
/// state (static buffers, atexit handlers); unloading them mid-test or at
/// process shutdown can race against destructors and segfault. Leaking is
/// the safest behavior for a test driver.
fn load_leaked(path: &std::path::Path) -> &'static Library {
    let boxed = Box::new(unsafe { Library::new(path).expect("failed to load lib") });
    Box::leak(boxed)
}

pub fn load_c() -> LoadedLib {
    static ONCE: OnceLock<&'static Library> = OnceLock::new();
    let lib_ref = *ONCE.get_or_init(|| {
        let root = workspace_root();
        let path = root.join("c_lib_build/libdriver_c.so");
        load_leaked(&path)
    });
    LoadedLib { lib: lib_ref }
}

pub fn load_rust() -> LoadedLib {
    static ONCE: OnceLock<&'static Library> = OnceLock::new();
    let lib_ref = *ONCE.get_or_init(|| {
        let root = workspace_root();
        let candidates = [
            root.join("target/debug/libdriver.so"),
            root.join("target/release/libdriver.so"),
        ];
        let path = candidates
            .iter()
            .find(|p| p.exists())
            .expect("Rust libdriver.so not found; run `cargo build` first")
            .clone();
        load_leaked(&path)
    });
    LoadedLib { lib: lib_ref }
}

/// Read a null-terminated C string into bytes (no terminator).
pub fn c_str_to_bytes(p: *const c_char) -> Vec<u8> {
    if p.is_null() {
        return Vec::new();
    }
    let mut out = Vec::new();
    unsafe {
        let mut i = 0isize;
        loop {
            let b = *p.offset(i) as u8;
            if b == 0 {
                break;
            }
            out.push(b);
            i += 1;
        }
    }
    out
}

/// Read the scene's name into a Vec<u8> stripped at first null.
pub fn scene_name_bytes(s: *const SceneC) -> Vec<u8> {
    unsafe {
        let raw_ptr: *const c_char = (&raw const (*s).name) as *const c_char;
        let mut out = Vec::new();
        for i in 0..MAX_SCENE_NAME {
            let b = *raw_ptr.add(i);
            if b == 0 {
                break;
            }
            out.push(b as u8);
        }
        out
    }
}

/// Read shape's name into Vec<u8>.
pub fn shape_name_bytes(s: *const ShapeC) -> Vec<u8> {
    unsafe {
        let raw_ptr: *const c_char = (&raw const (*s).name) as *const c_char;
        let mut out = Vec::new();
        for i in 0..MAX_SHAPE_NAME {
            let b = *raw_ptr.add(i);
            if b == 0 {
                break;
            }
            out.push(b as u8);
        }
        out
    }
}

/// Read a shape art row.
pub fn shape_art_row_bytes(s: *const ShapeC, row: usize) -> Vec<u8> {
    unsafe {
        let row_ptr: *const c_char = (&raw const (*s).art[row]) as *const c_char;
        let mut out = Vec::new();
        for i in 0..MAX_SHAPE_WIDTH {
            let b = *row_ptr.add(i);
            if b == 0 {
                break;
            }
            out.push(b as u8);
        }
        out
    }
}

/// Compare two shape singletons read from the two libraries.
/// They will have *different* addresses, so we compare contents:
/// name, type_, width, height, and art[0..height].
pub fn shapes_content_equal(a: *const ShapeC, b: *const ShapeC) -> bool {
    unsafe {
        if a.is_null() || b.is_null() {
            return a.is_null() && b.is_null();
        }
        if (*a).type_ != (*b).type_
            || (*a).width != (*b).width
            || (*a).height != (*b).height
        {
            return false;
        }
        if shape_name_bytes(a) != shape_name_bytes(b) {
            return false;
        }
        for r in 0..(*a).height as usize {
            if shape_art_row_bytes(a, r) != shape_art_row_bytes(b, r) {
                return false;
            }
        }
        true
    }
}
