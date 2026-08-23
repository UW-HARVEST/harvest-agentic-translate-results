//! Declarations of the C shim's variadic functions (defined in shim.c) plus
//! their Rust-side `rs_*` implementations. The shim formats varargs then calls
//! these. The public linker symbols (js_error, js_typeerror, ...) are exported
//! by the C shim, matching the original MuJS names.
#![allow(non_snake_case)]

use crate::types::*;
use std::os::raw::{c_char, c_int, c_void};

extern "C-unwind" {
    // Public variadic error functions (exported by shim.c with correct names).
    pub fn js_error(J: *mut js_State, fmt: *const c_char, ...);
    pub fn js_evalerror(J: *mut js_State, fmt: *const c_char, ...);
    pub fn js_rangeerror(J: *mut js_State, fmt: *const c_char, ...);
    pub fn js_referenceerror(J: *mut js_State, fmt: *const c_char, ...);
    pub fn js_syntaxerror(J: *mut js_State, fmt: *const c_char, ...);
    pub fn js_typeerror(J: *mut js_State, fmt: *const c_char, ...);
    pub fn js_urierror(J: *mut js_State, fmt: *const c_char, ...);

    // Internal variadic helpers (shim symbols with _shim suffix).
    pub fn jsC_error(J: *mut js_State, node: *mut c_void, fmt: *const c_char, ...);
    pub fn jsP_error_shim(J: *mut js_State, fmt: *const c_char, ...);
    pub fn jsP_warning_shim(J: *mut js_State, fmt: *const c_char, ...);
    pub fn jsY_error_shim(J: *mut js_State, fmt: *const c_char, ...);
}
