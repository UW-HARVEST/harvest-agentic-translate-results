// ffi.rs - C-ABI wrappers for the cdylib.
//
// These wrappers expose the same symbols as the C shared library,
// using C-compatible types (raw pointers, c_char, c_int) so that
// callers loading the library via libloading or linking via the
// usual ELF path can call the implementation through identical
// symbol names. Internally they convert to/from the safer Rust
// representations and call the existing Rust implementations.

use std::ffi::{c_char, c_int, CStr};
use std::ptr;

use crate::shape::{
    self as rshape, shape_get_by_index, shape_manager_cleanup as r_shape_manager_cleanup,
    shape_manager_init as r_shape_manager_init, Shape,
};

// --- C-compatible structs ---
//
// These mirror the layout of the C structs declared in shape.h / scene.h.
// They are used as the FFI representation; we translate to/from the
// Rust-side representations (which use String/Vec) inside the wrappers.

pub const C_MAX_SHAPES_IN_SCENE: usize = 50;
pub const C_MAX_SCENE_NAME: usize = 64;
pub const C_MAX_SHAPE_NAME: usize = 32;
pub const C_MAX_SHAPE_WIDTH: usize = 80;
pub const C_MAX_SHAPE_HEIGHT: usize = 30;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CShape {
    pub shape_type: c_int, // shape_type_t enum -> int
    pub name: [c_char; C_MAX_SHAPE_NAME],
    pub art: [[c_char; C_MAX_SHAPE_WIDTH]; C_MAX_SHAPE_HEIGHT],
    pub width: c_int,
    pub height: c_int,
}

#[repr(C)]
pub struct CScene {
    pub name: [c_char; C_MAX_SCENE_NAME],
    pub shapes: [*mut CShape; C_MAX_SHAPES_IN_SCENE],
    pub shape_count: c_int,
}

// --- helpers ---

fn fill_cstr(dst: &mut [c_char], src: &str) {
    for c in dst.iter_mut() {
        *c = 0;
    }
    let bytes = src.as_bytes();
    let n = bytes.len().min(dst.len().saturating_sub(1));
    for i in 0..n {
        dst[i] = bytes[i] as c_char;
    }
}

fn fill_art_row(dst: &mut [c_char; C_MAX_SHAPE_WIDTH], src: &str) {
    for c in dst.iter_mut() {
        *c = 0;
    }
    let bytes = src.as_bytes();
    let n = bytes.len().min(C_MAX_SHAPE_WIDTH - 1);
    for i in 0..n {
        dst[i] = bytes[i] as c_char;
    }
}

unsafe fn rshape_to_cshape(r: &Shape, dst: &mut CShape) {
    dst.shape_type = r.shape_type.as_i32();
    fill_cstr(&mut dst.name, &r.name);
    for row in dst.art.iter_mut() {
        for c in row.iter_mut() {
            *c = 0;
        }
    }
    for (i, line) in r.art.iter().enumerate() {
        if i >= C_MAX_SHAPE_HEIGHT {
            break;
        }
        fill_art_row(&mut dst.art[i], line);
    }
    dst.width = r.width;
    dst.height = r.height;
}

// --- C-shape singletons (mirrors of the Rust singletons) ---
//
// Whenever scene_t* (CScene) carries shape_t* pointers, the test
// harness compares them by pointer identity. To keep things working
// with the Rust shape singletons and provide stable C-compatible
// pointers, we maintain a parallel table of CShape singletons,
// kept in sync with the Rust shapes.

use std::sync::OnceLock;

#[derive(Clone, Copy)]
struct PtrWrap(*mut CShape);
unsafe impl Send for PtrWrap {}
unsafe impl Sync for PtrWrap {}

static C_SHAPES: OnceLock<std::sync::Mutex<Option<Vec<PtrWrap>>>> = OnceLock::new();

fn c_shapes_table() -> &'static std::sync::Mutex<Option<Vec<PtrWrap>>> {
    C_SHAPES.get_or_init(|| std::sync::Mutex::new(None))
}

