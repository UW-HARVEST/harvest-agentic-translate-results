use std::ffi::c_void;
use crate::types::*;

extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
    pub fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
}

pub(crate) static mut GLOBAL_HOOKS: InternalHooks = InternalHooks {
    allocate: malloc,
    deallocate: free,
    reallocate: Some(realloc),
};

pub(crate) static mut GLOBAL_ERROR: ErrorInfo = ErrorInfo {
    json: std::ptr::null(),
    position: 0,
};
