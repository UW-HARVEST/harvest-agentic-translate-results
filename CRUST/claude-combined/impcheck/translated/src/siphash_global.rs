// Global siphash state shared between top_check and lrat_check (and parser).
// In C these are file-scope statics in siphash.c.
use crate::siphash::SipHash;
use std::cell::RefCell;

thread_local! {
    static GLOBAL_SH: RefCell<Option<SipHash>> = RefCell::new(None);
}

pub fn init(key: &[u8]) {
    let sh = SipHash::siphash_init(key);
    GLOBAL_SH.with(|s| {
        *s.borrow_mut() = Some(sh);
    });
}

pub fn reset() {
    GLOBAL_SH.with(|s| {
        if let Some(sh) = s.borrow_mut().as_mut() {
            sh.siphash_reset();
        }
    });
}

pub fn with_global_siphash<R>(f: impl FnOnce(&mut SipHash) -> R) -> R {
    GLOBAL_SH.with(|s| {
        let mut sr = s.borrow_mut();
        f(sr.as_mut().expect("global siphash not initialized"))
    })
}