fn c_shape_singleton_for_idx(i: c_int) -> *mut CShape {
    let lock = c_shapes_table().lock().unwrap();
    if let Some(v) = lock.as_ref() {
        if i >= 0 && (i as usize) < v.len() {
            return v[i as usize].0;
        }
    }
    ptr::null_mut()
}

// --- exports ---

#[no_mangle]
pub extern "C" fn shape_manager_init() {
    r_shape_manager_init();
    // Build parallel CShape table.
    let mut lock = c_shapes_table().lock().unwrap();
    if lock.is_some() {
        return; // already initialised
    }
    let mut v: Vec<PtrWrap> = Vec::with_capacity(rshape::SHAPE_COUNT as usize);
    for i in 0..rshape::SHAPE_COUNT {
        let rptr = shape_get_by_index(i);
        let mut cshape = unsafe { std::mem::zeroed::<CShape>() };
        if !rptr.is_null() {
            unsafe { rshape_to_cshape(&*rptr, &mut cshape) };
        }
        let b = Box::new(cshape);
        v.push(PtrWrap(Box::into_raw(b)));
    }
    *lock = Some(v);
}

#[no_mangle]
pub extern "C" fn shape_manager_cleanup() {
    r_shape_manager_cleanup();
    let mut lock = c_shapes_table().lock().unwrap();
    if let Some(v) = lock.take() {
        for p in v {
            if !p.0.is_null() {
                unsafe { drop(Box::from_raw(p.0)) };
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn shape_get(t: c_int) -> *mut CShape {
    if t < 0 || t >= rshape::SHAPE_COUNT {
        return ptr::null_mut();
    }
    c_shape_singleton_for_idx(t)
}

#[no_mangle]
pub extern "C" fn shape_print(shape: *const CShape) {
    if shape.is_null() {
        crate::cprint!("(null shape)\n");
        return;
    }
    unsafe {
        let s = &*shape;
        let name = CStr::from_ptr(s.name.as_ptr()).to_string_lossy();
        crate::cprint!("{}:\n", name);
        for i in 0..s.height as usize {
            let row = CStr::from_ptr(s.art[i].as_ptr()).to_string_lossy();
            crate::cprint!("{}\n", row);
        }
    }
    // C uses printf which is line-buffered when TTY / block-buffered otherwise.
    // For predictable test capture, flush at end of every public-facing print.
    crate::out::cout_flush();
}

#[no_mangle]
pub extern "C" fn shape_equals(s1: *const CShape, s2: *const CShape) -> c_int {
    if std::ptr::eq(s1, s2) { 1 } else { 0 }
}

#[no_mangle]
pub extern "C" fn shape_type_name(t: c_int) -> *const c_char {
    // Static C strings for each shape name. Mirrors the C version which
    // returns string literals.
    match t {
        0 => b"Tree\0".as_ptr() as *const c_char,
        1 => b"Tractor\0".as_ptr() as *const c_char,
        2 => b"House\0".as_ptr() as *const c_char,
        3 => b"Sun\0".as_ptr() as *const c_char,
        4 => b"Cloud\0".as_ptr() as *const c_char,
        5 => b"Flower\0".as_ptr() as *const c_char,
        6 => b"Car\0".as_ptr() as *const c_char,
        7 => b"Star\0".as_ptr() as *const c_char,
        8 => b"Heart\0".as_ptr() as *const c_char,
        9 => b"Rainbow\0".as_ptr() as *const c_char,
        _ => b"Unknown\0".as_ptr() as *const c_char,
    }
}

// --- Scene exports ---

#[no_mangle]
pub extern "C" fn scene_create(name: *const c_char) -> *mut CScene {
    let mut cscene = unsafe { std::mem::zeroed::<CScene>() };
    let scene_name = if name.is_null() {
        "Untitled Scene".to_string()
    } else {
        unsafe { CStr::from_ptr(name).to_string_lossy().into_owned() }
    };
    fill_cstr(&mut cscene.name, &scene_name);
    cscene.shape_count = 0;
    Box::into_raw(Box::new(cscene))
}

#[no_mangle]
pub extern "C" fn scene_destroy(scene: *mut CScene) {
    if !scene.is_null() {
        unsafe { drop(Box::from_raw(scene)) };
    }
}

#[no_mangle]
pub extern "C" fn scene_add_shape(scene: *mut CScene, shape: *mut CShape) -> c_int {
    if scene.is_null() || shape.is_null() {
        return -1;
    }
    unsafe {
        let s = &mut *scene;
        if (s.shape_count as usize) >= C_MAX_SHAPES_IN_SCENE {
            crate::ceprint!("Error: Scene is full\n");
            return -1;
        }
        let idx = s.shape_count as usize;
        s.shapes[idx] = shape;
        s.shape_count += 1;
        0
    }
}

#[no_mangle]
pub extern "C" fn scene_remove_shape(scene: *mut CScene, index: c_int) -> c_int {
    if scene.is_null() {
        return -1;
    }
    unsafe {
        let s = &mut *scene;
        if index < 0 || index >= s.shape_count {
            return -1;
        }
        let mut i = index as usize;
        let count = s.shape_count as usize;
        while i < count - 1 {
            s.shapes[i] = s.shapes[i + 1];
            i += 1;
        }
        s.shape_count -= 1;
        0
    }
}

#[no_mangle]
pub extern "C" fn scene_print(scene: *const CScene) {
    if scene.is_null() {
        crate::cprint!("(null scene)\n");
        crate::out::cout_flush();
        return;
    }
    unsafe {
        let s = &*scene;
        let name = CStr::from_ptr(s.name.as_ptr()).to_string_lossy();
        crate::cprint!("\n=== Scene: {} ===\n", name);
        crate::cprint!("Contains {} shape(s)\n\n", s.shape_count);
        for i in 0..s.shape_count as usize {
            crate::cprint!("Shape #{}:\n", i + 1);
            shape_print(s.shapes[i] as *const CShape);
            crate::cprint!("\n");
        }
    }
    crate::out::cout_flush();
}

#[no_mangle]
pub extern "C" fn scene_equals(s1: *const CScene, s2: *const CScene) -> c_int {
    if s1.is_null() || s2.is_null() {
        return 0;
    }
    unsafe {
        let a = &*s1;
        let b = &*s2;
        if a.shape_count != b.shape_count {
            return 0;
        }
        let mut matched = vec![0i32; C_MAX_SHAPES_IN_SCENE];
        for i in 0..a.shape_count as usize {
            let mut found = 0i32;
            for j in 0..b.shape_count as usize {
                if matched[j] == 0
                    && shape_equals(a.shapes[i] as *const CShape, b.shapes[j] as *const CShape) != 0
                {
                    matched[j] = 1;
                    found = 1;
                    break;
                }
            }
            if found == 0 {
                return 0;
            }
        }
        1
    }
}

#[no_mangle]
pub extern "C" fn scene_save(scene: *const CScene, filename: *const c_char) -> c_int {
    if scene.is_null() || filename.is_null() {
        return -1;
    }
    unsafe {
        let fname = CStr::from_ptr(filename).to_string_lossy().into_owned();
        let s = &*scene;
        let name = CStr::from_ptr(s.name.as_ptr()).to_string_lossy().into_owned();

        use std::fs::File;
        use std::io::Write;
        let mut f = match File::create(&fname) {
            Ok(f) => f,
            Err(_) => {
                crate::ceprint!("Error: Could not open file '{}' for writing\n", fname);
                return -1;
            }
        };
        let _ = writeln!(f, "{}", name);
        let _ = writeln!(f, "{}", s.shape_count);
        for i in 0..s.shape_count as usize {
            let sh_ptr = s.shapes[i];
            if sh_ptr.is_null() {
                let _ = writeln!(f, "0");
            } else {
                let sh = &*sh_ptr;
                let _ = writeln!(f, "{}", sh.shape_type);
            }
        }
        drop(f);
        crate::cprint!("Scene saved to '{}'\n", fname);
        crate::out::cout_flush();
        0
    }
}

#[no_mangle]
pub extern "C" fn scene_load(filename: *const c_char) -> *mut CScene {
    if filename.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        let fname = CStr::from_ptr(filename).to_string_lossy().into_owned();
        use std::fs::File;
        use std::io::{BufRead, BufReader, Read};
        let f = match File::open(&fname) {
            Ok(f) => f,
            Err(_) => {
                crate::ceprint!("Error: Could not open file '{}' for reading\n", fname);
                return ptr::null_mut();
            }
        };
        let mut reader = BufReader::new(f);

        // Read scene name (fgets-like)
        let mut name_line = String::new();
        match reader.read_line(&mut name_line) {
            Ok(0) => return ptr::null_mut(),
            Ok(_) => {}
            Err(_) => return ptr::null_mut(),
        }
        if name_line.len() > C_MAX_SCENE_NAME - 1 {
            name_line.truncate(C_MAX_SCENE_NAME - 1);
        }
        if name_line.ends_with('\n') {
            name_line.pop();
        }

        // Build CScene
        let mut cscene = std::mem::zeroed::<CScene>();
        fill_cstr(&mut cscene.name, &name_line);
        let scene_ptr = Box::into_raw(Box::new(cscene));

        // Read shape count using fscanf-like helper
        let shape_count = match read_int_reader(&mut reader) {
            Some(v) => v,
            None => {
                drop(Box::from_raw(scene_ptr));
                return ptr::null_mut();
            }
        };

        for _ in 0..shape_count {
            let t = match read_int_reader(&mut reader) {
                Some(v) => v,
                None => {
                    drop(Box::from_raw(scene_ptr));
                    return ptr::null_mut();
                }
            };
            let csh = if t >= 0 && t < rshape::SHAPE_COUNT {
                c_shape_singleton_for_idx(t)
            } else {
                ptr::null_mut()
            };
            if !csh.is_null() {
                scene_add_shape(scene_ptr, csh);
            }
        }

        crate::cprint!("Scene loaded from '{}'\n", fname);
        crate::out::cout_flush();
        scene_ptr
    }
}

fn read_int_reader<R: std::io::BufRead + std::io::Read>(reader: &mut R) -> Option<i32> {
    use std::io::Read;
    let mut buf = [0u8; 1];
    let mut sign: i32 = 1;
    let mut digits = String::new();
    let last_byte;

    loop {
        let n = reader.read(&mut buf).ok()?;
        if n == 0 {
            return None;
        }
        if !buf[0].is_ascii_whitespace() {
            last_byte = buf[0];
            break;
        }
    }

    if last_byte == b'-' {
        sign = -1;
    } else if last_byte == b'+' {
        // skip
    } else if last_byte.is_ascii_digit() {
        digits.push(last_byte as char);
    } else {
        return None;
    }

    loop {
        let n = match reader.read(&mut buf) {
            Ok(n) => n,
            Err(_) => break,
        };
        if n == 0 {
            break;
        }
        if buf[0].is_ascii_digit() {
            digits.push(buf[0] as char);
        } else {
            break;
        }
    }

    if digits.is_empty() {
        return None;
    }
    digits.parse::<i32>().ok().map(|v| v * sign)
}

#[no_mangle]
pub extern "C" fn scene_list_shapes(scene: *const CScene) {
    if scene.is_null() {
        crate::cprint!("(null scene)\n");
        crate::out::cout_flush();
        return;
    }
    unsafe {
        let s = &*scene;
        let name = CStr::from_ptr(s.name.as_ptr()).to_string_lossy();
        crate::cprint!("\nScene: {}\n", name);
        crate::cprint!("Shapes ({}):\n", s.shape_count);
        for i in 0..s.shape_count as usize {
            let sh_ptr = s.shapes[i];
            if !sh_ptr.is_null() {
                let sh = &*sh_ptr;
                let sname = CStr::from_ptr(sh.name.as_ptr()).to_string_lossy();
                crate::cprint!(
                    "  {}. {} (ptr: {})\n",
                    i + 1,
                    sname,
                    crate::util::format_ptr(sh_ptr as *const u8)
                );
            }
        }
    }
    crate::out::cout_flush();
}
