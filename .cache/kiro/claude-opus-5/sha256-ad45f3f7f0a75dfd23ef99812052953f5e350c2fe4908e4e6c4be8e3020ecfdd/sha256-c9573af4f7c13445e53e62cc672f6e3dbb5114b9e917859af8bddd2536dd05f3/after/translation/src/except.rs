//! Exception plumbing.
//!
//! MuJS implements JS exceptions with `setjmp`/`longjmp`.  Calling `setjmp`
//! from Rust is not sound, so internally we use unwinding instead:
//!
//!  * `js_savetry()` / `js_savetrypc()` (called from *C*) push a try frame
//!    marked `TRY_EXTERNAL`; `js_throw()` reaches those with a real `longjmp`,
//!    exactly like the C original.
//!  * Try frames pushed by the Rust translation are marked `TRY_RUST` and are
//!    reached by unwinding a `JsThrow` panic payload.
//!
//! `js_throw()` always targets the *innermost* try frame (it pops exactly one),
//! so a catch site only has to check `J->throwtarget` against the index it
//! pushed.
#![allow(non_snake_case)]

use crate::jsi::*;
use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

pub struct JsThrow;

static HOOK_ONCE: std::sync::Once = std::sync::Once::new();

/// Silence Rust panic reporting.
///
/// The unwinds this crate performs are ordinary control flow (a thrown JS
/// exception, or `regexp.c`'s `die()` longjmp), not failures, and the C library
/// prints nothing for them.  Set `MUJS_RUST_PANIC` in the environment to get the
/// default panic output back while debugging.
pub fn install_panic_hook() {
    HOOK_ONCE.call_once(|| {
        if std::env::var_os("MUJS_RUST_PANIC").is_some() {
            return;
        }
        std::panic::set_hook(Box::new(|_info| {}));
    });
}

#[inline]
pub fn is_js_throw(p: &Box<dyn Any + Send>) -> bool {
    p.downcast_ref::<JsThrow>().is_some()
}

/// Raise a `JsThrow`; unwinds to the nearest matching catch site.
#[inline]
pub fn raise() -> ! {
    std::panic::panic_any(JsThrow)
}

/// Equivalent of the C idiom
/// ```c
/// if (js_try(J)) { ...handler... }
/// ...body...
/// js_endtry(J);
/// ```
/// The `body` closure *must* end with `js_endtry(J)` just like the C code.
/// Returns `true` when an exception was caught (i.e. `js_try` returned nonzero).
pub unsafe fn js_try_run<F: FnOnce()>(J: *mut js_State, body: F) -> bool {
    unsafe {
        let idx = crate::jsrun::js_pushtry(J, core::ptr::null_mut(), TRY_RUST);
        match catch_unwind(AssertUnwindSafe(body)) {
            Ok(()) => false,
            Err(p) => {
                if is_js_throw(&p) && (*J).throwtarget == idx {
                    true
                } else {
                    resume_unwind(p)
                }
            }
        }
    }
}
