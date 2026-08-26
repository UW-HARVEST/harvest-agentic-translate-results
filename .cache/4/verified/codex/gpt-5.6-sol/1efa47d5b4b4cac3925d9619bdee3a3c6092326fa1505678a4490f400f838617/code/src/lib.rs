#[cfg(not(test))]
mod logic;

#[cfg(not(test))]
use std::ffi::c_int;

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> c_int {
    logic::run();
    0
}
