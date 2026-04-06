extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn atoi(__nptr: *const ::core::ffi::c_char) -> ::core::ffi::c_int;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn getenv(__name: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    fn strncpy(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> *mut ::core::ffi::c_char;
    fn log_info(message: *const ::core::ffi::c_char);
    fn log_warning(message: *const ::core::ffi::c_char);
    fn log_error(message: *const ::core::ffi::c_char);
}
pub type size_t = usize;
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
#[no_mangle]
pub unsafe extern "C" fn create_task_manager() -> *mut TaskManager {
    let mut manager: *mut TaskManager =
        malloc(::core::mem::size_of::<TaskManager>() as size_t) as *mut TaskManager;
    if manager.is_null() {
        log_error(
            b"Failed to allocate memory for TaskManager.\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        return ::core::ptr::null_mut::<TaskManager>();
    }
    let mut max_tasks_env: *const ::core::ffi::c_char =
        getenv(b"MAX_TASKS\0" as *const u8 as *const ::core::ffi::c_char);
    (*manager).max_tasks = if !max_tasks_env.is_null() {
        atoi(max_tasks_env)
    } else {
        10 as ::core::ffi::c_int
    };
    (*manager).task_count = 0 as ::core::ffi::c_int;
    (*manager).tasks = malloc(
        ((*manager).max_tasks as size_t).wrapping_mul(::core::mem::size_of::<Task>() as size_t),
    ) as *mut Task;
    if (*manager).tasks.is_null() {
        log_error(
            b"Failed to allocate memory for tasks.\0" as *const u8 as *const ::core::ffi::c_char,
        );
        free(manager as *mut ::core::ffi::c_void);
        return ::core::ptr::null_mut::<TaskManager>();
    }
    log_info(b"TaskManager created successfully.\0" as *const u8 as *const ::core::ffi::c_char);
    return manager;
}
#[no_mangle]
pub unsafe extern "C" fn add_task(
    mut manager: *mut TaskManager,
    mut description: *const ::core::ffi::c_char,
    mut priority: ::core::ffi::c_int,
) {
    if (*manager).task_count >= (*manager).max_tasks {
        log_warning(
            b"Cannot add task: Maximum task limit reached.\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        return;
    }
    let fresh0 = (*manager).task_count;
    (*manager).task_count = (*manager).task_count + 1;
    let mut task: *mut Task = (*manager).tasks.offset(fresh0 as isize) as *mut Task;
    strncpy(
        &raw mut (*task).description as *mut ::core::ffi::c_char,
        description,
        (::core::mem::size_of::<[::core::ffi::c_char; 256]>() as size_t).wrapping_sub(1 as size_t),
    );
    (*task).description[(::core::mem::size_of::<[::core::ffi::c_char; 256]>() as usize)
        .wrapping_sub(1 as usize) as usize] = '\0' as i32 as ::core::ffi::c_char;
    (*task).priority = priority;
    log_info(b"Task added successfully.\0" as *const u8 as *const ::core::ffi::c_char);
}
#[no_mangle]
pub unsafe extern "C" fn print_tasks(mut manager: *const TaskManager) {
    printf(b"Tasks:\n\0" as *const u8 as *const ::core::ffi::c_char);
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*manager).task_count {
        printf(
            b"  [%d] %s (Priority: %d)\n\0" as *const u8 as *const ::core::ffi::c_char,
            i + 1 as ::core::ffi::c_int,
            &raw mut (*(*manager).tasks.offset(i as isize)).description as *mut ::core::ffi::c_char,
            (*(*manager).tasks.offset(i as isize)).priority,
        );
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn destroy_task_manager(mut manager: *mut TaskManager) {
    free((*manager).tasks as *mut ::core::ffi::c_void);
    free(manager as *mut ::core::ffi::c_void);
    log_info(b"TaskManager destroyed successfully.\0" as *const u8 as *const ::core::ffi::c_char);
}
