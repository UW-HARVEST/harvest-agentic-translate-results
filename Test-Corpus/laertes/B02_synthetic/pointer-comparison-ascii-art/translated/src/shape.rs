extern "C" {
    static mut stderr: *mut _IO_FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn strcpy(
        __dest: *mut libc::c_char,
        __src: *const libc::c_char,
    ) -> *mut libc::c_char;
    fn malloc(__size: size_t) -> *mut libc::c_void;
    fn free(__ptr: *mut libc::c_void);
    fn exit(__status: libc::c_int) -> !;
}
pub use crate::src::scene::size_t;
pub use crate::src::scene::shape_type_t;
pub const SHAPE_COUNT: shape_type_t = 10;
pub const SHAPE_RAINBOW: shape_type_t = 9;
pub const SHAPE_HEART: shape_type_t = 8;
pub const SHAPE_STAR: shape_type_t = 7;
pub const SHAPE_CAR: shape_type_t = 6;
pub const SHAPE_FLOWER: shape_type_t = 5;
pub const SHAPE_CLOUD: shape_type_t = 4;
pub const SHAPE_SUN: shape_type_t = 3;
pub const SHAPE_HOUSE: shape_type_t = 2;
pub const SHAPE_TRACTOR: shape_type_t = 1;
pub const SHAPE_TREE: shape_type_t = 0;
// #[derive(Copy, Clone)]

pub use crate::src::scene::shape_t;
// #[derive(Copy, Clone)]

pub use crate::src::scene::_IO_FILE;
pub use crate::src::scene::__off64_t;
pub use crate::src::scene::_IO_lock_t;
pub use crate::src::scene::__off_t;
// #[derive(Copy, Clone)]

