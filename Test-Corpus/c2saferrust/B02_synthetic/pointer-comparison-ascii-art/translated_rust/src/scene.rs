extern "C" {
    fn shape_get(type_0: shape_type_t) -> *mut shape_t;
    fn shape_print(shape: *const shape_t);
    fn shape_equals(s1: *const shape_t, s2: *const shape_t) -> ::core::ffi::c_int;
    static mut stderr: *mut _IO_FILE;
    fn fclose(__stream: *mut FILE) -> ::core::ffi::c_int;
    fn fopen(
        __filename: *const ::core::ffi::c_char,
        __modes: *const ::core::ffi::c_char,
    ) -> *mut FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn fscanf(__stream: *mut FILE, __format: *const ::core::ffi::c_char, ...)
        -> ::core::ffi::c_int;
    fn fgets(
        __s: *mut ::core::ffi::c_char,
        __n: ::core::ffi::c_int,
        __stream: *mut FILE,
    ) -> *mut ::core::ffi::c_char;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn strcpy(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strncpy(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> *mut ::core::ffi::c_char;
    fn strcspn(
        __s: *const ::core::ffi::c_char,
        __reject: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_ulong;
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
pub struct scene_t {
    pub name: [::core::ffi::c_char; 64],
    pub shapes: [*mut shape_t; 50],
    pub shape_count: ::core::ffi::c_int,
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
pub const MAX_SHAPES_IN_SCENE: ::core::ffi::c_int = 50 as ::core::ffi::c_int;
pub const MAX_SCENE_NAME: ::core::ffi::c_int = 64 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn scene_create(mut name: *const ::core::ffi::c_char) -> *mut scene_t {
    let mut scene: *mut scene_t =
        malloc(::core::mem::size_of::<scene_t>() as size_t) as *mut scene_t;
    if scene.is_null() {
        return ::core::ptr::null_mut::<scene_t>();
    }
    if !name.is_null() {
        strncpy(
            &raw mut (*scene).name as *mut ::core::ffi::c_char,
            name,
            (MAX_SCENE_NAME - 1 as ::core::ffi::c_int) as size_t,
        );
        (*scene).name[(MAX_SCENE_NAME - 1 as ::core::ffi::c_int) as usize] =
            '\0' as i32 as ::core::ffi::c_char;
    } else {
        strcpy(
            &raw mut (*scene).name as *mut ::core::ffi::c_char,
            b"Untitled Scene\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    (*scene).shape_count = 0 as ::core::ffi::c_int;
    return scene;
}
#[no_mangle]
pub unsafe extern "C" fn scene_destroy(mut scene: *mut scene_t) {
    if !scene.is_null() {
        free(scene as *mut ::core::ffi::c_void);
    }
}
#[no_mangle]
pub unsafe extern "C" fn scene_add_shape(
    mut scene: *mut scene_t,
    mut shape: *mut shape_t,
) -> ::core::ffi::c_int {
    if scene.is_null() || shape.is_null() {
        return -(1 as ::core::ffi::c_int);
    }
    if (*scene).shape_count >= MAX_SHAPES_IN_SCENE {
        fprintf(
            stderr as *mut FILE,
            b"Error: Scene is full\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        return -(1 as ::core::ffi::c_int);
    }
    let fresh0 = (*scene).shape_count;
    (*scene).shape_count = (*scene).shape_count + 1;
    (*scene).shapes[fresh0 as usize] = shape;
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn scene_remove_shape(
    mut scene: *mut scene_t,
    mut index: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if scene.is_null() || index < 0 as ::core::ffi::c_int || index >= (*scene).shape_count {
        return -(1 as ::core::ffi::c_int);
    }
    let mut i: ::core::ffi::c_int = index;
    while i < (*scene).shape_count - 1 as ::core::ffi::c_int {
        (*scene).shapes[i as usize] = (*scene).shapes[(i + 1 as ::core::ffi::c_int) as usize];
        i += 1;
    }
    (*scene).shape_count -= 1;
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn scene_print(mut scene: *const scene_t) {
    if scene.is_null() {
        printf(b"(null scene)\n\0" as *const u8 as *const ::core::ffi::c_char);
        return;
    }
    printf(
        b"\n=== Scene: %s ===\n\0" as *const u8 as *const ::core::ffi::c_char,
        &raw const (*scene).name as *const ::core::ffi::c_char,
    );
    printf(
        b"Contains %d shape(s)\n\n\0" as *const u8 as *const ::core::ffi::c_char,
        (*scene).shape_count,
    );
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*scene).shape_count {
        printf(
            b"Shape #%d:\n\0" as *const u8 as *const ::core::ffi::c_char,
            i + 1 as ::core::ffi::c_int,
        );
        shape_print((*scene).shapes[i as usize]);
        printf(b"\n\0" as *const u8 as *const ::core::ffi::c_char);
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn scene_equals(
    mut s1: *const scene_t,
    mut s2: *const scene_t,
) -> ::core::ffi::c_int {
    if s1.is_null() || s2.is_null() {
        return 0 as ::core::ffi::c_int;
    }
    if (*s1).shape_count != (*s2).shape_count {
        return 0 as ::core::ffi::c_int;
    }
    let mut matched: [::core::ffi::c_int; 50] = [0 as ::core::ffi::c_int; 50];
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*s1).shape_count {
        let mut found: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        let mut j: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while j < (*s2).shape_count {
            if matched[j as usize] == 0
                && shape_equals((*s1).shapes[i as usize], (*s2).shapes[j as usize]) != 0
            {
                matched[j as usize] = 1 as ::core::ffi::c_int;
                found = 1 as ::core::ffi::c_int;
                break;
            } else {
                j += 1;
            }
        }
        if found == 0 {
            return 0 as ::core::ffi::c_int;
        }
        i += 1;
    }
    return 1 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn scene_save(
    mut scene: *const scene_t,
    mut filename: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if scene.is_null() || filename.is_null() {
        return -(1 as ::core::ffi::c_int);
    }
    let mut file: *mut FILE = fopen(filename, b"w\0" as *const u8 as *const ::core::ffi::c_char);
    if file.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: Could not open file '%s' for writing\n\0" as *const u8
                as *const ::core::ffi::c_char,
            filename,
        );
        return -(1 as ::core::ffi::c_int);
    }
    fprintf(
        file,
        b"%s\n\0" as *const u8 as *const ::core::ffi::c_char,
        &raw const (*scene).name as *const ::core::ffi::c_char,
    );
    fprintf(
        file,
        b"%d\n\0" as *const u8 as *const ::core::ffi::c_char,
        (*scene).shape_count,
    );
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*scene).shape_count {
        fprintf(
            file,
            b"%d\n\0" as *const u8 as *const ::core::ffi::c_char,
            (*(*scene).shapes[i as usize]).type_0 as ::core::ffi::c_uint,
        );
        i += 1;
    }
    fclose(file);
    printf(
        b"Scene saved to '%s'\n\0" as *const u8 as *const ::core::ffi::c_char,
        filename,
    );
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn scene_load(mut filename: *const ::core::ffi::c_char) -> *mut scene_t {
    if filename.is_null() {
        return ::core::ptr::null_mut::<scene_t>();
    }
    let mut file: *mut FILE = fopen(filename, b"r\0" as *const u8 as *const ::core::ffi::c_char);
    if file.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: Could not open file '%s' for reading\n\0" as *const u8
                as *const ::core::ffi::c_char,
            filename,
        );
        return ::core::ptr::null_mut::<scene_t>();
    }
    let mut name: [::core::ffi::c_char; 64] = [0; 64];
    if fgets(
        &raw mut name as *mut ::core::ffi::c_char,
        MAX_SCENE_NAME,
        file,
    )
    .is_null()
    {
        fclose(file);
        return ::core::ptr::null_mut::<scene_t>();
    }
    name[strcspn(
        &raw mut name as *mut ::core::ffi::c_char,
        b"\n\0" as *const u8 as *const ::core::ffi::c_char,
    ) as usize] = 0 as ::core::ffi::c_char;
    let mut scene: *mut scene_t = scene_create(&raw mut name as *mut ::core::ffi::c_char);
    if scene.is_null() {
        fclose(file);
        return ::core::ptr::null_mut::<scene_t>();
    }
    let mut shape_count: ::core::ffi::c_int = 0;
    if fscanf(
        file,
        b"%d\n\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut shape_count,
    ) != 1 as ::core::ffi::c_int
    {
        scene_destroy(scene);
        fclose(file);
        return ::core::ptr::null_mut::<scene_t>();
    }
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < shape_count {
        let mut type_0: ::core::ffi::c_int = 0;
        if fscanf(
            file,
            b"%d\n\0" as *const u8 as *const ::core::ffi::c_char,
            &raw mut type_0,
        ) != 1 as ::core::ffi::c_int
        {
            scene_destroy(scene);
            fclose(file);
            return ::core::ptr::null_mut::<scene_t>();
        }
        let mut shape: *mut shape_t = shape_get(type_0 as shape_type_t);
        if !shape.is_null() {
            scene_add_shape(scene, shape);
        }
        i += 1;
    }
    fclose(file);
    printf(
        b"Scene loaded from '%s'\n\0" as *const u8 as *const ::core::ffi::c_char,
        filename,
    );
    return scene;
}
#[no_mangle]
pub unsafe extern "C" fn scene_list_shapes(mut scene: *const scene_t) {
    if scene.is_null() {
        printf(b"(null scene)\n\0" as *const u8 as *const ::core::ffi::c_char);
        return;
    }
    printf(
        b"\nScene: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
        &raw const (*scene).name as *const ::core::ffi::c_char,
    );
    printf(
        b"Shapes (%d):\n\0" as *const u8 as *const ::core::ffi::c_char,
        (*scene).shape_count,
    );
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*scene).shape_count {
        printf(
            b"  %d. %s (ptr: %p)\n\0" as *const u8 as *const ::core::ffi::c_char,
            i + 1 as ::core::ffi::c_int,
            &raw mut (**(&raw const (*scene).shapes as *const *mut shape_t).offset(i as isize)).name
                as *mut ::core::ffi::c_char,
            (*scene).shapes[i as usize] as *mut ::core::ffi::c_void,
        );
        i += 1;
    }
}
