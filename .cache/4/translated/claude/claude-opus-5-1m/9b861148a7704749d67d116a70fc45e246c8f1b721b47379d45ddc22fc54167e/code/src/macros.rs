//! Helper macros used to transliterate the C sources.
#![allow(unused_macros)]

/// `snprintf(buf, sizeof buf, fmt, ...)` on a fixed size `[c_char; N]` array.
#[macro_export]
macro_rules! sfmt {
    ($buf:expr, $fmt:expr $(, $a:expr)*) => {
        $crate::cstd::snprintf($buf.as_mut_ptr(), $buf.len() as $crate::cstd::size_t, $fmt $(, $a)*)
    };
}

/// Build a 256 byte message with printf formatting, throw error of given class.
#[macro_export]
macro_rules! jsthrow_fmt {
    ($J:expr, $proto:ident, $fmt:expr $(, $a:expr)*) => {{
        let mut __buf = [0 as $crate::cstd::c_char; 256];
        $crate::cstd::snprintf(__buf.as_mut_ptr(), 256, $fmt $(, $a)*);
        $crate::jserror::js_newerrorx($J, __buf.as_ptr(), (*$J).$proto);
        $crate::jsrun::js_throw($J)
    }};
}

#[macro_export]
macro_rules! js_error {
    ($J:expr, $fmt:expr $(, $a:expr)*) => { $crate::jsthrow_fmt!($J, Error_prototype, $fmt $(, $a)*) };
}
#[macro_export]
macro_rules! js_evalerror {
    ($J:expr, $fmt:expr $(, $a:expr)*) => { $crate::jsthrow_fmt!($J, EvalError_prototype, $fmt $(, $a)*) };
}
#[macro_export]
macro_rules! js_rangeerror {
    ($J:expr, $fmt:expr $(, $a:expr)*) => { $crate::jsthrow_fmt!($J, RangeError_prototype, $fmt $(, $a)*) };
}
#[macro_export]
macro_rules! js_referenceerror {
    ($J:expr, $fmt:expr $(, $a:expr)*) => { $crate::jsthrow_fmt!($J, ReferenceError_prototype, $fmt $(, $a)*) };
}
#[macro_export]
macro_rules! js_syntaxerror {
    ($J:expr, $fmt:expr $(, $a:expr)*) => { $crate::jsthrow_fmt!($J, SyntaxError_prototype, $fmt $(, $a)*) };
}
#[macro_export]
macro_rules! js_typeerror {
    ($J:expr, $fmt:expr $(, $a:expr)*) => { $crate::jsthrow_fmt!($J, TypeError_prototype, $fmt $(, $a)*) };
}
#[macro_export]
macro_rules! js_urierror {
    ($J:expr, $fmt:expr $(, $a:expr)*) => { $crate::jsthrow_fmt!($J, URIError_prototype, $fmt $(, $a)*) };
}
