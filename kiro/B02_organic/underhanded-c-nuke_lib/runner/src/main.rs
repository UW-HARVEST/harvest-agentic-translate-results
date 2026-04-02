#![cfg_attr(fuzzing, no_main)]

use cando2::*;

harness! {
    state: {
        test: Vec<c_double>,
        reference: Vec<c_double>,
        bins: c_int,
        threshold: c_double,
        returns: c_int
    },

    library: "underhanded-c-nuke_lib",
    symbol: "match",
    signature: unsafe extern "C" fn(*const c_double, *const c_double, c_int, c_double) -> c_int,

    fn run(&mut self) {
        self.returns = unsafe {
            (*SYMBOL)(
                self.test.as_ptr(),
                self.reference.as_ptr(),
                self.bins,
                self.threshold,
            )
        };
    }
}
