extern "C" {
    static mut stderr: *mut _IO_FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn malloc(__size: size_t) -> *mut libc::c_void;
    fn free(__ptr: *mut libc::c_void);
    
    
    
    
    
    
    fn strncpy(
        __dest: *mut libc::c_char,
        __src: *const libc::c_char,
        __n: size_t,
    ) -> *mut libc::c_char;
    fn strchr(__s: *const libc::c_char, __c: libc::c_int)
        -> *mut libc::c_char;
    fn strlen(__s: *const libc::c_char) -> size_t;
}
pub use crate::src::logger::finalize_logger;
pub use crate::src::logger::initialize_logger;
pub use crate::src::task_manager::add_task;
pub use crate::src::task_manager::create_task_manager;
pub use crate::src::task_manager::destroy_task_manager;
pub use crate::src::task_manager::print_tasks;
pub type size_t = usize;
pub type __off_t = libc::c_long;
pub type __off64_t = libc::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: libc::c_int,
    pub _IO_read_ptr: *mut libc::c_char,
    pub _IO_read_end: *mut libc::c_char,
    pub _IO_read_base: *mut libc::c_char,
    pub _IO_write_base: *mut libc::c_char,
    pub _IO_write_ptr: *mut libc::c_char,
    pub _IO_write_end: *mut libc::c_char,
    pub _IO_buf_base: *mut libc::c_char,
    pub _IO_buf_end: *mut libc::c_char,
    pub _IO_save_base: *mut libc::c_char,
    pub _IO_backup_base: *mut libc::c_char,
    pub _IO_save_end: *mut libc::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: libc::c_int,
    pub _flags2: libc::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: libc::c_ushort,
    pub _vtable_offset: libc::c_schar,
    pub _shortbuf: [libc::c_char; 1],
    pub _lock: *mut libc::c_void,
    pub _offset: __off64_t,
    pub __pad1: *mut libc::c_void,
    pub __pad2: *mut libc::c_void,
    pub __pad3: *mut libc::c_void,
    pub __pad4: *mut libc::c_void,
    pub __pad5: size_t,
    pub _mode: libc::c_int,
    pub _unused2: [libc::c_char; 20],
}
pub type _IO_lock_t = ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_marker {
    pub _next: *mut _IO_marker,
    pub _sbuf: *mut _IO_FILE,
    pub _pos: libc::c_int,
}
pub type FILE = _IO_FILE;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Task {
    pub description: [libc::c_char; 256],
    pub priority: libc::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct TaskManager {
    pub tasks: *mut Task,
    pub max_tasks: libc::c_int,
    pub task_count: libc::c_int,
}
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
pub const EXIT_FAILURE: libc::c_int = 1 as libc::c_int;
#[no_mangle]
pub unsafe extern "C" fn driver(mut tasks: *const libc::c_char) -> libc::c_int {
    let mut res: libc::c_int = initialize_logger();
    if res != 0 as libc::c_int {
        return EXIT_FAILURE;
    }
    let mut manager: *mut TaskManager = create_task_manager();
    if manager.is_null() {
        return EXIT_FAILURE;
    }
    let mut start: *const libc::c_char = tasks;
    let mut priority: libc::c_int = 1 as libc::c_int;
    while *start as libc::c_int != '\0' as i32 {
        let mut end: *const libc::c_char = strchr(start, '\n' as i32);
        if end.is_null() {
            end = start.offset(strlen(start) as isize);
        }
        let mut length: size_t = end.offset_from(start) as libc::c_long as size_t;
        let mut task: *mut libc::c_char =
            malloc(length.wrapping_add(1 as size_t)) as *mut libc::c_char;
        if task.is_null() {
            fprintf(
                stderr as *mut FILE,
                b"Error: Failed to allocate memory for task.\n\0" as *const u8
                    as *const libc::c_char,
            );
            destroy_task_manager(manager);
            finalize_logger();
            return EXIT_FAILURE;
        }
        strncpy(task, start, length);
        *task.offset(length as isize) = '\0' as i32 as libc::c_char;
        let fresh0 = priority;
        priority = priority + 1;
        add_task(manager, task, fresh0);
        free(task as *mut libc::c_void);
        start = if *end as libc::c_int == '\n' as i32 {
            end.offset(1 as libc::c_int as isize)
        } else {
            end
        };
    }
    print_tasks(manager);
    destroy_task_manager(manager);
    finalize_logger();
    return 0 as libc::c_int;
}