pub use crate::src::scene::_IO_marker;
pub use crate::src::scene::FILE;
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
static mut shapes: [*mut shape_t; 10] = [
    std::ptr::null::<shape_t>() as *mut shape_t,
    std::ptr::null::<shape_t>() as *mut shape_t,
    std::ptr::null::<shape_t>() as *mut shape_t,
    std::ptr::null::<shape_t>() as *mut shape_t,
    std::ptr::null::<shape_t>() as *mut shape_t,
    std::ptr::null::<shape_t>() as *mut shape_t,
    std::ptr::null::<shape_t>() as *mut shape_t,
    std::ptr::null::<shape_t>() as *mut shape_t,
    std::ptr::null::<shape_t>() as *mut shape_t,
    std::ptr::null::<shape_t>() as *mut shape_t,
];
unsafe extern "C" fn init_tree(mut shape: *mut shape_t) {
    (*shape).type_0 = SHAPE_TREE;
    strcpy(
        &raw mut (*shape).name as *mut libc::c_char,
        b"Tree\0" as *const u8 as *const libc::c_char,
    );
    (*shape).height = 7 as libc::c_int;
    (*shape).width = 11 as libc::c_int;
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(0 as libc::c_int as isize) as *mut libc::c_char,
        b"    /\\    \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(1 as libc::c_int as isize) as *mut libc::c_char,
        b"   /  \\   \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(2 as libc::c_int as isize) as *mut libc::c_char,
        b"  /____\\  \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(3 as libc::c_int as isize) as *mut libc::c_char,
        b"  /    \\  \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(4 as libc::c_int as isize) as *mut libc::c_char,
        b" /______\\ \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(5 as libc::c_int as isize) as *mut libc::c_char,
        b"    ||    \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(6 as libc::c_int as isize) as *mut libc::c_char,
        b"    ||    \0" as *const u8 as *const libc::c_char,
    );
}
unsafe extern "C" fn init_tractor(mut shape: *mut shape_t) {
    (*shape).type_0 = SHAPE_TRACTOR;
    strcpy(
        &raw mut (*shape).name as *mut libc::c_char,
        b"Tractor\0" as *const u8 as *const libc::c_char,
    );
    (*shape).height = 6 as libc::c_int;
    (*shape).width = 20 as libc::c_int;
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(0 as libc::c_int as isize) as *mut libc::c_char,
        b"      ________     \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(1 as libc::c_int as isize) as *mut libc::c_char,
        b"     |        |___ \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(2 as libc::c_int as isize) as *mut libc::c_char,
        b"     |  []  []|   |\0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(3 as libc::c_int as isize) as *mut libc::c_char,
        b"  ___|________|___|\0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(4 as libc::c_int as isize) as *mut libc::c_char,
        b" /  o        o   \\\0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(5 as libc::c_int as isize) as *mut libc::c_char,
        b"|___|        |___| \0" as *const u8 as *const libc::c_char,
    );
}
unsafe extern "C" fn init_house(mut shape: *mut shape_t) {
    (*shape).type_0 = SHAPE_HOUSE;
    strcpy(
        &raw mut (*shape).name as *mut libc::c_char,
        b"House\0" as *const u8 as *const libc::c_char,
    );
    (*shape).height = 7 as libc::c_int;
    (*shape).width = 13 as libc::c_int;
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(0 as libc::c_int as isize) as *mut libc::c_char,
        b"     /\\     \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(1 as libc::c_int as isize) as *mut libc::c_char,
        b"    /  \\    \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(2 as libc::c_int as isize) as *mut libc::c_char,
        b"   /____\\   \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(3 as libc::c_int as isize) as *mut libc::c_char,
        b"   |    |   \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(4 as libc::c_int as isize) as *mut libc::c_char,
        b"   | [] |   \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(5 as libc::c_int as isize) as *mut libc::c_char,
        b"   |    |   \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(6 as libc::c_int as isize) as *mut libc::c_char,
        b"   |____|   \0" as *const u8 as *const libc::c_char,
    );
}
unsafe extern "C" fn init_sun(mut shape: *mut shape_t) {
    (*shape).type_0 = SHAPE_SUN;
    strcpy(
        &raw mut (*shape).name as *mut libc::c_char,
        b"Sun\0" as *const u8 as *const libc::c_char,
    );
    (*shape).height = 7 as libc::c_int;
    (*shape).width = 11 as libc::c_int;
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(0 as libc::c_int as isize) as *mut libc::c_char,
        b"  \\  |  / \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(1 as libc::c_int as isize) as *mut libc::c_char,
        b"   \\ | /  \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(2 as libc::c_int as isize) as *mut libc::c_char,
        b"--- (@) ---\0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(3 as libc::c_int as isize) as *mut libc::c_char,
        b"   / | \\  \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(4 as libc::c_int as isize) as *mut libc::c_char,
        b"  /  |  \\ \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(5 as libc::c_int as isize) as *mut libc::c_char,
        b"          \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(6 as libc::c_int as isize) as *mut libc::c_char,
        b"          \0" as *const u8 as *const libc::c_char,
    );
}
unsafe extern "C" fn init_cloud(mut shape: *mut shape_t) {
    (*shape).type_0 = SHAPE_CLOUD;
    strcpy(
        &raw mut (*shape).name as *mut libc::c_char,
        b"Cloud\0" as *const u8 as *const libc::c_char,
    );
    (*shape).height = 4 as libc::c_int;
    (*shape).width = 16 as libc::c_int;
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(0 as libc::c_int as isize) as *mut libc::c_char,
        b"   _____       \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(1 as libc::c_int as isize) as *mut libc::c_char,
        b"  /     \\_    \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(2 as libc::c_int as isize) as *mut libc::c_char,
        b" /  ___  _\\  \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(3 as libc::c_int as isize) as *mut libc::c_char,
        b"(__/   \\_)   \0" as *const u8 as *const libc::c_char,
    );
}
unsafe extern "C" fn init_flower(mut shape: *mut shape_t) {
    (*shape).type_0 = SHAPE_FLOWER;
    strcpy(
        &raw mut (*shape).name as *mut libc::c_char,
        b"Flower\0" as *const u8 as *const libc::c_char,
    );
    (*shape).height = 7 as libc::c_int;
    (*shape).width = 9 as libc::c_int;
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(0 as libc::c_int as isize) as *mut libc::c_char,
        b"  \\|/  \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(1 as libc::c_int as isize) as *mut libc::c_char,
        b" -(@)- \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(2 as libc::c_int as isize) as *mut libc::c_char,
        b"  /|\\  \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(3 as libc::c_int as isize) as *mut libc::c_char,
        b"   |   \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(4 as libc::c_int as isize) as *mut libc::c_char,
        b"   |   \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(5 as libc::c_int as isize) as *mut libc::c_char,
        b"  / \\  \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(6 as libc::c_int as isize) as *mut libc::c_char,
        b" /   \\ \0" as *const u8 as *const libc::c_char,
    );
}
unsafe extern "C" fn init_car(mut shape: *mut shape_t) {
    (*shape).type_0 = SHAPE_CAR;
    strcpy(
        &raw mut (*shape).name as *mut libc::c_char,
        b"Car\0" as *const u8 as *const libc::c_char,
    );
    (*shape).height = 4 as libc::c_int;
    (*shape).width = 16 as libc::c_int;
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(0 as libc::c_int as isize) as *mut libc::c_char,
        b"  ____       \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(1 as libc::c_int as isize) as *mut libc::c_char,
        b" /|_||_\\____ \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(2 as libc::c_int as isize) as *mut libc::c_char,
        b"( o     o  ) \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(3 as libc::c_int as isize) as *mut libc::c_char,
        b" -----------  \0" as *const u8 as *const libc::c_char,
    );
}
unsafe extern "C" fn init_star(mut shape: *mut shape_t) {
    (*shape).type_0 = SHAPE_STAR;
    strcpy(
        &raw mut (*shape).name as *mut libc::c_char,
        b"Star\0" as *const u8 as *const libc::c_char,
    );
    (*shape).height = 5 as libc::c_int;
    (*shape).width = 9 as libc::c_int;
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(0 as libc::c_int as isize) as *mut libc::c_char,
        b"    *    \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(1 as libc::c_int as isize) as *mut libc::c_char,
        b"   ***   \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(2 as libc::c_int as isize) as *mut libc::c_char,
        b"  *****  \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(3 as libc::c_int as isize) as *mut libc::c_char,
        b" ******* \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(4 as libc::c_int as isize) as *mut libc::c_char,
        b"*********\0" as *const u8 as *const libc::c_char,
    );
}
unsafe extern "C" fn init_heart(mut shape: *mut shape_t) {
    (*shape).type_0 = SHAPE_HEART;
    strcpy(
        &raw mut (*shape).name as *mut libc::c_char,
        b"Heart\0" as *const u8 as *const libc::c_char,
    );
    (*shape).height = 6 as libc::c_int;
    (*shape).width = 11 as libc::c_int;
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(0 as libc::c_int as isize) as *mut libc::c_char,
        b" *** ***  \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(1 as libc::c_int as isize) as *mut libc::c_char,
        b"*********  \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(2 as libc::c_int as isize) as *mut libc::c_char,
        b"*********  \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(3 as libc::c_int as isize) as *mut libc::c_char,
        b" ******* \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(4 as libc::c_int as isize) as *mut libc::c_char,
        b"  *****  \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(5 as libc::c_int as isize) as *mut libc::c_char,
        b"   ***   \0" as *const u8 as *const libc::c_char,
    );
}
unsafe extern "C" fn init_rainbow(mut shape: *mut shape_t) {
    (*shape).type_0 = SHAPE_RAINBOW;
    strcpy(
        &raw mut (*shape).name as *mut libc::c_char,
        b"Rainbow\0" as *const u8 as *const libc::c_char,
    );
    (*shape).height = 5 as libc::c_int;
    (*shape).width = 21 as libc::c_int;
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(0 as libc::c_int as isize) as *mut libc::c_char,
        b"      _______      \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(1 as libc::c_int as isize) as *mut libc::c_char,
        b"    /         \\    \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(2 as libc::c_int as isize) as *mut libc::c_char,
        b"   /           \\   \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(3 as libc::c_int as isize) as *mut libc::c_char,
        b"  /             \\  \0" as *const u8 as *const libc::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [libc::c_char; 80])
            .offset(4 as libc::c_int as isize) as *mut libc::c_char,
        b" /               \\ \0" as *const u8 as *const libc::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn shape_manager_init() {
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < SHAPE_COUNT as libc::c_int {
        shapes[i as usize] = malloc(std::mem::size_of::<shape_t>() as size_t) as *mut shape_t;
        if shapes[i as usize].is_null() {
            fprintf(
                stderr as *mut FILE,
                b"Error: Failed to allocate shape\n\0" as *const u8 as *const libc::c_char,
            );
            exit(1 as libc::c_int);
        }
        i += 1;
    }
    init_tree(shapes[SHAPE_TREE as libc::c_int as usize]);
    init_tractor(shapes[SHAPE_TRACTOR as libc::c_int as usize]);
    init_house(shapes[SHAPE_HOUSE as libc::c_int as usize]);
    init_sun(shapes[SHAPE_SUN as libc::c_int as usize]);
    init_cloud(shapes[SHAPE_CLOUD as libc::c_int as usize]);
    init_flower(shapes[SHAPE_FLOWER as libc::c_int as usize]);
    init_car(shapes[SHAPE_CAR as libc::c_int as usize]);
    init_star(shapes[SHAPE_STAR as libc::c_int as usize]);
    init_heart(shapes[SHAPE_HEART as libc::c_int as usize]);
    init_rainbow(shapes[SHAPE_RAINBOW as libc::c_int as usize]);
}
#[no_mangle]
pub unsafe extern "C" fn shape_manager_cleanup() {
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < SHAPE_COUNT as libc::c_int {
        free(shapes[i as usize] as *mut libc::c_void);
        shapes[i as usize] = std::ptr::null_mut::<shape_t>();
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn shape_get(mut type_0: shape_type_t) -> *mut shape_t {
    if (type_0 as libc::c_uint) < 0 as libc::c_uint
        || type_0 as libc::c_uint >= SHAPE_COUNT as libc::c_int as libc::c_uint
    {
        return std::ptr::null_mut::<shape_t>();
    }
    return shapes[type_0 as usize];
}
#[no_mangle]
pub unsafe extern "C" fn shape_print(mut shape: *const shape_t) {
    if shape.is_null() {
        printf(b"(null shape)\n\0" as *const u8 as *const libc::c_char);
        return;
    }
    printf(
        b"%s:\n\0" as *const u8 as *const libc::c_char,
        &raw const (*shape).name as *const libc::c_char,
    );
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < (*shape).height {
        printf(
            b"%s\n\0" as *const u8 as *const libc::c_char,
            &raw const *(&raw const (*shape).art as *const [libc::c_char; 80])
                .offset(i as isize) as *const libc::c_char,
        );
        i += 1;
    }
}
#[no_mangle]
pub extern "C" fn shape_equals(
    mut s1: *const shape_t,
    mut s2: *const shape_t,
) -> libc::c_int {
    return if s1 == s2 {
        1 as libc::c_int
    } else {
        0 as libc::c_int
    };
}
#[no_mangle]
pub extern "C" fn shape_type_name(mut type_0: shape_type_t) -> *const libc::c_char {
    match type_0 as libc::c_uint {
        0 => return b"Tree\0" as *const u8 as *const libc::c_char,
        1 => return b"Tractor\0" as *const u8 as *const libc::c_char,
        2 => return b"House\0" as *const u8 as *const libc::c_char,
        3 => return b"Sun\0" as *const u8 as *const libc::c_char,
        4 => return b"Cloud\0" as *const u8 as *const libc::c_char,
        5 => return b"Flower\0" as *const u8 as *const libc::c_char,
        6 => return b"Car\0" as *const u8 as *const libc::c_char,
        7 => return b"Star\0" as *const u8 as *const libc::c_char,
        8 => return b"Heart\0" as *const u8 as *const libc::c_char,
        9 => return b"Rainbow\0" as *const u8 as *const libc::c_char,
        _ => return b"Unknown\0" as *const u8 as *const libc::c_char,
    };
}
