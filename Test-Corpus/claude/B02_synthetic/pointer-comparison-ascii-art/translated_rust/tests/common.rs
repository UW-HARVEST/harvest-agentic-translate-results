// Common helper for integration tests.
// We avoid depending on crate internals: we mirror the C struct layout
// here exactly so that we can call the C and the Rust shared-library
// implementations through identical function signatures.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// Global mutex to serialize tests that touch shape_manager_* state.
/// Both the C and the Rust libraries keep singleton tables that are
/// initialised once and torn down on cleanup; running tests in parallel
/// against the same library would either re-init or use freed pointers.
pub fn serialize() -> MutexGuard<'static, ()> {
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

pub const MAX_SHAPES_IN_SCENE: usize = 50;
pub const MAX_SCENE_NAME: usize = 64;
pub const MAX_SHAPE_NAME: usize = 32;
pub const MAX_SHAPE_WIDTH: usize = 80;
pub const MAX_SHAPE_HEIGHT: usize = 30;
pub const SHAPE_COUNT: c_int = 10;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CShape {
    pub shape_type: c_int,
    pub name: [c_char; MAX_SHAPE_NAME],
    pub art: [[c_char; MAX_SHAPE_WIDTH]; MAX_SHAPE_HEIGHT],
    pub width: c_int,
    pub height: c_int,
}

#[repr(C)]
pub struct CScene {
    pub name: [c_char; MAX_SCENE_NAME],
    pub shapes: [*mut CShape; MAX_SHAPES_IN_SCENE],
    pub shape_count: c_int,
}

pub type FnVoid = unsafe extern "C" fn();
pub type FnShapeGet = unsafe extern "C" fn(c_int) -> *mut CShape;
pub type FnShapePrint = unsafe extern "C" fn(*const CShape);
pub type FnShapeEquals = unsafe extern "C" fn(*const CShape, *const CShape) -> c_int;
pub type FnShapeTypeName = unsafe extern "C" fn(c_int) -> *const c_char;
pub type FnSceneCreate = unsafe extern "C" fn(*const c_char) -> *mut CScene;
pub type FnSceneDestroy = unsafe extern "C" fn(*mut CScene);
pub type FnSceneAddShape = unsafe extern "C" fn(*mut CScene, *mut CShape) -> c_int;
pub type FnSceneRemoveShape = unsafe extern "C" fn(*mut CScene, c_int) -> c_int;
pub type FnScenePrint = unsafe extern "C" fn(*const CScene);
pub type FnSceneEquals = unsafe extern "C" fn(*const CScene, *const CScene) -> c_int;
pub type FnSceneSave = unsafe extern "C" fn(*const CScene, *const c_char) -> c_int;
pub type FnSceneLoad = unsafe extern "C" fn(*const c_char) -> *mut CScene;
pub type FnSceneListShapes = unsafe extern "C" fn(*const CScene);

pub struct ApiSyms {
    pub _lib: Library,
    pub shape_manager_init: FnVoid,
    pub shape_manager_cleanup: FnVoid,
    pub shape_get: FnShapeGet,
    pub shape_print: FnShapePrint,
    pub shape_equals: FnShapeEquals,
    pub shape_type_name: FnShapeTypeName,
    pub scene_create: FnSceneCreate,
    pub scene_destroy: FnSceneDestroy,
    pub scene_add_shape: FnSceneAddShape,
    pub scene_remove_shape: FnSceneRemoveShape,
    pub scene_print: FnScenePrint,
    pub scene_equals: FnSceneEquals,
    pub scene_save: FnSceneSave,
    pub scene_load: FnSceneLoad,
    pub scene_list_shapes: FnSceneListShapes,
}

impl ApiSyms {
    pub fn load(lib_path: &str) -> ApiSyms {
        unsafe {
            let lib = Library::new(lib_path)
                .unwrap_or_else(|e| panic!("failed to load {}: {}", lib_path, e));
            macro_rules! sym {
                ($name:literal, $t:ty) => {{
                    let s: Symbol<$t> = lib
                        .get($name.as_bytes())
                        .unwrap_or_else(|e| panic!("missing symbol {}: {}", $name, e));
                    *s.into_raw()
                }};
            }
            ApiSyms {
                shape_manager_init: sym!("shape_manager_init", FnVoid),
                shape_manager_cleanup: sym!("shape_manager_cleanup", FnVoid),
                shape_get: sym!("shape_get", FnShapeGet),
                shape_print: sym!("shape_print", FnShapePrint),
                shape_equals: sym!("shape_equals", FnShapeEquals),
                shape_type_name: sym!("shape_type_name", FnShapeTypeName),
                scene_create: sym!("scene_create", FnSceneCreate),
                scene_destroy: sym!("scene_destroy", FnSceneDestroy),
                scene_add_shape: sym!("scene_add_shape", FnSceneAddShape),
                scene_remove_shape: sym!("scene_remove_shape", FnSceneRemoveShape),
                scene_print: sym!("scene_print", FnScenePrint),
                scene_equals: sym!("scene_equals", FnSceneEquals),
                scene_save: sym!("scene_save", FnSceneSave),
                scene_load: sym!("scene_load", FnSceneLoad),
                scene_list_shapes: sym!("scene_list_shapes", FnSceneListShapes),
                _lib: lib,
            }
        }
    }
}

pub fn c_lib_path() -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver_c.so");
    p.to_string_lossy().into_owned()
}

pub fn rust_lib_path() -> String {
    // The integration tests use whatever profile cargo is using.
    // We accept either debug or release builds.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for sub in &["target/release/libdriver.so", "target/debug/libdriver.so"] {
        let p = manifest.join(sub);
        if p.exists() {
            return p.to_string_lossy().into_owned();
        }
    }
    panic!("Rust libdriver.so not found in target/{{release,debug}}");
}

pub fn cstring(s: &str) -> CString {
    CString::new(s).unwrap()
}

pub fn buf_to_string(buf: &[c_char]) -> String {
    let mut v = Vec::new();
    for &c in buf {
        if c == 0 {
            break;
        }
        v.push(c as u8);
    }
    String::from_utf8_lossy(&v).into_owned()
}
