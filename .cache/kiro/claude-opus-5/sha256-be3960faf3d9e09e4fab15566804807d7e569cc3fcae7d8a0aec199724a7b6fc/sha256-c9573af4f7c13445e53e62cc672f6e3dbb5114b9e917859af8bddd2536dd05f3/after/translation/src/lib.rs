//! MuJS 1.3.8 — Rust translation of the C library in `c_src/`.
//!
//! The crate is a faithful, mechanical transliteration: raw pointers, the same
//! data structures, the same control flow and the same observable behaviour
//! (including the original bugs).
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_parens)]
#![allow(unused_assignments)]
#![allow(unused_mut)]
#![allow(unused_variables)]
#![allow(unused_labels)]
#![allow(dead_code)]

/* ------------------------------------------------------------------ */
/* Error throwing macros (mirror the DERROR() family in jserror.c)     */
/* ------------------------------------------------------------------ */

#[macro_export]
macro_rules! js_throw_errorx {
    ($J:expr, $proto:ident, $fmt:expr $(, $a:expr)*) => {{
        let mut __buf: [core::ffi::c_char; 256] = [0; 256];
        $crate::jsi::snprintf(__buf.as_mut_ptr(), 256, $fmt $(, $a)*);
        $crate::jserror::js_error_throw($J, __buf.as_ptr(), (*$J).$proto)
    }};
}

#[macro_export]
macro_rules! js_error {
    ($J:expr, $fmt:expr $(, $a:expr)*) => {
        $crate::js_throw_errorx!($J, Error_prototype, $fmt $(, $a)*)
    };
}
#[macro_export]
macro_rules! js_evalerror {
    ($J:expr, $fmt:expr $(, $a:expr)*) => {
        $crate::js_throw_errorx!($J, EvalError_prototype, $fmt $(, $a)*)
    };
}
#[macro_export]
macro_rules! js_rangeerror {
    ($J:expr, $fmt:expr $(, $a:expr)*) => {
        $crate::js_throw_errorx!($J, RangeError_prototype, $fmt $(, $a)*)
    };
}
#[macro_export]
macro_rules! js_referenceerror {
    ($J:expr, $fmt:expr $(, $a:expr)*) => {
        $crate::js_throw_errorx!($J, ReferenceError_prototype, $fmt $(, $a)*)
    };
}
#[macro_export]
macro_rules! js_syntaxerror {
    ($J:expr, $fmt:expr $(, $a:expr)*) => {
        $crate::js_throw_errorx!($J, SyntaxError_prototype, $fmt $(, $a)*)
    };
}
#[macro_export]
macro_rules! js_typeerror {
    ($J:expr, $fmt:expr $(, $a:expr)*) => {
        $crate::js_throw_errorx!($J, TypeError_prototype, $fmt $(, $a)*)
    };
}
#[macro_export]
macro_rules! js_urierror {
    ($J:expr, $fmt:expr $(, $a:expr)*) => {
        $crate::js_throw_errorx!($J, URIError_prototype, $fmt $(, $a)*)
    };
}

pub mod jsi;

pub mod except;
pub mod utf;
pub mod utfdata;

pub mod jsarray;
pub mod jsboolean;
pub mod jsbuiltin;
pub mod jscompile;
pub mod jsdate;
pub mod jsdtoa;
pub mod jserror;
pub mod jsfunction;
pub mod jsgc;
pub mod jsintern;
pub mod jslex;
pub mod jsmath;
pub mod jsnumber;
pub mod jsobject;
pub mod json;
pub mod jsparse;
pub mod jsproperty;
pub mod jsregexp;
pub mod jsrepr;
pub mod jsrun;
pub mod jsstate;
pub mod jsstring;
pub mod jsvalue;
pub mod regexp;
pub mod vararg;
