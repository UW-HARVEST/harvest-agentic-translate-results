//! The public error raising functions of mujs are C variadic functions.
//!
//! Stable Rust cannot *define* variadic functions, so we reconstruct the
//! SysV x86-64 `va_list` by hand: the first six integer class arguments arrive
//! in rdi/rsi/rdx/rcx/r8/r9, further integer class arguments on the stack and
//! the first eight floating point arguments in xmm0..xmm7.  By declaring a
//! non-variadic function with 14 integer parameters and 8 double parameters we
//! receive exactly those registers/stack slots and can then materialise a
//! `va_list` for `vsnprintf`, giving behaviour identical to the C original.

use crate::jserror::js_newerrorx;
use crate::jsi::*;
use crate::jsrun::js_throw;

#[repr(C)]
struct VaListTag {
    gp_offset: c_uint,
    fp_offset: c_uint,
    overflow_arg_area: *mut c_void,
    reg_save_area: *mut c_void,
}

#[repr(C, align(16))]
struct RegSave {
    gp: [u64; 6],
    fp: [[u64; 2]; 8],
}

macro_rules! define_error_fn {
    ($name:ident, $proto:ident) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            J: *mut js_State,
            fmt: *const c_char,
            a2: u64,
            a3: u64,
            a4: u64,
            a5: u64,
            s0: u64,
            s1: u64,
            s2: u64,
            s3: u64,
            s4: u64,
            s5: u64,
            s6: u64,
            s7: u64,
            f0: f64,
            f1: f64,
            f2: f64,
            f3: f64,
            f4: f64,
            f5: f64,
            f6: f64,
            f7: f64,
        ) -> ! {
            let mut regsave = RegSave {
                gp: [J as u64, fmt as u64, a2, a3, a4, a5],
                fp: [
                    [f0.to_bits(), 0],
                    [f1.to_bits(), 0],
                    [f2.to_bits(), 0],
                    [f3.to_bits(), 0],
                    [f4.to_bits(), 0],
                    [f5.to_bits(), 0],
                    [f6.to_bits(), 0],
                    [f7.to_bits(), 0],
                ],
            };
            let mut overflow: [u64; 8] = [s0, s1, s2, s3, s4, s5, s6, s7];
            let mut ap = VaListTag {
                gp_offset: 16, /* J and fmt are named parameters */
                fp_offset: 48,
                overflow_arg_area: overflow.as_mut_ptr() as *mut c_void,
                reg_save_area: &mut regsave as *mut RegSave as *mut c_void,
            };
            let mut buf: [c_char; 256] = [0; 256];
            vsnprintf(
                buf.as_mut_ptr(),
                256,
                fmt,
                &mut ap as *mut VaListTag as *mut c_void,
            );
            js_newerrorx(J, buf.as_ptr(), (*J).$proto);
            js_throw(J)
        }
    };
}

/// `jsC_error(js_State *J, js_Ast *node, const char *fmt, ...)` from jscompile.c
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsC_error(
    J: *mut js_State,
    node: *mut js_Ast,
    fmt: *const c_char,
    a3: u64,
    a4: u64,
    a5: u64,
    s0: u64,
    s1: u64,
    s2: u64,
    s3: u64,
    s4: u64,
    s5: u64,
    s6: u64,
    s7: u64,
    f0: f64,
    f1: f64,
    f2: f64,
    f3: f64,
    f4: f64,
    f5: f64,
    f6: f64,
    f7: f64,
) -> ! {
    let mut regsave = RegSave {
        gp: [J as u64, node as u64, fmt as u64, a3, a4, a5],
        fp: [
            [f0.to_bits(), 0],
            [f1.to_bits(), 0],
            [f2.to_bits(), 0],
            [f3.to_bits(), 0],
            [f4.to_bits(), 0],
            [f5.to_bits(), 0],
            [f6.to_bits(), 0],
            [f7.to_bits(), 0],
        ],
    };
    let mut overflow: [u64; 8] = [s0, s1, s2, s3, s4, s5, s6, s7];
    let mut ap = VaListTag {
        gp_offset: 24, /* J, node and fmt are named parameters */
        fp_offset: 48,
        overflow_arg_area: overflow.as_mut_ptr() as *mut c_void,
        reg_save_area: &mut regsave as *mut RegSave as *mut c_void,
    };
    let mut msgbuf: [c_char; 256] = [0; 256];
    vsnprintf(
        msgbuf.as_mut_ptr(),
        256,
        fmt,
        &mut ap as *mut VaListTag as *mut c_void,
    );
    jsC_error_str(J, node, msgbuf.as_ptr())
}

/// Non-variadic core of `jsC_error`, taking the already formatted message.
pub unsafe fn jsC_error_str(J: *mut js_State, node: *mut js_Ast, msgbuf: *const c_char) -> ! {
    let mut buf: [c_char; 512] = [0; 512];
    snprintf(
        buf.as_mut_ptr(),
        256,
        cs!("%s:%d: "),
        (*J).filename,
        (*node).line,
    );
    strcat(buf.as_mut_ptr(), msgbuf);
    crate::jserror::js_newsyntaxerror(J, buf.as_ptr());
    js_throw(J)
}

define_error_fn!(js_error, Error_prototype);
define_error_fn!(js_evalerror, EvalError_prototype);
define_error_fn!(js_rangeerror, RangeError_prototype);
define_error_fn!(js_referenceerror, ReferenceError_prototype);
define_error_fn!(js_syntaxerror, SyntaxError_prototype);
define_error_fn!(js_typeerror, TypeError_prototype);
define_error_fn!(js_urierror, URIError_prototype);
