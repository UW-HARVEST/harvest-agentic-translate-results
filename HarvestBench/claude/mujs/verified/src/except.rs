//! Exception mechanism and low-level helpers modeling C setjmp/longjmp with
//! Rust panic/unwind. See design notes in types.rs.
#![allow(non_snake_case, dead_code)]

use crate::types::*;
use std::os::raw::c_int;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::sync::Once;

static HOOK: Once = Once::new();

/// Install a panic hook that suppresses the default message for panics used to
/// model C longjmp (JsThrow and the regexp compiler's Kaboom). Other panics
/// print normally. Idempotent.
pub fn install_panic_hook() {
    HOOK.call_once(|| {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let p = info.payload();
            if p.is::<JsThrow>() || p.is::<crate::regexp::Kaboom>() {
                return; // silent: this is a controlled longjmp-style unwind
            }
            prev(info);
        }));
    });
}

/// Push a try entry with pc = NULL. Returns the pushed index.
#[inline]
pub unsafe fn savetry(J: *mut js_State, pc: *mut js_Instruction) -> c_int {
    if (*J).trytop == JS_TRYLIMIT as c_int {
        crate::jsrun::js_trystackoverflow(J);
    }
    let i = (*J).trytop as usize;
    (*J).trybuf[i].E = (*J).E;
    (*J).trybuf[i].envtop = (*J).envtop;
    (*J).trybuf[i].tracetop = (*J).tracetop;
    (*J).trybuf[i].top = (*J).top;
    (*J).trybuf[i].bot = (*J).bot;
    (*J).trybuf[i].strict = (*J).strict;
    (*J).trybuf[i].pc = pc;
    (*J).trytop += 1;
    i as c_int
}

/// Run `body` under a protected try frame (models `if (js_try(J)) { handler }`
/// followed by body + js_endtry). Returns true if an exception targeting this
/// frame was caught (state already restored, exception value on stack); the
/// caller should then run its C handler. Returns false if body completed
/// normally (body is responsible for calling js_endtry).
pub unsafe fn protect<F: FnOnce()>(J: *mut js_State, body: F) -> bool {
    let idx = savetry(J, std::ptr::null_mut());
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(()) => false,
        Err(payload) => {
            if let Some(thr) = payload.downcast_ref::<JsThrow>() {
                if thr.target == idx {
                    return true;
                }
            }
            resume_unwind(payload);
        }
    }
}
