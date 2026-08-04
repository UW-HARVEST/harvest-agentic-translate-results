static mut INNER: i32 = 1;

/// # Safety
/// Caller must pass a valid pointer to a mutable i32.
#[no_mangle]
pub unsafe extern "C" fn static_alias(outer: *mut i32) -> *mut i32 {
    let outer_val = *outer;
    if outer_val >= INNER {
        INNER += outer_val;
        &raw mut INNER
    } else {
        *outer += INNER;
        outer
    }
}
