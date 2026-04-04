extern "C" {
    static mut stderr: *mut _IO_FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn create_task_manager() -> *mut TaskManager;
    fn add_task(
        manager: *mut TaskManager,
        description: *const ::core::ffi::c_char,
        priority: ::core::ffi::c_int,
    );
    fn print_tasks(manager: *const TaskManager);
    fn destroy_task_manager(manager: *mut TaskManager);
    fn initialize_logger() -> ::core::ffi::c_int;
    fn finalize_logger();
    fn strncpy(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> *mut ::core::ffi::c_char;
    fn strchr(__s: *const ::core::ffi::c_char, __c: ::core::ffi::c_int)
        -> *mut ::core::ffi::c_char;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
}
pub type size_t = usize;
pub type __off_t = ::core::ffi::c_long;
pub type __off64_t = ::core::ffi::c_long;
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
pub type _IO_lock_t = ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_marker {
    pub _next: *mut _IO_marker,
    pub _sbuf: *mut _IO_FILE,
    pub _pos: ::core::ffi::c_int,
}
pub type FILE = _IO_FILE;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Task {
    pub description: [::core::ffi::c_char; 256],
    pub priority: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct TaskManager {
    pub tasks: *mut Task,
    pub max_tasks: ::core::ffi::c_int,
    pub task_count: ::core::ffi::c_int,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const EXIT_FAILURE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn driver(mut tasks: *const ::core::ffi::c_char) -> ::core::ffi::c_int {
    let mut res: ::core::ffi::c_int = initialize_logger();
    if res != 0 as ::core::ffi::c_int {
        return EXIT_FAILURE;
    }
    let mut manager: *mut TaskManager = create_task_manager();
    if manager.is_null() {
        return EXIT_FAILURE;
    }
    let mut start: *const ::core::ffi::c_char = tasks;
    let mut priority: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    while *start as ::core::ffi::c_int != '\0' as i32 {
        let mut end: *const ::core::ffi::c_char = strchr(start, '\n' as i32);
        if end.is_null() {
            end = start.offset(strlen(start) as isize);
        }
        let mut length: size_t = end.offset_from(start) as ::core::ffi::c_long as size_t;
        let mut task: *mut ::core::ffi::c_char =
            malloc(length.wrapping_add(1 as size_t)) as *mut ::core::ffi::c_char;
        if task.is_null() {
            fprintf(
                stderr as *mut FILE,
                b"Error: Failed to allocate memory for task.\n\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            destroy_task_manager(manager);
            finalize_logger();
            return EXIT_FAILURE;
        }
        strncpy(task, start, length);
        *task.offset(length as isize) = '\0' as i32 as ::core::ffi::c_char;
        let fresh0 = priority;
        priority = priority + 1;
        add_task(manager, task, fresh0);
        free(task as *mut ::core::ffi::c_void);
        start = if *end as ::core::ffi::c_int == '\n' as i32 {
            end.offset(1 as ::core::ffi::c_int as isize)
        } else {
            end
        };
    }
    print_tasks(manager);
    destroy_task_manager(manager);
    finalize_logger();
    return 0 as ::core::ffi::c_int;
}
