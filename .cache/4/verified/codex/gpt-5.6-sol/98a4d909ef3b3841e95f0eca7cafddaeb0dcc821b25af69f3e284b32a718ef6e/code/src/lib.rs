use std::ffi::{c_char, c_int, c_void};
use std::ptr;

const MAX_SHAPE_WIDTH: usize = 80;
const MAX_SHAPE_HEIGHT: usize = 30;
const MAX_SHAPE_NAME: usize = 32;
const SHAPE_COUNT: c_int = 10;
const MAX_SHAPES_IN_SCENE: usize = 50;
const MAX_SCENE_NAME: usize = 64;

#[repr(C)]
pub struct Shape {
    pub type_id: c_int,
    pub name: [c_char; MAX_SHAPE_NAME],
    pub art: [[c_char; MAX_SHAPE_WIDTH]; MAX_SHAPE_HEIGHT],
    pub width: c_int,
    pub height: c_int,
}

#[repr(C)]
pub struct Scene {
    pub name: [c_char; MAX_SCENE_NAME],
    pub shapes: [*mut Shape; MAX_SHAPES_IN_SCENE],
    pub shape_count: c_int,
}

#[repr(C)]
struct CFile {
    _private: [u8; 0],
}

unsafe extern "C" {
    static mut stderr: *mut CFile;

    fn malloc(size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
    fn exit(status: c_int) -> !;

    fn printf(format: *const c_char, ...) -> c_int;
    fn fprintf(file: *mut CFile, format: *const c_char, ...) -> c_int;
    fn fopen(filename: *const c_char, mode: *const c_char) -> *mut CFile;
    fn fclose(file: *mut CFile) -> c_int;
    fn fgets(buffer: *mut c_char, size: c_int, file: *mut CFile) -> *mut c_char;
    #[link_name = "__isoc99_fscanf"]
    fn fscanf(file: *mut CFile, format: *const c_char, ...) -> c_int;
}

static mut SHAPES: [*mut Shape; SHAPE_COUNT as usize] = [ptr::null_mut(); SHAPE_COUNT as usize];

const TREE: &[&[u8]] = &[
    b"    /\\    ",
    b"   /  \\   ",
    b"  /____\\  ",
    b"  /    \\  ",
    b" /______\\ ",
    b"    ||    ",
    b"    ||    ",
];
const TRACTOR: &[&[u8]] = &[
    b"      ________     ",
    b"     |        |___ ",
    b"     |  []  []|   |",
    b"  ___|________|___|",
    b" /  o        o   \\",
    b"|___|        |___| ",
];
const HOUSE: &[&[u8]] = &[
    b"     /\\     ",
    b"    /  \\    ",
    b"   /____\\   ",
    b"   |    |   ",
    b"   | [] |   ",
    b"   |    |   ",
    b"   |____|   ",
];
const SUN: &[&[u8]] = &[
    b"  \\  |  / ",
    b"   \\ | /  ",
    b"--- (@) ---",
    b"   / | \\  ",
    b"  /  |  \\ ",
    b"          ",
    b"          ",
];
const CLOUD: &[&[u8]] = &[
    b"   _____       ",
    b"  /     \\_    ",
    b" /  ___  _\\  ",
    b"(__/   \\_)   ",
];
const FLOWER: &[&[u8]] = &[
    b"  \\|/  ",
    b" -(@)- ",
    b"  /|\\  ",
    b"   |   ",
    b"   |   ",
    b"  / \\  ",
    b" /   \\ ",
];
const CAR: &[&[u8]] = &[
    b"  ____       ",
    b" /|_||_\\____ ",
    b"( o     o  ) ",
    b" -----------  ",
];
const STAR: &[&[u8]] = &[
    b"    *    ",
    b"   ***   ",
    b"  *****  ",
    b" ******* ",
    b"*********",
];
const HEART: &[&[u8]] = &[
    b" *** ***  ",
    b"*********  ",
    b"*********  ",
    b" ******* ",
    b"  *****  ",
    b"   ***   ",
];
const RAINBOW: &[&[u8]] = &[
    b"      _______      ",
    b"    /         \\    ",
    b"   /           \\   ",
    b"  /             \\  ",
    b" /               \\ ",
];

const SHAPE_DATA: &[(&[u8], &[&[u8]], c_int)] = &[
    (b"Tree", TREE, 11),
    (b"Tractor", TRACTOR, 20),
    (b"House", HOUSE, 13),
    (b"Sun", SUN, 11),
    (b"Cloud", CLOUD, 16),
    (b"Flower", FLOWER, 9),
    (b"Car", CAR, 16),
    (b"Star", STAR, 9),
    (b"Heart", HEART, 11),
    (b"Rainbow", RAINBOW, 21),
];

const TYPE_NAMES: [&[u8]; SHAPE_COUNT as usize] = [
    b"Tree\0",
    b"Tractor\0",
    b"House\0",
    b"Sun\0",
    b"Cloud\0",
    b"Flower\0",
    b"Car\0",
    b"Star\0",
    b"Heart\0",
    b"Rainbow\0",
];
const UNKNOWN: &[u8] = b"Unknown\0";

unsafe fn copy_c_string(destination: *mut c_char, source: &[u8]) {
    for (index, byte) in source.iter().copied().enumerate() {
        destination.add(index).write(byte as c_char);
    }
    destination.add(source.len()).write(0);
}

unsafe fn initialize_shape(shape: *mut Shape, type_id: usize) {
    let (name, art, width) = SHAPE_DATA[type_id];
    (*shape).type_id = type_id as c_int;
    copy_c_string((*shape).name.as_mut_ptr(), name);
    (*shape).height = art.len() as c_int;
    (*shape).width = width;
    for (row, line) in art.iter().enumerate() {
        copy_c_string((*shape).art[row].as_mut_ptr(), line);
    }
}

#[no_mangle]
pub unsafe extern "C" fn shape_manager_init() {
    for index in 0..SHAPE_COUNT as usize {
        let shape = malloc(std::mem::size_of::<Shape>()) as *mut Shape;
        SHAPES[index] = shape;
        if shape.is_null() {
            fprintf(
                stderr,
                b"Error: Failed to allocate shape\n\0".as_ptr().cast(),
            );
            exit(1);
        }
    }
    for index in 0..SHAPE_COUNT as usize {
        initialize_shape(SHAPES[index], index);
    }
}

#[no_mangle]
pub unsafe extern "C" fn shape_manager_cleanup() {
    for index in 0..SHAPE_COUNT as usize {
        free(SHAPES[index].cast());
        SHAPES[index] = ptr::null_mut();
    }
}

#[no_mangle]
pub unsafe extern "C" fn shape_get(type_id: c_int) -> *mut Shape {
    if !(0..SHAPE_COUNT).contains(&type_id) {
        return ptr::null_mut();
    }
    SHAPES[type_id as usize]
}

#[no_mangle]
pub unsafe extern "C" fn shape_print(shape: *const Shape) {
    if shape.is_null() {
        printf(b"(null shape)\n\0".as_ptr().cast());
        return;
    }

    printf(
        b"%s:\n\0".as_ptr().cast(),
        (*shape).name.as_ptr() as *const c_char,
    );
    for row in 0..(*shape).height {
        printf(
            b"%s\n\0".as_ptr().cast(),
            (*shape).art[row as usize].as_ptr() as *const c_char,
        );
    }
}

#[no_mangle]
pub extern "C" fn shape_equals(first: *const Shape, second: *const Shape) -> c_int {
    c_int::from(first == second)
}

#[no_mangle]
pub extern "C" fn shape_type_name(type_id: c_int) -> *const c_char {
    if let Ok(index) = usize::try_from(type_id) {
        if let Some(name) = TYPE_NAMES.get(index) {
            return name.as_ptr().cast();
        }
    }
    UNKNOWN.as_ptr().cast()
}

#[no_mangle]
pub unsafe extern "C" fn scene_create(name: *const c_char) -> *mut Scene {
    let scene = malloc(std::mem::size_of::<Scene>()) as *mut Scene;
    if scene.is_null() {
        return ptr::null_mut();
    }

    if name.is_null() {
        copy_c_string((*scene).name.as_mut_ptr(), b"Untitled Scene");
    } else {
        let mut index = 0;
        while index < MAX_SCENE_NAME - 1 && name.add(index).read() != 0 {
            (*scene).name[index] = name.add(index).read();
            index += 1;
        }
        while index < MAX_SCENE_NAME - 1 {
            (*scene).name[index] = 0;
            index += 1;
        }
        (*scene).name[MAX_SCENE_NAME - 1] = 0;
    }
    (*scene).shape_count = 0;
    scene
}

#[no_mangle]
pub unsafe extern "C" fn scene_destroy(scene: *mut Scene) {
    if !scene.is_null() {
        free(scene.cast());
    }
}

#[no_mangle]
pub unsafe extern "C" fn scene_add_shape(scene: *mut Scene, shape: *mut Shape) -> c_int {
    if scene.is_null() || shape.is_null() {
        return -1;
    }
    if (*scene).shape_count >= MAX_SHAPES_IN_SCENE as c_int {
        fprintf(stderr, b"Error: Scene is full\n\0".as_ptr().cast());
        return -1;
    }

    let index = (*scene).shape_count as usize;
    (*scene).shapes[index] = shape;
    (*scene).shape_count += 1;
    0
}

#[no_mangle]
pub unsafe extern "C" fn scene_remove_shape(scene: *mut Scene, index: c_int) -> c_int {
    if scene.is_null() || index < 0 || index >= (*scene).shape_count {
        return -1;
    }

    for position in index..((*scene).shape_count - 1) {
        (*scene).shapes[position as usize] = (*scene).shapes[position as usize + 1];
    }
    (*scene).shape_count -= 1;
    0
}

#[no_mangle]
pub unsafe extern "C" fn scene_print(scene: *const Scene) {
    if scene.is_null() {
        printf(b"(null scene)\n\0".as_ptr().cast());
        return;
    }

    printf(
        b"\n=== Scene: %s ===\n\0".as_ptr().cast(),
        (*scene).name.as_ptr(),
    );
    printf(
        b"Contains %d shape(s)\n\n\0".as_ptr().cast(),
        (*scene).shape_count,
    );
    for index in 0..(*scene).shape_count {
        printf(b"Shape #%d:\n\0".as_ptr().cast(), index + 1);
        shape_print((*scene).shapes[index as usize]);
        printf(b"\n\0".as_ptr().cast());
    }
}

#[no_mangle]
pub unsafe extern "C" fn scene_equals(first: *const Scene, second: *const Scene) -> c_int {
    if first.is_null() || second.is_null() {
        return 0;
    }
    if (*first).shape_count != (*second).shape_count {
        return 0;
    }

    let mut matched = [false; MAX_SHAPES_IN_SCENE];
    for first_index in 0..(*first).shape_count {
        let mut found = false;
        for second_index in 0..(*second).shape_count {
            let second_index = second_index as usize;
            if !matched[second_index]
                && shape_equals(
                    (*first).shapes[first_index as usize],
                    (*second).shapes[second_index],
                ) != 0
            {
                matched[second_index] = true;
                found = true;
                break;
            }
        }
        if !found {
            return 0;
        }
    }
    1
}

#[no_mangle]
pub unsafe extern "C" fn scene_save(scene: *const Scene, filename: *const c_char) -> c_int {
    if scene.is_null() || filename.is_null() {
        return -1;
    }

    let file = fopen(filename, b"w\0".as_ptr().cast());
    if file.is_null() {
        fprintf(
            stderr,
            b"Error: Could not open file '%s' for writing\n\0"
                .as_ptr()
                .cast(),
            filename,
        );
        return -1;
    }

    fprintf(file, b"%s\n\0".as_ptr().cast(), (*scene).name.as_ptr());
    fprintf(file, b"%d\n\0".as_ptr().cast(), (*scene).shape_count);
    for index in 0..(*scene).shape_count {
        fprintf(
            file,
            b"%d\n\0".as_ptr().cast(),
            (*(*scene).shapes[index as usize]).type_id,
        );
    }

    fclose(file);
    printf(b"Scene saved to '%s'\n\0".as_ptr().cast(), filename);
    0
}

#[no_mangle]
pub unsafe extern "C" fn scene_load(filename: *const c_char) -> *mut Scene {
    if filename.is_null() {
        return ptr::null_mut();
    }

    let file = fopen(filename, b"r\0".as_ptr().cast());
    if file.is_null() {
        fprintf(
            stderr,
            b"Error: Could not open file '%s' for reading\n\0"
                .as_ptr()
                .cast(),
            filename,
        );
        return ptr::null_mut();
    }

    let mut name = [0 as c_char; MAX_SCENE_NAME];
    if fgets(name.as_mut_ptr(), MAX_SCENE_NAME as c_int, file).is_null() {
        fclose(file);
        return ptr::null_mut();
    }
    if let Some(index) = name.iter().position(|&byte| byte == b'\n' as c_char) {
        name[index] = 0;
    }

    let scene = scene_create(name.as_ptr());
    if scene.is_null() {
        fclose(file);
        return ptr::null_mut();
    }

    let mut shape_count = 0;
    if fscanf(
        file,
        b"%d\n\0".as_ptr().cast(),
        &mut shape_count as *mut c_int,
    ) != 1
    {
        scene_destroy(scene);
        fclose(file);
        return ptr::null_mut();
    }

    for _ in 0..shape_count {
        let mut type_id = 0;
        if fscanf(file, b"%d\n\0".as_ptr().cast(), &mut type_id as *mut c_int) != 1 {
            scene_destroy(scene);
            fclose(file);
            return ptr::null_mut();
        }

        let shape = shape_get(type_id);
        if !shape.is_null() {
            scene_add_shape(scene, shape);
        }
    }

    fclose(file);
    printf(b"Scene loaded from '%s'\n\0".as_ptr().cast(), filename);
    scene
}

#[no_mangle]
pub unsafe extern "C" fn scene_list_shapes(scene: *const Scene) {
    if scene.is_null() {
        printf(b"(null scene)\n\0".as_ptr().cast());
        return;
    }

    printf(b"\nScene: %s\n\0".as_ptr().cast(), (*scene).name.as_ptr());
    printf(b"Shapes (%d):\n\0".as_ptr().cast(), (*scene).shape_count);
    for index in 0..(*scene).shape_count {
        let shape = (*scene).shapes[index as usize];
        printf(
            b"  %d. %s (ptr: %p)\n\0".as_ptr().cast(),
            index + 1,
            (*shape).name.as_ptr(),
            shape.cast::<c_void>(),
        );
    }
}
