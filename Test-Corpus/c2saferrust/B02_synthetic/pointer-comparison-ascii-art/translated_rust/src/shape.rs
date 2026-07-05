extern "C" {
    static mut stderr: *mut _IO_FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn strcpy(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn exit(__status: ::core::ffi::c_int) -> !;
}
pub type size_t = usize;
pub type shape_type_t = ::core::ffi::c_uint;
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct shape_t {
    pub type_0: shape_type_t,
    pub name: [::core::ffi::c_char; 32],
    pub art: [[::core::ffi::c_char; 80]; 30],
    pub width: ::core::ffi::c_int,
    pub height: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: ::core::ffi::c_int,
    pub _IO_read_ptr: *mut ::core::ffi::c_char,
    pub _IO_read_end: *mut ::core::ffi::c_char,
    pub _IO_read_base: *mut ::core::ffi::c_char,
    pub _IO_write_base: *mut ::core::ffi::c_char,
    pub _IO_write_ptr: *mut ::core::ffi::c_char,
    pub _IO_write_end: *mut ::core::ffi::c_char,
    pub _IO_buf_base: *mut ::core::ffi::c_char,
    pub _IO_buf_end: *mut ::core::ffi::c_char,
    pub _IO_save_base: *mut ::core::ffi::c_char,
    pub _IO_backup_base: *mut ::core::ffi::c_char,
    pub _IO_save_end: *mut ::core::ffi::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: ::core::ffi::c_int,
    pub _flags2: ::core::ffi::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: ::core::ffi::c_ushort,
    pub _vtable_offset: ::core::ffi::c_schar,
    pub _shortbuf: [::core::ffi::c_char; 1],
    pub _lock: *mut ::core::ffi::c_void,
    pub _offset: __off64_t,
    pub __pad1: *mut ::core::ffi::c_void,
    pub __pad2: *mut ::core::ffi::c_void,
    pub __pad3: *mut ::core::ffi::c_void,
    pub __pad4: *mut ::core::ffi::c_void,
    pub __pad5: size_t,
    pub _mode: ::core::ffi::c_int,
    pub _unused2: [::core::ffi::c_char; 20],
}
pub type __off64_t = ::core::ffi::c_long;
pub type _IO_lock_t = ();
pub type __off_t = ::core::ffi::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_marker {
    pub _next: *mut _IO_marker,
    pub _sbuf: *mut _IO_FILE,
    pub _pos: ::core::ffi::c_int,
}
pub type FILE = _IO_FILE;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
static mut shapes: [*mut shape_t; 10] = [
    ::core::ptr::null::<shape_t>() as *mut shape_t,
    ::core::ptr::null::<shape_t>() as *mut shape_t,
    ::core::ptr::null::<shape_t>() as *mut shape_t,
    ::core::ptr::null::<shape_t>() as *mut shape_t,
    ::core::ptr::null::<shape_t>() as *mut shape_t,
    ::core::ptr::null::<shape_t>() as *mut shape_t,
    ::core::ptr::null::<shape_t>() as *mut shape_t,
    ::core::ptr::null::<shape_t>() as *mut shape_t,
    ::core::ptr::null::<shape_t>() as *mut shape_t,
    ::core::ptr::null::<shape_t>() as *mut shape_t,
];
unsafe extern "C" fn init_tree(mut shape: *mut shape_t) {
    (*shape).type_0 = SHAPE_TREE;
    strcpy(
        &raw mut (*shape).name as *mut ::core::ffi::c_char,
        b"Tree\0" as *const u8 as *const ::core::ffi::c_char,
    );
    (*shape).height = 7 as ::core::ffi::c_int;
    (*shape).width = 11 as ::core::ffi::c_int;
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(0 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"    /\\    \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"   /  \\   \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(2 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"  /____\\  \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(3 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"  /    \\  \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(4 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b" /______\\ \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(5 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"    ||    \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(6 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"    ||    \0" as *const u8 as *const ::core::ffi::c_char,
    );
}
unsafe extern "C" fn init_tractor(mut shape: *mut shape_t) {
    (*shape).type_0 = SHAPE_TRACTOR;
    strcpy(
        &raw mut (*shape).name as *mut ::core::ffi::c_char,
        b"Tractor\0" as *const u8 as *const ::core::ffi::c_char,
    );
    (*shape).height = 6 as ::core::ffi::c_int;
    (*shape).width = 20 as ::core::ffi::c_int;
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(0 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"      ________     \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"     |        |___ \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(2 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"     |  []  []|   |\0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(3 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"  ___|________|___|\0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(4 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b" /  o        o   \\\0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(5 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"|___|        |___| \0" as *const u8 as *const ::core::ffi::c_char,
    );
}
unsafe extern "C" fn init_house(mut shape: *mut shape_t) {
    (*shape).type_0 = SHAPE_HOUSE;
    strcpy(
        &raw mut (*shape).name as *mut ::core::ffi::c_char,
        b"House\0" as *const u8 as *const ::core::ffi::c_char,
    );
    (*shape).height = 7 as ::core::ffi::c_int;
    (*shape).width = 13 as ::core::ffi::c_int;
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(0 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"     /\\     \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"    /  \\    \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(2 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"   /____\\   \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(3 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"   |    |   \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(4 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"   | [] |   \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(5 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"   |    |   \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(6 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"   |____|   \0" as *const u8 as *const ::core::ffi::c_char,
    );
}
unsafe extern "C" fn init_sun(mut shape: *mut shape_t) {
    (*shape).type_0 = SHAPE_SUN;
    strcpy(
        &raw mut (*shape).name as *mut ::core::ffi::c_char,
        b"Sun\0" as *const u8 as *const ::core::ffi::c_char,
    );
    (*shape).height = 7 as ::core::ffi::c_int;
    (*shape).width = 11 as ::core::ffi::c_int;
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(0 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"  \\  |  / \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"   \\ | /  \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(2 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"--- (@) ---\0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(3 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"   / | \\  \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(4 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"  /  |  \\ \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(5 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"          \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(6 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"          \0" as *const u8 as *const ::core::ffi::c_char,
    );
}
unsafe extern "C" fn init_cloud(mut shape: *mut shape_t) {
    (*shape).type_0 = SHAPE_CLOUD;
    strcpy(
        &raw mut (*shape).name as *mut ::core::ffi::c_char,
        b"Cloud\0" as *const u8 as *const ::core::ffi::c_char,
    );
    (*shape).height = 4 as ::core::ffi::c_int;
    (*shape).width = 16 as ::core::ffi::c_int;
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(0 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"   _____       \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"  /     \\_    \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(2 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b" /  ___  _\\  \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(3 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"(__/   \\_)   \0" as *const u8 as *const ::core::ffi::c_char,
    );
}
unsafe extern "C" fn init_flower(mut shape: *mut shape_t) {
    (*shape).type_0 = SHAPE_FLOWER;
    strcpy(
        &raw mut (*shape).name as *mut ::core::ffi::c_char,
        b"Flower\0" as *const u8 as *const ::core::ffi::c_char,
    );
    (*shape).height = 7 as ::core::ffi::c_int;
    (*shape).width = 9 as ::core::ffi::c_int;
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(0 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"  \\|/  \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b" -(@)- \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(2 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"  /|\\  \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(3 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"   |   \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(4 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"   |   \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(5 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"  / \\  \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(6 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b" /   \\ \0" as *const u8 as *const ::core::ffi::c_char,
    );
}
unsafe extern "C" fn init_car(mut shape: *mut shape_t) {
    (*shape).type_0 = SHAPE_CAR;
    strcpy(
        &raw mut (*shape).name as *mut ::core::ffi::c_char,
        b"Car\0" as *const u8 as *const ::core::ffi::c_char,
    );
    (*shape).height = 4 as ::core::ffi::c_int;
    (*shape).width = 16 as ::core::ffi::c_int;
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(0 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"  ____       \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b" /|_||_\\____ \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(2 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"( o     o  ) \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(3 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b" -----------  \0" as *const u8 as *const ::core::ffi::c_char,
    );
}
unsafe extern "C" fn init_star(mut shape: *mut shape_t) {
    (*shape).type_0 = SHAPE_STAR;
    strcpy(
        &raw mut (*shape).name as *mut ::core::ffi::c_char,
        b"Star\0" as *const u8 as *const ::core::ffi::c_char,
    );
    (*shape).height = 5 as ::core::ffi::c_int;
    (*shape).width = 9 as ::core::ffi::c_int;
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(0 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"    *    \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"   ***   \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(2 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"  *****  \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(3 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b" ******* \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(4 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"*********\0" as *const u8 as *const ::core::ffi::c_char,
    );
}
unsafe extern "C" fn init_heart(mut shape: *mut shape_t) {
    (*shape).type_0 = SHAPE_HEART;
    strcpy(
        &raw mut (*shape).name as *mut ::core::ffi::c_char,
        b"Heart\0" as *const u8 as *const ::core::ffi::c_char,
    );
    (*shape).height = 6 as ::core::ffi::c_int;
    (*shape).width = 11 as ::core::ffi::c_int;
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(0 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b" *** ***  \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"*********  \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(2 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"*********  \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(3 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b" ******* \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(4 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"  *****  \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(5 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"   ***   \0" as *const u8 as *const ::core::ffi::c_char,
    );
}
unsafe extern "C" fn init_rainbow(mut shape: *mut shape_t) {
    (*shape).type_0 = SHAPE_RAINBOW;
    strcpy(
        &raw mut (*shape).name as *mut ::core::ffi::c_char,
        b"Rainbow\0" as *const u8 as *const ::core::ffi::c_char,
    );
    (*shape).height = 5 as ::core::ffi::c_int;
    (*shape).width = 21 as ::core::ffi::c_int;
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(0 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"      _______      \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"    /         \\    \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(2 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"   /           \\   \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(3 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b"  /             \\  \0" as *const u8 as *const ::core::ffi::c_char,
    );
    strcpy(
        &raw mut *(&raw mut (*shape).art as *mut [::core::ffi::c_char; 80])
            .offset(4 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
        b" /               \\ \0" as *const u8 as *const ::core::ffi::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn shape_manager_init() {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < SHAPE_COUNT as ::core::ffi::c_int {
        shapes[i as usize] = malloc(::core::mem::size_of::<shape_t>() as size_t) as *mut shape_t;
        if shapes[i as usize].is_null() {
            fprintf(
                stderr as *mut FILE,
                b"Error: Failed to allocate shape\n\0" as *const u8 as *const ::core::ffi::c_char,
            );
            exit(1 as ::core::ffi::c_int);
        }
        i += 1;
    }
    init_tree(shapes[SHAPE_TREE as ::core::ffi::c_int as usize]);
    init_tractor(shapes[SHAPE_TRACTOR as ::core::ffi::c_int as usize]);
    init_house(shapes[SHAPE_HOUSE as ::core::ffi::c_int as usize]);
    init_sun(shapes[SHAPE_SUN as ::core::ffi::c_int as usize]);
    init_cloud(shapes[SHAPE_CLOUD as ::core::ffi::c_int as usize]);
    init_flower(shapes[SHAPE_FLOWER as ::core::ffi::c_int as usize]);
    init_car(shapes[SHAPE_CAR as ::core::ffi::c_int as usize]);
    init_star(shapes[SHAPE_STAR as ::core::ffi::c_int as usize]);
    init_heart(shapes[SHAPE_HEART as ::core::ffi::c_int as usize]);
    init_rainbow(shapes[SHAPE_RAINBOW as ::core::ffi::c_int as usize]);
}
#[no_mangle]
pub unsafe extern "C" fn shape_manager_cleanup() {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < SHAPE_COUNT as ::core::ffi::c_int {
        free(shapes[i as usize] as *mut ::core::ffi::c_void);
        shapes[i as usize] = ::core::ptr::null_mut::<shape_t>();
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn shape_get(mut type_0: shape_type_t) -> *mut shape_t {
    if (type_0 as ::core::ffi::c_uint) < 0 as ::core::ffi::c_uint
        || type_0 as ::core::ffi::c_uint >= SHAPE_COUNT as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return ::core::ptr::null_mut::<shape_t>();
    }
    return shapes[type_0 as usize];
}
#[no_mangle]
pub unsafe extern "C" fn shape_print(mut shape: *const shape_t) {
    if shape.is_null() {
        printf(b"(null shape)\n\0" as *const u8 as *const ::core::ffi::c_char);
        return;
    }
    printf(
        b"%s:\n\0" as *const u8 as *const ::core::ffi::c_char,
        &raw const (*shape).name as *const ::core::ffi::c_char,
    );
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*shape).height {
        printf(
            b"%s\n\0" as *const u8 as *const ::core::ffi::c_char,
            &raw const *(&raw const (*shape).art as *const [::core::ffi::c_char; 80])
                .offset(i as isize) as *const ::core::ffi::c_char,
        );
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn shape_equals(
    mut s1: *const shape_t,
    mut s2: *const shape_t,
) -> ::core::ffi::c_int {
    return if s1 == s2 {
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    };
}
#[no_mangle]
pub unsafe extern "C" fn shape_type_name(mut type_0: shape_type_t) -> *const ::core::ffi::c_char {
    match type_0 as ::core::ffi::c_uint {
        0 => return b"Tree\0" as *const u8 as *const ::core::ffi::c_char,
        1 => return b"Tractor\0" as *const u8 as *const ::core::ffi::c_char,
        2 => return b"House\0" as *const u8 as *const ::core::ffi::c_char,
        3 => return b"Sun\0" as *const u8 as *const ::core::ffi::c_char,
        4 => return b"Cloud\0" as *const u8 as *const ::core::ffi::c_char,
        5 => return b"Flower\0" as *const u8 as *const ::core::ffi::c_char,
        6 => return b"Car\0" as *const u8 as *const ::core::ffi::c_char,
        7 => return b"Star\0" as *const u8 as *const ::core::ffi::c_char,
        8 => return b"Heart\0" as *const u8 as *const ::core::ffi::c_char,
        9 => return b"Rainbow\0" as *const u8 as *const ::core::ffi::c_char,
        _ => return b"Unknown\0" as *const u8 as *const ::core::ffi::c_char,
    };
}
